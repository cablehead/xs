import { createEffect, createMemo } from "solid-js";
import { createStore } from "solid-js/store";
import { Frame } from "./stream";

export type CASStore = { [key: string]: string };
export type StreamStore = { [key: string]: Frame[] };

type StreamProps = {
  dataSignal: () => Frame | null;
  fetchContent: (hash: string) => Promise<string>;
};

export function useStore({ dataSignal, fetchContent }: StreamProps) {
  const [frames, setFrames] = createStore<StreamStore>({});
  const [CAS, setCAS] = createStore<CASStore>({});

  const fetchAndCacheContent = async (hash: string) => {
    if (CAS[hash]) return; // Content already cached
    try {
      const content = await fetchContent(hash);
      setCAS(hash, content);
    } catch (error) {
      console.error("Error fetching content:", error);
    }
  };

  createEffect(() => {
    const frame = dataSignal();
    if (!frame) return;

    // Only process frames with relevant topics
    if (frame.topic !== "pb.recv" && frame.topic !== "content") return;

    const frameId = frame.meta?.updates?.id ?? frame.id;

    setFrames(frameId, (existingFrames = []) => [frame, ...existingFrames]);

    // If the frame has a hash, fetch and cache its content
    if (frame.hash) {
      fetchAndCacheContent(frame.hash);
    }
  });

  const index = createMemo(() => {
    return Object.keys(frames)
      .sort((a, b) => b.localeCompare(a)) // Sort in descending order by frame ID
      .map((id) => frames[id][0]); // Map to the latest frame for each id
  });

  return {
    CAS,
    index,
  };
}
