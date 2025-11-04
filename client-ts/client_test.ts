/**
 * Tests for xs-store TypeScript client
 *
 * To run these tests:
 * 1. Start xs serve in another terminal:
 *    xs serve /tmp/test-store
 *
 * 2. Run tests:
 *    deno test --allow-net client_test.ts
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.208.0/assert/mod.ts";
import { XsStoreClient } from "./mod.ts";

const TEST_URL = Deno.env.get("XS_TEST_URL") || "http://localhost:7777";

Deno.test("XsStoreClient - version", async () => {
  const client = new XsStoreClient(TEST_URL);
  const version = await client.version();
  assertExists(version.version);
});

Deno.test("XsStoreClient - append and get", async () => {
  const client = new XsStoreClient(TEST_URL);

  const frame = await client.append("test.basic", "test content", {
    meta: { test: true },
  });

  assertExists(frame.id);
  assertEquals(frame.topic, "test.basic");
  assertEquals(frame.meta, { test: true });

  const retrieved = await client.get(frame.id);
  assertExists(retrieved);
  assertEquals(retrieved!.id, frame.id);
  assertEquals(retrieved!.topic, "test.basic");
});

Deno.test("XsStoreClient - append without content", async () => {
  const client = new XsStoreClient(TEST_URL);

  const frame = await client.append("test.no-content", undefined, {
    meta: { hasContent: false },
  });

  assertExists(frame.id);
  assertEquals(frame.topic, "test.no-content");
  assertEquals(frame.hash, undefined);
});

Deno.test("XsStoreClient - cat with limit", async () => {
  const client = new XsStoreClient(TEST_URL);

  // Create some test events
  await client.append("test.cat", "message 1");
  await client.append("test.cat", "message 2");
  await client.append("test.cat", "message 3");

  const frames = [];
  for await (const frame of client.cat({ limit: 2 })) {
    frames.push(frame);
  }

  assertEquals(frames.length, 2);
});

Deno.test("XsStoreClient - cat with topic filter", async () => {
  const client = new XsStoreClient(TEST_URL);

  const testTopic = `test.filter.${Date.now()}`;
  await client.append(testTopic, "filtered");
  await client.append("test.other", "not filtered");

  const frames = [];
  for await (const frame of client.cat({ topic: testTopic })) {
    frames.push(frame);
  }

  assertEquals(frames.length >= 1, true);
  assertEquals(frames.every((f) => f.topic === testTopic), true);
});

Deno.test("XsStoreClient - head", async () => {
  const client = new XsStoreClient(TEST_URL);

  const testTopic = `test.head.${Date.now()}`;
  const frame1 = await client.append(testTopic, "first");
  const frame2 = await client.append(testTopic, "second");

  const head = await client.head(testTopic);
  assertExists(head);

  if (head && typeof head === 'object' && 'id' in head) {
    assertEquals(head.id, frame2.id);
  }
});

Deno.test("XsStoreClient - CAS operations", async () => {
  const client = new XsStoreClient(TEST_URL);

  const content = "Hello from CAS!";
  const hash = await client.casPost(content);
  assertExists(hash);

  const stream = await client.cas(hash);
  assertExists(stream);

  const retrieved = await new Response(stream!).text();
  assertEquals(retrieved, content);
});

Deno.test("XsStoreClient - remove", async () => {
  const client = new XsStoreClient(TEST_URL);

  const frame = await client.append("test.remove", "to be deleted");
  assertExists(frame.id);

  await client.remove(frame.id);

  const deleted = await client.get(frame.id);
  assertEquals(deleted, null);
});

Deno.test("XsStoreClient - TTL options", async () => {
  const client = new XsStoreClient(TEST_URL);

  const frame1 = await client.append("test.ttl", "forever", {
    ttl: "forever",
  });
  assertEquals(frame1.ttl, "forever");

  const frame2 = await client.append("test.ttl", "ephemeral", {
    ttl: "ephemeral",
  });
  assertEquals(frame2.ttl, "ephemeral");

  const frame3 = await client.append("test.ttl", "timed", {
    ttl: "time:60000",
  });
  assertEquals(frame3.ttl, "time:60000");

  const frame4 = await client.append("test.ttl", "head-limited", {
    ttl: "head:5",
  });
  assertEquals(frame4.ttl, "head:5");
});

Deno.test("XsStoreClient - exec", async () => {
  const client = new XsStoreClient(TEST_URL);

  const response = await client.exec(`"hello from nushell"`);
  const result = await response.text();
  assertEquals(result, "hello from nushell");
});

Deno.test("XsStoreClient - import", async () => {
  const client = new XsStoreClient(TEST_URL);

  // First create a frame to get a valid structure
  const original = await client.append("test.import", "original");

  // Import it again with modified data
  const imported = await client.import({
    ...original,
    topic: "test.imported",
  });

  assertExists(imported);
  assertEquals(imported.topic, "test.imported");
});

Deno.test("XsStoreClient - get non-existent frame", async () => {
  const client = new XsStoreClient(TEST_URL);

  const nonExistent = await client.get("01234567890123456789012");
  assertEquals(nonExistent, null);
});

Deno.test("XsStoreClient - cas non-existent hash", async () => {
  const client = new XsStoreClient(TEST_URL);

  const nonExistent = await client.cas("sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
  assertEquals(nonExistent, null);
});
