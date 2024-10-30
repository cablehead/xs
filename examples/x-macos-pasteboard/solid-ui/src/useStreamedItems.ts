import { createStore } from "solid-js/store";
import { onCleanup, onMount } from "solid-js";

export interface StreamedItem {
  topic: string;
  meta?: any;
}

interface State {
  items: StreamedItem[];
  error: string | null;
  loading: boolean;
}

export function useStreamedItems() {
  const [state, setState] = createStore<State>({
    items: [],
    error: null,
    loading: true,
  });

  onMount(() => {
    const controller = new AbortController();
    const signal = controller.signal;

    const fetchData = async () => {
      try {
        const response = await fetch("/api?follow", { signal });
        if (!response.ok) {
          throw new Error("Network response was not ok");
        }

        const textStream = response.body!
          .pipeThrough(new TextDecoderStream())
          .pipeThrough(splitStream("\n"));

        const reader = textStream.getReader();

        while (true) {
          const { value, done } = await reader.read();
          if (done) {
            console.log("Stream complete");
            break;
          }
          if (value.trim()) {
            try {
              const json = JSON.parse(value);
              setState("items", (items) => [json, ...items]);
              setState("loading", false);
            } catch (e) {
              console.error("Failed to parse JSON", value, e);
            }
          }
        }

        reader.releaseLock();
        setState("loading", false);
      } catch (err: any) {
        if (err.name !== "AbortError") {
          setState("error", err.message);
          setState("loading", false);
        }
      }
    };

    fetchData();

    onCleanup(() => {
      controller.abort();
    });
  });

  return { state };
}

// Utility function to split a stream by a delimiter
function splitStream(delimiter: string) {
  let buffer = "";
  return new TransformStream<string, string>({
    transform(chunk, controller) {
      buffer += chunk;
      const parts = buffer.split(delimiter);
      buffer = parts.pop()!; // Save the last partial line
      parts.forEach((part) => controller.enqueue(part));
    },
    flush(controller) {
      if (buffer) {
        controller.enqueue(buffer);
      }
    },
  });
}
