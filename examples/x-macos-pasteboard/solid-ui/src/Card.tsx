import { Component, Show } from "solid-js";
import { styled } from "solid-styled-components";
import { Frame } from "./stream";
import { CASStore } from "./store";

const CardWrapper = styled("div")`
  display: flex;
  flex-direction: column;
  margin-bottom: 2em;
  overflow: hidden;
  border-radius: 0.25em;
`;

const Content = styled("div")`
  flex: 1;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 0.25em 0.5em;
`;

const Footer = styled("footer")`
  font-size: 0.7em;
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
  const frame = frames[0];

  const renderContent = () => {
    const content = CAS[frame.hash || ""];
    if (!content) return null;

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
      return <pre>{content}</pre>;
    }
  };

  // Find the first `pb.recv` frame, then extract the `source` from its content in CAS
  const sourceFrame = frames.find((f) => f.topic === "pb.recv");
  let source = null;
  if (sourceFrame) {
    const sourceContent = CAS[sourceFrame.hash || ""];
    if (sourceContent) {
      try {
        const parsedContent = JSON.parse(sourceContent);
        source = parsedContent.source;
      } catch (error) {
        console.error("Failed to parse JSON content for source:", error);
      }
    }
  }

  return (
    <CardWrapper>
      <Content>{renderContent()}</Content>
      <Footer>
        <span>{frame.id}</span>
        {source && <span>{source}</span>}
      </Footer>
    </CardWrapper>
  );
};

export default Card;
