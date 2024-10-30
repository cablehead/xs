import { createSignal, onCleanup, onMount } from "solid-js";

export type Frame = {
  id: string;
  topic: string;
  hash?: string;
  meta?: Record<string, string>;
};

export function useFrames() {
  const [frames, setFrames] = createSignal<Frame[]>([]);

  onMount(() => {
    const controller = new AbortController();
    const signal = controller.signal;

    const fetchData = async () => {
      const response = await fetch("/api?follow", { signal });
      const textStream = response.body!
        .pipeThrough(new TextDecoderStream())
        .pipeThrough(splitStream("\n"));

      const reader = textStream.getReader();

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        if (value.trim()) {
          const json = JSON.parse(value);
          setFrames((frames) => [json, ...frames]); // Add new frames to the beginning
        }
      }

      reader.releaseLock();
    };

    fetchData();

    onCleanup(() => {
      controller.abort();
    });
  });

  return frames;
}

// Utility function to split a stream by a delimiter
function splitStream(delimiter: string) {
  let buffer = "";
  return new TransformStream<string, string>({
    transform(chunk, controller) {
      buffer += chunk;
      const parts = buffer.split(delimiter);
      buffer = parts.pop()!;
      parts.forEach((part) => controller.enqueue(part));
    },
    flush(controller) {
      if (buffer) {
        controller.enqueue(buffer);
      }
    },
  });
}
