/**
 * Example usage of the xs-store TypeScript client
 *
 * To run this example:
 * 1. Start xs serve in another terminal:
 *    xs serve /tmp/test-store
 *
 * 2. Run this example:
 *    deno run --allow-net example.ts
 */

import { XsStoreClient } from "./mod.ts";

async function main() {
  // Connect to the xs store server
  // Adjust the URL based on your xs serve configuration
  const client = new XsStoreClient("http://localhost:7777");

  try {
    // Get server version
    console.log("\n=== Version ===");
    const version = await client.version();
    console.log("Server version:", version.version);

    // Append some events
    console.log("\n=== Appending Events ===");
    const frame1 = await client.append("example.hello", "Hello, World!", {
      meta: { source: "example", timestamp: new Date().toISOString() },
    });
    console.log("Created frame 1:", frame1.id);

    const frame2 = await client.append("example.hello", "Second message", {
      meta: { source: "example", count: 2 },
      ttl: "head:10", // Keep only last 10 events for this topic
    });
    console.log("Created frame 2:", frame2.id);

    const frame3 = await client.append("example.goodbye", "Farewell", {
      meta: { source: "example" },
    });
    console.log("Created frame 3:", frame3.id);

    // Get a specific frame
    console.log("\n=== Get Frame ===");
    const retrieved = await client.get(frame1.id);
    console.log("Retrieved frame:", retrieved);

    // Read recent events
    console.log("\n=== Read Recent Events ===");
    let count = 0;
    for await (const frame of client.cat({ limit: 5 })) {
      console.log(`  ${frame.id} | ${frame.topic} | ${JSON.stringify(frame.meta)}`);
      count++;
    }
    console.log(`Read ${count} frames`);

    // Read events for a specific topic
    console.log("\n=== Read Topic: example.hello ===");
    for await (const frame of client.cat({ topic: "example.hello" })) {
      console.log(`  ${frame.id} | ${frame.topic}`);
      if (frame.hash) {
        // If there's content, retrieve it
        const stream = await client.cas(frame.hash);
        if (stream) {
          const content = await new Response(stream).text();
          console.log(`    Content: ${content}`);
        }
      }
    }

    // Get head of a topic
    console.log("\n=== Get Head ===");
    const head = await client.head("example.hello");
    if (head && typeof head === 'object' && 'id' in head) {
      console.log("Head of example.hello:", head.id);
    } else {
      console.log("No head found for example.hello");
    }

    // Upload to CAS and reference in event
    console.log("\n=== Content-Addressable Storage ===");
    const content = JSON.stringify({
      message: "This is stored in CAS",
      data: [1, 2, 3, 4, 5],
    });
    const hash = await client.casPost(content);
    console.log("Uploaded to CAS with hash:", hash);

    const frameWithCas = await client.append("example.with-cas", undefined, {
      meta: { contentHash: hash, description: "Event with CAS content" },
    });
    console.log("Created frame with CAS reference:", frameWithCas.id);

    // Retrieve the CAS content
    const casStream = await client.cas(hash);
    if (casStream) {
      const retrieved = await new Response(casStream).text();
      console.log("Retrieved from CAS:", retrieved);
    }

    // Execute a Nushell script
    console.log("\n=== Execute Script ===");
    const scriptResponse = await client.exec(`
      .cat --limit 3 | each { |frame|
        { id: $frame.id, topic: $frame.topic }
      }
    `);
    const scriptResult = await scriptResponse.json();
    console.log("Script result:", scriptResult);

    // Remove an event
    console.log("\n=== Remove Event ===");
    await client.remove(frame3.id);
    console.log("Removed frame:", frame3.id);

    // Verify it's gone
    const deleted = await client.get(frame3.id);
    console.log("Frame after deletion:", deleted); // Should be null

    console.log("\n=== Example Complete ===");
  } catch (error) {
    console.error("Error:", error);
    console.error("\nMake sure xs serve is running:");
    console.error("  xs serve /tmp/test-store");
  }
}

// Run the example
if (import.meta.main) {
  main();
}
