import { createEffect, createMemo } from "solid-js";
import { createStore } from "solid-js/store";
import { Frame } from "./stream";

export type StreamStore = { [key: string]: Frame[] };

type StreamProps = {
  dataSignal: () => Frame | null;
};

export function useStore({ dataSignal }: StreamProps) {
  const [frames, setFrames] = createStore<StreamStore>({});

  createEffect(() => {
    const frame = dataSignal();
    if (!frame) return;

    if (frame.topic !== "pb.recv" && frame.topic !== "content") return;

    const frameId = frame.meta?.updates ?? frame.id;
    setFrames(frameId, (existingFrames = []) => [frame, ...existingFrames]);
  });

  const index = createMemo(() => {
    return Object.keys(frames)
      .sort((a, b) => b.localeCompare(a))
      .map((id) => frames[id]);
  });

  return {
    index,
  };
}
