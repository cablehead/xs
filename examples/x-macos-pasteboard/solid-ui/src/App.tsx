import { Component, For, Show } from "solid-js";
import { useFrameStream } from "./stream";
import { useStore } from "./store";
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

  const { CAS, index } = useStore({
    dataSignal: frameSignal,
    fetchContent,
  });

  return (
    <div>
      <h1 style="text-align: right;">clipboard</h1>
      <For each={index()}>
        {(frame) => (
          <Show when={frame.hash}>
            <Card frame={frame} content={CAS[frame.hash || ""]} />
          </Show>
        )}
      </For>
    </div>
  );
};

export default App;
