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
from typing import Dict, List, Optional, Union, Any, Callable, Iterator
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


def is_hex_number(s: str) -> bool:
    """Check if a string is a hexadecimal number (used for chunk size detection)"""
    if not s:
        return False

    # Handle potential chunk extensions by splitting at semicolon
    if ';' in s:
        s = s.split(';', 1)[0]

    try:
        int(s, 16)
        return True
    except ValueError:
        return False


def request(method: str, path: str = "", data: bytes = None, headers: Dict = None, debug=False) -> str:
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

            # Find where headers end and body begins
            header_end = response_text.find("\r\n\r\n")
            if header_end == -1:
                return response_text

            headers_text = response_text[:header_end]
            body = response_text[header_end + 4:]

            # Check if response is chunked
            if "Transfer-Encoding: chunked" in headers_text:
                # Handle chunked encoding manually
                lines = []
                pos = 0

                while pos < len(body):
                    # Find the chunk size line
                    chunk_size_end = body.find("\r\n", pos)
                    if chunk_size_end == -1:
                        break

                    # Parse hex chunk size (ignore chunk extensions)
                    chunk_size_line = body[pos:chunk_size_end].strip()
                    if ';' in chunk_size_line:
                        chunk_size_line = chunk_size_line.split(';', 1)[0]

                    try:
                        chunk_size = int(chunk_size_line, 16)
                    except ValueError:
                        # Not a valid chunk size line, might already be in the data
                        pos += 1
                        continue

                    if chunk_size == 0:
                        # End of chunks
                        break

                    # Move past the chunk size line
                    pos = chunk_size_end + 2

                    # Get chunk data
                    chunk_data = body[pos:pos + chunk_size]
                    pos += chunk_size + 2  # Skip past chunk and CRLF

                    # Add normalized lines from this chunk
                    chunk_lines = chunk_data.splitlines()
                    lines.extend(chunk_lines)

                return "\n".join(lines)
            else:
                # Not chunked, return body directly
                return body

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


def _cat(options: Dict) -> Iterator[Dict]:
    """Read frames from the store, returning an iterator of frames"""
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

    addr = xs_addr()

    try:
        # For follow mode, use streaming approach
        if options.get("follow", False):
            yield from _cat_stream(path, headers)
            return

        # For non-follow mode, use the simpler approach
        content = request("GET", path, headers=headers)

        for line in content.splitlines():
            line = line.strip()

            # Skip empty lines and hex chunk size lines
            if not line or is_hex_number(line):
                continue

            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                # Skip invalid JSON lines
                continue

    except Exception as e:
        print(f"Error in _cat: {e}")


def _cat_stream(path: str, headers: Dict) -> Iterator[Dict]:
    """Stream frames from the store when using follow mode"""
    addr = xs_addr()

    if addr.startswith('./') or addr.startswith('/'):
        # Unix socket connection
        sock_path = f"{addr}/sock"
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

        try:
            sock.connect(sock_path)

            # Construct HTTP request
            req_parts = [f"GET /{path} HTTP/1.1", "Host: localhost", "Connection: keep-alive"]

            # Add headers
            if headers:
                for key, value in headers.items():
                    req_parts.append(f"{key}: {value}")

            # Finish headers
            req_parts.append("")
            req_parts.append("")

            # Send request
            sock.sendall("\r\n".join(req_parts).encode("utf-8"))

            # Process response
            buffer = b""
            header_received = False

            # First, yield existing frames
            existing_frames = list(_cat({
                "context": path.split("context=")[1].split("&")[0] if "context=" in path else None,
                "follow": False
            }))

            for frame in existing_frames:
                yield frame

            # Then keep connection open for new frames
            while True:
                chunk = sock.recv(4096)
                if not chunk:
                    break

                buffer += chunk

                # Handle initial headers
                if not header_received and b"\r\n\r\n" in buffer:
                    header_end = buffer.find(b"\r\n\r\n")
                    buffer = buffer[header_end + 4:]
                    header_received = True

                # Process complete lines
                while b"\n" in buffer:
                    line_end = buffer.find(b"\n")
                    line = buffer[:line_end].strip().decode("utf-8")
                    buffer = buffer[line_end + 1:]

                    if not line or is_hex_number(line):
                        continue

                    try:
                        yield json.loads(line)
                    except json.JSONDecodeError:
                        continue

        except Exception as e:
            print(f"Error in streaming: {e}")
        finally:
            sock.close()
    else:
        # HTTP(S) connection
        # First, yield existing frames
        existing_frames = list(_cat({
            "context": path.split("context=")[1].split("&")[0] if "context=" in path else None,
            "follow": False
        }))

        for frame in existing_frames:
            yield frame

        # Then keep connection open for new frames
        req = urllib.request.Request(url=f"{addr}/{path}", method="GET", headers=headers or {})
        try:
            with urllib.request.urlopen(req) as response:
                for line in response:
                    line = line.decode("utf-8").strip()
                    if not line or is_hex_number(line):
                        continue

                    try:
                        yield json.loads(line)
                    except json.JSONDecodeError:
                        continue
        except Exception as e:
            print(f"Error in streaming: {e}")
            return


def xs_context_collect() -> List[Dict[str, str]]:
    """Collect all contexts"""
    # Start with system context
    contexts = {XS_CONTEXT_SYSTEM: "system"}

    try:
        # Get frames by using _cat directly
        for frame in _cat({"context": XS_CONTEXT_SYSTEM}):
            if not isinstance(frame, dict):
                continue

            if frame.get("topic") == "xs.context":
                meta = frame.get("meta", {})
                if isinstance(meta, dict) and "name" in meta:
                    contexts[frame.get("id")] = meta["name"]
            elif frame.get("topic") == "xs.annotate":
                meta = frame.get("meta", {})
                if isinstance(meta, dict) and "updates" in meta and "name" in meta:
                    if meta["updates"] in contexts:
                        contexts[meta["updates"]] = meta["name"]
    except Exception as e:
        print(f"Warning: Error collecting contexts: {e}")

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


class XS:
    @staticmethod
    def cat(follow: bool = False, pulse: Optional[int] = None, tail: bool = False,
            last_id: Optional[str] = None, limit: Optional[int] = None,
            context: Optional[str] = None, all: bool = False) -> Iterator[Dict]:
        """
        Cat the event stream

        Args:
            follow: long poll for new events and yield frames as they arrive
            pulse: specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
            tail: begin long after the end of the stream
            last_id: start reading from a specific frame ID
            limit: maximum number of frames to return
            context: the context to read from
            all: cat across all contexts

        Returns:
            Iterator yielding frames (for both streaming and non-streaming requests)
        """
        ctx = None
        if not all and context is not None:
            ctx = xs_context(context)

        yield from _cat({
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
