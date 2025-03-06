#!/usr/bin/env python3
"""
xs.py - Python interface for cross-stream

A Python implementation of the cross-stream interface, similar to xs.nu.
"""

import os
import json
import base64
import socket
import urllib.request
import urllib.parse
import urllib.error
import pathlib
import subprocess
import shutil
import glob
from typing import Dict, List, Optional, Union, Any, Callable
from dataclasses import dataclass
from http.client import HTTPResponse


XS_CONTEXT_SYSTEM = "0000000000000000000000000"


def xs_addr() -> str:
    """Get the address of the xs store from environment or default to ./store"""
    return os.environ.get("XS_ADDR", "./store")


def make_unix_socket_connection(sock_path: str):
    """Create a connection to a Unix socket"""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(sock_path)
    return sock


def request(method: str, path: str = "", data: bytes = None, headers: Dict = None) -> str:
    """Make a request to the xs store"""
    addr = xs_addr()

    if addr.startswith("./") or addr.startswith("/"):
        # Unix socket connection
        sock_path = f"{addr}/sock"
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            sock.connect(sock_path)

            # Construct HTTP request manually
            req_parts = [f"{method} /{path} HTTP/1.1", "Host: localhost", "Connection: close"]

            # Add headers
            if headers:
                for key, value in headers.items():
                    req_parts.append(f"{key}: {value}")

            # Add content length if we have data
            if data:
                req_parts.append(f"Content-Length: {len(data)}")

            # Finish headers
            req_parts.append("")
            req_parts.append("")

            # Create the request
            request_str = "\r\n".join(req_parts)

            # Send the request
            sock.sendall(request_str.encode("utf-8"))

            # Send data if we have it
            if data:
                sock.sendall(data)

            # Read the response
            response = b""
            while True:
                chunk = sock.recv(4096)
                if not chunk:
                    break
                response += chunk

            # Parse the HTTP response
            response_text = response.decode("utf-8")

            # Simple but effective way to extract the body
            if "\r\n\r\n" in response_text:
                body = response_text.split("\r\n\r\n", 1)[1]
                return body
            else:
                # Fall back to returning the whole response if headers separator not found
                # This shouldn't happen with a well-formed HTTP response
                return response_text

        except Exception as e:
            print(f"Error connecting to Unix socket: {e}")
            raise
        finally:
            sock.close()
    else:
        # HTTP connection
        req = urllib.request.Request(url=f"{addr}/{path}", method=method, data=data, headers=headers or {})
        with urllib.request.urlopen(req) as response:
            return response.read().decode("utf-8")


def xs_context_collect() -> List[Dict[str, str]]:
    """Collect all contexts"""
    contexts = {"0000000000000000000000000": "system"}

    for frame in _cat({"context": XS_CONTEXT_SYSTEM}):
        if frame.get("topic") == "xs.context":
            contexts[frame.get("id")] = frame.get("meta", {}).get("name")
        elif frame.get("topic") == "xs.annotate":
            if frame.get("meta", {}).get("updates") in contexts:
                contexts[frame.get("meta", {}).get("updates")] = frame.get("meta", {}).get("name")

    return [{"id": k, "name": v} for k, v in contexts.items()]


def xs_context(selected: Optional[str] = None) -> str:
    """Get or resolve a context ID"""
    if selected is None:
        return os.environ.get("XS_CONTEXT", XS_CONTEXT_SYSTEM)

    contexts = xs_context_collect()
    for context in contexts:
        if context["id"] == selected or context["name"] == selected:
            return context["id"]

    raise ValueError(f"Context not found: {selected}")


