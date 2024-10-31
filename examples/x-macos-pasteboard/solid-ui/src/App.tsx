import { Component, For } from "solid-js";
import { useFrameStream } from "./store/stream";
import { useStore } from "./store";
import { createCAS } from "./store/cas";
import Card from "./Card";

const App: Component = () => {
  const frameSignal = useFrameStream();

  const fetchContent = async (hash: string) => {
    const response = await fetch(`/api/cas/${hash}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch content for hash ${hash}`);
    }
    return await response.text();
  };

  const { index } = useStore({ dataSignal: frameSignal });
  const CAS = createCAS(fetchContent);

  return (
    <div>
      <h1>a solid clipboard</h1>
      <For each={index()}>
        {(frames) => <Card frames={frames} CAS={CAS} />}
      </For>
    </div>
  );
};

export default App;
