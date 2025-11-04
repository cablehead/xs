import type {
  AcceptType,
  AppendOptions,
  Frame,
  HeadOptions,
  ReadOptions,
  TTL,
  VersionInfo,
} from "./types.ts";

/**
 * Client for interacting with the xs store HTTP API
 */
export class XsStoreClient {
  private baseUrl: string;
  private headers: Record<string, string>;

  /**
   * Create a new xs store client
   * @param baseUrl - Base URL of the xs store server (e.g., "http://localhost:8080")
   * @param options - Optional configuration
   */
  constructor(
    baseUrl: string,
    options?: {
      /** Additional headers to include in all requests */
      headers?: Record<string, string>;
    },
  ) {
    this.baseUrl = baseUrl.replace(/\/$/, ""); // Remove trailing slash
    this.headers = options?.headers ?? {};
  }

  /**
   * Read events from the stream
   * @param options - Read options for filtering and following
   * @param acceptType - Response format: "ndjson" or "sse" (default: "ndjson")
   * @returns Async iterable of frames
   */
  async *cat(
    options: ReadOptions = {},
    acceptType: AcceptType = "ndjson",
  ): AsyncIterable<Frame> {
    const params = new URLSearchParams();

    // Handle follow option
    if (options.follow !== undefined && options.follow !== false) {
      if (options.follow === true) {
        params.set("follow", "true");
      } else {
        params.set("follow", options.follow.toString());
      }
    }

    if (options.tail) params.set("tail", "true");
    if (options.lastId) params.set("last-id", options.lastId);
    if (options.limit) params.set("limit", options.limit.toString());
    if (options.contextId) params.set("context-id", options.contextId);
    if (options.topic) params.set("topic", options.topic);

    const queryString = params.toString();
    const url = `${this.baseUrl}/${queryString ? `?${queryString}` : ""}`;

    const headers = { ...this.headers };
    if (acceptType === "sse") {
      headers["Accept"] = "text/event-stream";
    }

    const response = await fetch(url, { headers });

    if (!response.ok) {
      throw new Error(
        `Failed to read stream: ${response.status} ${response.statusText}`,
      );
    }

    if (!response.body) {
      throw new Error("Response body is null");
    }

    if (acceptType === "sse") {
      yield* this.parseSSE(response.body);
    } else {
      yield* this.parseNDJSON(response.body);
    }
  }

  /**
   * Get a specific frame by ID
   * @param id - Frame ID (SCRU128)
   * @returns Frame or null if not found
   */
  async get(id: string): Promise<Frame | null> {
    const response = await fetch(`${this.baseUrl}/${id}`, {
      headers: this.headers,
    });

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      throw new Error(
        `Failed to get frame: ${response.status} ${response.statusText}`,
      );
    }

