import { Component, createMemo, createSignal, For, Show } from "solid-js";
import { styled } from "solid-styled-components";
import { Frame } from "./store/stream";
import { CASStore } from "./store/cas";

const CardWrapper = styled("div")`
  display: flex;
  flex-direction: column;
  margin-bottom: 1em;
  overflow: hidden;
  border-radius: 0.25em;
`;

const Content = styled("div")`
  flex: 1;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 0.25em 0.5em;
`;

const Meta = styled("div")`
  font-size: 0.80em;
  color: var(--color-sub-fg);
  background-color: var(--color-sub-bg);
  padding: 0.5em 1em;
  display: flex;
  align-items: center;
  justify-content: space-between;
`;

type CardProps = {
  frames: Frame[];
  CAS: CASStore;
};

const Card: Component<CardProps> = (props) => {
  const { frames, CAS } = props;
  const [currentIndex, setCurrentIndex] = createSignal(0);
  const frame = () => frames[currentIndex()];
  const contentSignal = () => CAS.get(frame().hash);

  const renderContent = () => {
    const content = contentSignal()();
    if (!content) return null;

    if (frame().topic === "pb.recv") {
      try {
        const jsonContent = JSON.parse(content);
        return <pre>{JSON.stringify(jsonContent, null, 2)}</pre>;
      } catch (error) {
        console.error("Failed to parse JSON content:", error);
        return <p>{content}</p>;
      }
    } else if (frame().meta?.content_type === "image") {
      return <img src={`/api/cas/${frame().hash}`} alt="Frame content" />;
    } else {
      return <pre>{content}</pre>;
    }
  };

  // Create a reactive derived signal for `source`
  const source = createMemo(() => {
    const sourceFrame = frames.find((f) => f.topic === "pb.recv");
    if (!sourceFrame) return null;

    const sourceContent = CAS.get(sourceFrame.hash)();
    if (!sourceContent) return null;

    try {
      const parsedContent = JSON.parse(sourceContent);
      return parsedContent.source;
    } catch (error) {
      console.error("Failed to parse JSON content for source:", error);
      return null;
    }
  });

  return (
    <CardWrapper>
      <Meta>
        <span>{frame().id}</span>
        <nav>
          <For each={frames}>
            {(_, idx) => (
              <button
                onClick={() => setCurrentIndex(idx())}
                style={{ margin: "0 0.25em" }}
                disabled={currentIndex() === idx()}
              >
                {idx()}
              </button>
            )}
          </For>
        </nav>
        <Show when={source()}>
          <span>{source()}</span>
        </Show>
      </Meta>
      <Content>{renderContent()}</Content>
    </CardWrapper>
  );
};

export default Card;
