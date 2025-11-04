/**
 * TTL (Time-To-Live) options for frames
 */
export type TTL =
  | "forever" // Event is kept indefinitely
  | "ephemeral" // Event is not stored; only active subscribers see it
  | `time:${number}` // Event is kept for a custom duration in milliseconds
  | `head:${number}`; // Retains only the last n events for the topic (n >= 1)

/**
 * Frame represents an event in the xs store
 */
export interface Frame {
  /** Unique SCRU128 identifier for this frame */
  id: string;
  /** Context identifier (SCRU128) */
  context_id: string;
  /** Topic/category for this event */
  topic: string;
  /** Optional content hash (ssri Integrity format) */
  hash?: string;
  /** Optional metadata (JSON value) */
  meta?: unknown;
  /** Optional Time-To-Live setting */
  ttl?: TTL;
}

/**
 * Follow options for streaming operations
 */
export type FollowOption =
  | false // Don't follow
  | true // Follow without heartbeat
  | number; // Follow with heartbeat interval in milliseconds

/**
 * Options for reading from the stream
 */
export interface ReadOptions {
  /** Whether to follow/long-poll for new events */
  follow?: FollowOption;
  /** Start reading from the end of the stream */
  tail?: boolean;
  /** Resume reading after this frame ID */
  lastId?: string;
  /** Maximum number of frames to return */
  limit?: number;
  /** Filter by context ID */
  contextId?: string;
  /** Filter by topic */
  topic?: string;
}

/**
 * Options for appending events
 */
export interface AppendOptions {
  /** Optional metadata to include with the event */
  meta?: unknown;
  /** Context to append to (defaults to zero context) */
  contextId?: string;
  /** Time-To-Live for the event */
  ttl?: TTL;
}

/**
 * Options for getting the head of a topic
 */
export interface HeadOptions {
  /** Whether to follow for new head updates */
  follow?: boolean;
  /** Context to query (defaults to zero context) */
  contextId?: string;
}

/**
 * Accept type for streaming responses
 */
export type AcceptType = "ndjson" | "sse";

/**
 * Version information from the server
 */
export interface VersionInfo {
  version: string;
}
