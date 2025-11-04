/**
 * xs-store TypeScript Client
 *
 * A TypeScript client for interacting with the xs store HTTP API.
 * Supports all xs store operations including streaming, CAS, and event management.
 *
 * @example
 * ```typescript
 * import { XsStoreClient } from "./mod.ts";
 *
 * const client = new XsStoreClient("http://localhost:8080");
 *
 * // Append an event
 * const frame = await client.append("my.topic", "Hello, world!", {
 *   meta: { source: "example" },
 * });
 *
 * // Read events
 * for await (const frame of client.cat({ topic: "my.topic" })) {
 *   console.log(frame);
 * }
 * ```
 *
 * @module
 */

export { XsStoreClient } from "./client.ts";
export type {
  AcceptType,
  AppendOptions,
  FollowOption,
  Frame,
  HeadOptions,
  ReadOptions,
  TTL,
  VersionInfo,
} from "./types.ts";
