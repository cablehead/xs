import { css, html, LitElement } from "https://cdn.skypack.dev/lit";

export class TerminalLine extends LitElement {
  static properties = {
    prompt: { type: String },
    command: { type: String },
    pipe: { type: String },
    topic: { type: String },
  };

  static styles = css`
    :host {
      display: block;
      margin: 10px 0;
      color: #8b949e;
      line-height: 1.4;
      white-space: nowrap;
    }

    .prompt {
      color: #79c0ff;
    }

    .command {
      color: #a5d6ff;
    }

    .pipe {
      color: #ff7b72;
    }

    .topic {
      color: #ffa657;
    }
  `;

  render() {
    return html`
      ${this.prompt
        ? html`
          <span class="prompt">${this.prompt}</span>
        `
        : ""} ${this.command
        ? html`
          <span class="command">${this.command}</span>
        `
        : ""} ${this.pipe
        ? html`
          <span class="pipe">${this.pipe}</span>
        `
        : ""} ${this.topic
        ? html`
          <span class="topic">${this.topic}</span>
        `
        : ""}
      <slot></slot>
    `;
  }
}

customElements.define("terminal-line", TerminalLine);
