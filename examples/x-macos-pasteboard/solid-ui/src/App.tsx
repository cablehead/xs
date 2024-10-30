import { Component, For } from "solid-js";
import { useFrameStream } from "./stream";
import { useStore } from "./store";

const App: Component = () => {
  const frameSignal = useFrameStream();

  const fetchContent = async (hash: string) => {
    const response = await fetch(`/api/cas/${hash}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch content for hash ${hash}`);
    }
    return await response.text();
  };

  const { CAS, index } = useStore({
    dataSignal: frameSignal,
    fetchContent,
  });

  return (
    <div>
      <h1>Latest Frames Content</h1>
      <For each={index()}>
        {(frame) => (
          <div>
            <h2>Frame ID: {frame.id}</h2>
            <p>Content: {CAS[frame.hash || ""]}</p>
          </div>
        )}
      </For>
    </div>
  );
};

export default App;
