import { Component, For } from "solid-js";
import { useFrames } from "./stream.ts";

const App: Component = () => {
  const frames = useFrames();
  return (
    <div>
      <main>
        <article>
          <For each={frames()}>
            {(item) => (
              <section>
                <ul>
                  <li>
                    {item.id}: {item.topic}
                  </li>
                </ul>
              </section>
            )}
          </For>
        </article>
      </main>
    </div>
  );
};

export default App;