def _cat(options: Dict) -> List[Dict]:
    """Low-level function to read frames from the store"""
    params = []

    if options.get("follow", False):
        params.append("follow=true")
    if options.get("tail", False):
        params.append("tail=true")
    if options.get("all", False):
        params.append("all=true")
    if options.get("last_id"):
        params.append(f"last_id={options['last_id']}")
    if options.get("limit"):
        params.append(f"limit={options['limit']}")
    if options.get("pulse"):
        params.append(f"pulse={options['pulse']}")
    if options.get("context"):
        params.append(f"context={options['context']}")

    url_params = "&".join(params)
    path = f"?{url_params}" if url_params else ""

    headers = {}
    if options.get("follow", False):
        headers["Accept"] = "text/event-stream"

    try:
        content = request("GET", path, headers=headers)

        if options.get("follow", False):
            # Handle SSE stream
            # This would be implemented with a generator for SSE parsing
            # For simplicity, not implemented in this example
            return []
        else:
            result = []
            for line in content.splitlines():
                line = line.strip()
                if line:
                    # Skip lines that are just numbers (common in HTTP chunked encoding)
                    if line.isdigit():
                        continue
                    try:
                        result.append(json.loads(line))
                    except json.JSONDecodeError as e:
                        print(f"Warning: Could not parse JSON: {e}. Line: {line[:50]}...")
            return result
    except Exception as e:
        print(f"Error in _cat: {e}")
        return []


