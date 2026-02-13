import { css, html, LitElement } from "https://esm.sh/lit";

export class EventBubble extends LitElement {
  static properties = {
    type: { type: String }, // 'goal' or 'notes'
    eventId: { type: String },
    content: { type: String },
    frameId: { type: String, attribute: "frame-id" },
  };

  static styles = css`
    :host {
      display: block;
      margin-bottom: 1px;
    }

    .event-bubble {
      padding: 10px 14px;
      border-radius: 4px;
      width: 100%;
      text-align: left;
      position: relative;
      cursor: pointer;
      transition: all 0.3s ease;
      transform: scale(1);
      font-family: monospace;
      display: flex;
      justify-content: space-between;
      align-items: center;
      box-sizing: border-box;
    }

    .event-bubble:hover {
      transform: scale(1.02);
    }

    .event-bubble.goal {
      background: rgba(165, 162, 255, 0.15);
      border: 1px solid #a5a2ff;
      color: #a5a2ff;
    }

    .event-bubble.notes {
      background: rgba(121, 192, 255, 0.15);
      border: 1px solid #79c0ff;
      color: #79c0ff;
    }

    .event-id {
      opacity: 0.7;
      font-family: monospace;
      letter-spacing: -0.5px;
    }
  `;

  render() {
    return html`
      <div
        class="event-bubble ${this.type}"
        data-topic="${this.type}"
        data-content="${this.content}"
      >
        <span>${this.type}</span>
        <span class="event-id">${this.frameId}</span>
      </div>
    `;
  }
}

customElements.define("event-bubble", EventBubble);
