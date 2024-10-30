import { Component, For, Show } from "solid-js";
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

  const renderContent = (frame) => {
    const content = CAS[frame.hash || ""];
    if (!content) return null;

    // Conditional rendering based on topic and meta.content_type
    if (frame.topic === "pb.recv") {
      try {
        const jsonContent = JSON.parse(content);
        return <pre>{JSON.stringify(jsonContent, null, 2)}</pre>;
      } catch (error) {
        console.error("Failed to parse JSON content:", error);
        return <p>{content}</p>; // Fallback if JSON parsing fails
      }
    } else if (frame.meta?.content_type === "image") {
      return <img src={`/api/cas/${frame.hash}`} alt="Frame content" />;
    } else {
      return <p>{content}</p>;
    }
  };

  return (
    <div>
      <For each={index()}>
        {(frame) => (
          <Show when={frame.hash}>
            {renderContent(frame)}
          </Show>
        )}
      </For>
    </div>
  );
};

export default App;
