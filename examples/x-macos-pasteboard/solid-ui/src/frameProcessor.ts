import { createEffect, createSignal } from "solid-js";
import { Frame, useFrameStream } from "./stream";

type FrameMap = Record<string, Frame[]>;

export function useFrameProcessor() {
  const [frameMap, setFrameMap] = createSignal<FrameMap>({});
  const frame = useFrameStream();

  const processFrame = (newFrame: Frame) => {
    const { id, topic, meta } = newFrame;

    // Ignore frames unless they have the topic "pb.recv" or "content"
    if (topic !== "pb.recv" && topic !== "content") {
      return;
    }

    setFrameMap((map) => {
      const updatedMap = { ...map };

      if (topic === "pb.recv") {
        // Update the map to hold only the current frame under `id`
        updatedMap[id] = [newFrame];
      } else if (topic === "content" && meta?.updates?.id) {
        const updateId = meta.updates.id;

        // Append new frame to existing array of frames for `updateId`
        updatedMap[updateId] = [newFrame, ...(updatedMap[updateId] || [])];
      }

      return updatedMap;
    });
  };

  // Use createEffect to watch the frame signal and process each new frame
  createEffect(() => {
    const newFrame = frame();
    if (newFrame) processFrame(newFrame);
  });

  return frameMap;
}
