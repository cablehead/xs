# xs-store TypeScript Client

A TypeScript/JavaScript client for interacting with the [xs](https://github.com/cablehead/xs) store HTTP API.

## Features

- **Full API Coverage**: Supports all xs store operations
- **Streaming Support**: Async iterables for streaming events
- **Type-Safe**: Complete TypeScript type definitions
- **Modern**: Built for Deno with standards-compliant fetch API
- **Flexible**: Works in browsers, Deno, and Node.js (with fetch polyfill)

## Installation

### Deno

```typescript
import { XsStoreClient } from "https://raw.githubusercontent.com/cablehead/xs/main/client-ts/mod.ts";
```

### Node.js / Bun

Copy the files or use as a git submodule. Requires a fetch implementation (built-in in Node 18+).

## Usage

### Basic Example

```typescript
import { XsStoreClient } from "./mod.ts";

const client = new XsStoreClient("http://localhost:8080");

// Append an event
const frame = await client.append("my.topic", "Hello, world!", {
  meta: { source: "example" },
});

console.log("Created frame:", frame.id);

// Read events
for await (const frame of client.cat({ topic: "my.topic" })) {
  console.log(frame.topic, frame.meta);
}
```

### Streaming with Follow

```typescript
// Follow the stream for new events
for await (const frame of client.cat({ follow: true, tail: true })) {
  console.log("New event:", frame.topic);
}
```

### Content-Addressable Storage

```typescript
// Upload content
const hash = await client.casPost("Hello, CAS!");

// Append event with CAS reference
await client.append("my.topic", null, { meta: { hash } });

// Retrieve content
const stream = await client.cas(hash);
if (stream) {
  const content = await new Response(stream).text();
  console.log(content); // "Hello, CAS!"
}
```

### Working with Contexts

```typescript
// Append to a specific context
await client.append("my.topic", "data", {
  contextId: "01234567890abcdef01234567",
});

// Read from a specific context
for await (const frame of client.cat({ contextId: "01234567890abcdef01234567" })) {
  console.log(frame);
}
```

### TTL Options

```typescript
// Keep forever (default)
await client.append("my.topic", "data", { ttl: "forever" });

// Ephemeral (not stored, only for active subscribers)
await client.append("my.topic", "data", { ttl: "ephemeral" });

// Keep for 1 hour
await client.append("my.topic", "data", { ttl: "time:3600000" });

// Keep only last 10 events for this topic
await client.append("my.topic", "data", { ttl: "head:10" });
```

### Execute Nushell Scripts

```typescript
const response = await client.exec(`
  .cat --limit 10 | each { |frame| $frame.topic } | uniq
`);

const topics = await response.json();
console.log("Topics:", topics);
```

## API Reference

### Constructor

```typescript
new XsStoreClient(baseUrl: string, options?: { headers?: Record<string, string> })
```

Creates a new client instance.

### Methods

#### `cat(options?, acceptType?)`

Read events from the stream.

**Parameters:**
- `options`: ReadOptions (optional)
  - `follow`: boolean | number - Follow for new events (with optional heartbeat interval)
  - `tail`: boolean - Start from end of stream
  - `lastId`: string - Resume after this frame ID
  - `limit`: number - Maximum frames to return
  - `contextId`: string - Filter by context
  - `topic`: string - Filter by topic
- `acceptType`: "ndjson" | "sse" - Response format (default: "ndjson")

**Returns:** `AsyncIterable<Frame>`

#### `get(id)`

Get a specific frame by ID.

**Returns:** `Promise<Frame | null>`

#### `append(topic, content?, options?)`

Append an event to the stream.

**Parameters:**
- `topic`: string - Event topic
- `content`: string | Uint8Array | ReadableStream (optional)
- `options`: AppendOptions (optional)
  - `meta`: any - Metadata
  - `contextId`: string - Context
  - `ttl`: TTL - Time-to-live setting

**Returns:** `Promise<Frame>`

#### `remove(id)`

Remove an event by ID.

**Returns:** `Promise<void>`

#### `head(topic, options?)`

Get the most recent event for a topic.

**Parameters:**
- `topic`: string
- `options`: HeadOptions (optional)
  - `follow`: boolean - Stream updates
  - `contextId`: string - Context

**Returns:** `Promise<Frame | null | AsyncIterable<Frame>>`

#### `cas(hash)`

Get content from CAS.

**Returns:** `Promise<ReadableStream<Uint8Array> | null>`

#### `casPost(content)`

Upload content to CAS.

**Returns:** `Promise<string>` - Content hash

#### `import(frame)`

Import a frame directly.

**Returns:** `Promise<Frame>`

#### `exec(script)`

Execute a Nushell script.

**Returns:** `Promise<Response>`

#### `version()`

Get server version.

**Returns:** `Promise<VersionInfo>`

## Types

See [types.ts](./types.ts) for complete type definitions.

## Development

```bash
# Check types
deno task check

# Run tests
deno task test

# Format code
deno task fmt

# Lint code
deno task lint
```

## License

Same as xs project (check main repository)