    return await response.json();
  }

  /**
   * Append an event to the stream
   * @param topic - Topic for the event
   * @param content - Event content (string, Uint8Array, or ReadableStream)
   * @param options - Append options
   * @returns The created frame
   */
  async append(
    topic: string,
    content?: string | Uint8Array | ReadableStream<Uint8Array>,
    options: AppendOptions = {},
  ): Promise<Frame> {
    const params = new URLSearchParams();
    if (options.contextId) params.set("context", options.contextId);
    if (options.ttl) params.set("ttl", options.ttl);

    const queryString = params.toString();
    const url =
      `${this.baseUrl}/${topic}${queryString ? `?${queryString}` : ""}`;

    const headers = { ...this.headers };

    // Encode meta as base64 if provided
    if (options.meta !== undefined) {
      const metaJson = JSON.stringify(options.meta);
      const metaBase64 = btoa(metaJson);
      headers["xs-meta"] = metaBase64;
    }

    // Convert content to appropriate body type
    let body: BodyInit | null = null;
    if (typeof content === "string") {
      body = content;
    } else if (content instanceof Uint8Array) {
      body = content as BodyInit;
    } else if (content instanceof ReadableStream) {
      body = content as BodyInit;
    }

    const response = await fetch(url, {
      method: "POST",
      headers,
      body,
    });

    if (!response.ok) {
      throw new Error(
        `Failed to append event: ${response.status} ${response.statusText}`,
      );
    }

    return await response.json();
  }

  /**
   * Remove an event by ID
   * @param id - Frame ID to remove
   */
  async remove(id: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/${id}`, {
      method: "DELETE",
      headers: this.headers,
    });

    if (!response.ok) {
      throw new Error(
        `Failed to remove frame: ${response.status} ${response.statusText}`,
      );
    }
  }

  /**
   * Get the most recent event for a topic
   * @param topic - Topic to query
   * @param options - Head options
   * @returns Frame or null if not found, or async iterable if following
   */
  async head(
    topic: string,
    options: HeadOptions = {},
  ): Promise<Frame | null | AsyncIterable<Frame>> {
    const params = new URLSearchParams();
    if (options.follow) params.set("follow", "true");
    if (options.contextId) params.set("context", options.contextId);

    const queryString = params.toString();
    const url =
      `${this.baseUrl}/head/${topic}${queryString ? `?${queryString}` : ""}`;

    const response = await fetch(url, { headers: this.headers });

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      throw new Error(
        `Failed to get head: ${response.status} ${response.statusText}`,
      );
    }

    // If following, return async iterable
    if (options.follow) {
      if (!response.body) {
        throw new Error("Response body is null");
      }
      return this.parseNDJSON(response.body);
    }

    // Otherwise return single frame
    return await response.json();
  }

  /**
   * Get content from content-addressable storage
   * @param hash - Content hash (ssri Integrity format)
   * @returns ReadableStream of content
   */
  async cas(hash: string): Promise<ReadableStream<Uint8Array> | null> {
    const response = await fetch(`${this.baseUrl}/cas/${hash}`, {
      headers: this.headers,
    });

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      throw new Error(
        `Failed to get CAS content: ${response.status} ${response.statusText}`,
      );
    }

    return response.body;
  }

  /**
   * Upload content to content-addressable storage
   * @param content - Content to upload
   * @returns Content hash (ssri Integrity format)
   */
  async casPost(
    content: string | Uint8Array | ReadableStream<Uint8Array>,
  ): Promise<string> {
    let body: BodyInit;
    if (typeof content === "string") {
      body = content;
    } else if (content instanceof Uint8Array) {
      body = content as BodyInit;
    } else {
      body = content as BodyInit;
    }

    const response = await fetch(`${this.baseUrl}/cas`, {
      method: "POST",
      headers: this.headers,
      body,
    });

    if (!response.ok) {
      throw new Error(
        `Failed to upload CAS content: ${response.status} ${response.statusText}`,
      );
    }

    return await response.text();
  }

  /**
   * Import a frame directly
   * @param frame - Frame to import
   * @returns The imported frame
   */
  async import(frame: Frame): Promise<Frame> {
    const response = await fetch(`${this.baseUrl}/import`, {
      method: "POST",
      headers: {
        ...this.headers,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(frame),
    });

    if (!response.ok) {
      throw new Error(
        `Failed to import frame: ${response.status} ${response.statusText}`,
      );
    }

    return await response.json();
  }

  /**
   * Execute a Nushell script on the server
   * @param script - Nushell script to execute
   * @returns Response based on script output type
   */
  async exec(script: string): Promise<Response> {
    const response = await fetch(`${this.baseUrl}/exec`, {
      method: "POST",
      headers: this.headers,
      body: script,
    });

    if (!response.ok) {
      throw new Error(
        `Failed to execute script: ${response.status} ${response.statusText}`,
      );
    }

    return response;
  }

  /**
   * Get server version information
   * @returns Version information
   */
  async version(): Promise<VersionInfo> {
    const response = await fetch(`${this.baseUrl}/version`, {
      headers: this.headers,
    });

    if (!response.ok) {
      throw new Error(
        `Failed to get version: ${response.status} ${response.statusText}`,
      );
    }

    return await response.json();
  }

  /**
   * Parse NDJSON stream into frames
   */
  private async *parseNDJSON(
    body: ReadableStream<Uint8Array>,
  ): AsyncIterable<Frame> {
    const reader = body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n");
        buffer = lines.pop() || "";

        for (const line of lines) {
          if (line.trim()) {
            yield JSON.parse(line);
          }
        }
      }

      // Process remaining buffer
      if (buffer.trim()) {
        yield JSON.parse(buffer);
      }
    } finally {
      reader.releaseLock();
    }
  }

  /**
   * Parse Server-Sent Events stream into frames
   */
  private async *parseSSE(
    body: ReadableStream<Uint8Array>,
  ): AsyncIterable<Frame> {
    const reader = body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const events = buffer.split("\n\n");
        buffer = events.pop() || "";

        for (const event of events) {
          if (!event.trim()) continue;

          const lines = event.split("\n");
          let data = "";

          for (const line of lines) {
            if (line.startsWith("data: ")) {
              data = line.substring(6);
              break;
            }
          }

          if (data) {
            yield JSON.parse(data);
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
  }
}
