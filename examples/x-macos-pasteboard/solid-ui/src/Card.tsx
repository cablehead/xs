import { Component, Show } from "solid-js";
import { styled } from "solid-styled-components";
import { Frame } from "./stream";

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
  font-size: 0.80em;
  color: var(--color-sub-fg);
  background-color: var(--color-sub-bg);

  padding: 0.5em 1em;

  display: flex;
  align-items: center;
  justify-content: start;
`;

type CardProps = {
  frame: Frame;
  content: string | null;
};

const Card: Component<CardProps> = (props) => {
  const renderContent = () => {
    const { frame, content } = props;
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
      return <pre>{content}</pre>;
    }
  };

  return (
    <CardWrapper>
      <Content>{renderContent()}</Content>
      <Footer>{props.frame.id}</Footer>
    </CardWrapper>
  );
};

export default Card;
