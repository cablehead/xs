import { Component, For } from "solid-js";
import { useFrameProcessor } from "./frameProcessor";

const App: Component = () => {
  const frameMap = useFrameProcessor();

  return (
    <div>
      <h1>Processed Frames</h1>
      <For each={Object.entries(frameMap())}>
        {([key, frames]) => (
          <div>
            <h2>Frames for ID: {key}</h2>
            <ul>
              <For each={frames}>
                {(frame) => (
                  <li>
                    {frame.id}: {frame.topic}
                  </li>
                )}
              </For>
            </ul>
          </div>
        )}
      </For>
    </div>
  );
};

export default App;