class XS:
    @staticmethod
    def cat(follow: bool = False, pulse: Optional[int] = None, tail: bool = False,
            last_id: Optional[str] = None, limit: Optional[int] = None,
            context: Optional[str] = None, all: bool = False) -> List[Dict]:
        """
        Cat the event stream

        Args:
            follow: long poll for new events
            pulse: specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
            tail: begin long after the end of the stream
            last_id: start reading from a specific frame ID
            limit: maximum number of frames to return
            context: the context to read from
            all: cat across all contexts

        Returns:
            List of frames
        """
        ctx = None
        if not all and context is not None:
            ctx = xs_context(context)

        return _cat({
            "follow": follow,
            "pulse": pulse,
            "tail": tail,
            "last_id": last_id,
            "limit": limit,
            "context": ctx,
            "all": all
        })

    @staticmethod
    def cas(hash_value: Optional[str] = None, input_data: Optional[str] = None) -> str:
        """
        Retrieve content from Content-Addressable Storage

        Args:
            hash_value: The hash to look up
            input_data: Alternative source for the hash

        Returns:
            Content associated with the hash
        """
        if hash_value is None:
            hash_value = input_data

        if hash_value is None:
            return None

        # If hash_value is a dict with a hash key, extract it
        if isinstance(hash_value, dict) and "hash" in hash_value:
            hash_value = hash_value["hash"]

        return request("GET", f"cas/{hash_value}")

    @staticmethod
    def get(id: str) -> Dict:
        """
        Get a frame by ID

        Args:
            id: Frame ID

        Returns:
            Frame data
        """
        resp = request("GET", id)
        return json.loads(resp)

    @staticmethod
    def head(topic: str, follow: bool = False, context: Optional[str] = None) -> Dict:
        """
        Get the head frame for a topic

        Args:
            topic: Topic name
            follow: Long poll for updates
            context: Context to use

        Returns:
            Head frame
        """
        params = []
        if context is not None:
            ctx = xs_context(context)
            params.append(f"context={ctx}")

        if follow:
            params.append("follow=true")

        url_params = "&".join(params)
        path_suffix = f"?{url_params}" if url_params else ""
        path = f"head/{topic}{path_suffix}"

        resp = request("GET", path)
        return json.loads(resp)

    @staticmethod
    def append(topic: str, content: str, meta: Optional[Dict] = None,
               context: Optional[str] = None, ttl: Optional[str] = None) -> Dict:
        """
        Append an event to the stream

        Args:
            topic: Topic name
            content: Content to append
            meta: Optional metadata
            context: Context to append to
            ttl: Time-To-Live for the event:
                - "forever": The event is kept indefinitely.
                - "ephemeral": The event is not stored; only active subscribers can see it.
                - "time:<milliseconds>": The event is kept for a custom duration in milliseconds.
                - "head:<n>": Retains only the last n events for the topic (n must be >= 1).

        Returns:
            Frame data
        """
        params = []
        if ttl is not None:
            params.append(f"ttl={ttl}")

        if context is not None:
            ctx = xs_context(context)
            params.append(f"context={ctx}")

        url_params = "&".join(params)
        path = f"{topic}?{url_params}" if url_params else topic

        headers = {}
        if meta is not None:
            meta_json = json.dumps(meta)
            meta_b64 = base64.b64encode(meta_json.encode("utf-8")).decode("utf-8")
            headers["xs-meta"] = meta_b64

        resp = request("POST", path, data=content.encode("utf-8"), headers=headers)
        return json.loads(resp)

    @staticmethod
    def remove(id: str) -> None:
        """
        Remove a frame by ID

        Args:
            id: Frame ID
        """
        request("DELETE", id)

    @staticmethod
    def rm(id: str) -> None:
        """Alias for remove"""
        XS.remove(id)

    class Ctx:
        @staticmethod
        def get() -> str:
            """Get current context ID"""
            return os.environ.get("XS_CONTEXT", XS_CONTEXT_SYSTEM)

        @staticmethod
        def list() -> List[Dict]:
            """List all contexts"""
            active = XS.Ctx.get()
            contexts = xs_context_collect()
            for context in contexts:
                context["active"] = context["id"] == active
            return contexts

        @staticmethod
        def ls() -> List[Dict]:
            """Alias for list"""
            return XS.Ctx.list()

        @staticmethod
        def switch(id: Optional[str] = None) -> str:
            """
            Switch to a different context

            Args:
                id: Context ID or name

            Returns:
                The active context ID
            """
            if id is None:
                # Interactive selection would go here
                # For this example, just return current context
                return XS.Ctx.get()

            os.environ["XS_CONTEXT"] = xs_context(id)
            return XS.Ctx.get()

        @staticmethod
        def new(name: str) -> str:
            """
            Create a new context

            Args:
                name: Context name

            Returns:
                New context ID
            """
            frame = XS.append("xs.context", "", meta={"name": name}, context=XS_CONTEXT_SYSTEM)
            return XS.Ctx.switch(frame["id"])

        @staticmethod
        def rename(id: str, name: str) -> None:
            """
            Rename a context

            Args:
                id: Context ID
                name: New name
            """
            XS.append("xs.annotate", "", meta={
                "updates": xs_context(id),
                "name": name
            }, context=XS_CONTEXT_SYSTEM)

    @staticmethod
    def export(path: str) -> None:
        """
        Export store to a file

        Args:
            path: Export path
        """
        if os.path.exists(path):
            print("path exists")
            return

        os.makedirs(os.path.join(path, "cas"), exist_ok=True)

        frames_path = os.path.join(path, "frames.jsonl")

        with open(frames_path, "w") as f:
            for frame in _cat({}):
                f.write(json.dumps(frame) + "\n")

        # Get unique hashes
        hashes = set()
        with open(frames_path, "r") as f:
            for line in f:
                frame = json.loads(line)
                if "hash" in frame and frame["hash"]:
                    hashes.add(frame["hash"])

        # Save content for each hash
        for hash_value in hashes:
            hash_b64 = base64.b64encode(hash_value.encode()).decode()
            out_path = os.path.join(path, "cas", hash_b64)
            content = XS.cas(hash_value)
            with open(out_path, "w") as f:
                f.write(content)

    @staticmethod
    def import_store(path: str) -> None:
        """
        Import store from a file

        Args:
            path: Import path
        """
        # Import cas files
        cas_files = glob.glob(os.path.join(path, "cas", "*"))
        for cas_file in cas_files:
            hash_b64 = os.path.basename(cas_file)
            hash_value = base64.b64decode(hash_b64).decode()

            with open(cas_file, "r") as f:
                content = f.read()

            # Post to CAS
            headers = {"Content-Type": "application/octet-stream"}
            resp = request("POST", "cas", data=content.encode(), headers=headers)
            got = resp.read().decode()

            if got != hash_value:
                raise ValueError(f"Hash mismatch: got={got}, want={hash_value}")

        # Import frames
        frames_path = os.path.join(path, "frames.jsonl")
        with open(frames_path, "r") as f:
            for line in f:
                frame = json.loads(line)
                if "context_id" not in frame:
                    frame["context_id"] = XS_CONTEXT_SYSTEM

                headers = {"Content-Type": "application/json"}
                request("POST", "import", data=json.dumps(frame).encode(), headers=headers)


# Create convenience aliases for easier access
cat = XS.cat
cas = XS.cas
get = XS.get
head = XS.head
append = XS.append
remove = XS.remove
rm = XS.rm
ctx = XS.Ctx
export = XS.export
import_store = XS.import_store


if __name__ == "__main__":
    # Example usage
    # print(cat())
    pass
