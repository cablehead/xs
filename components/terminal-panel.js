import { css, html, LitElement } from "https://cdn.skypack.dev/lit";

export class TerminalPanel extends LitElement {
  static properties = {
    title: { type: String },
    actions: { type: Array },
  };

  constructor() {
    super();
    this.actions = [];
  }

  static styles = css`
    :host {
      display: block;
    }

    .terminal-panel {
      background: var(--terminal-bg);
      border: 1px solid var(--terminal-border);
      border-radius: var(--terminal-radius);
      overflow: hidden;
      font-family: var(--terminal-font);
    }

    .terminal-header {
      background: var(--terminal-header-bg);
      border-bottom: 1px solid var(--terminal-border);
      padding: 8px 20px 8px 85px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      position: relative;
      min-height: 44px;
      box-sizing: border-box;
      width: 100%;
    }

    .terminal-header::before {
      content: "";
      position: absolute;
      top: 16px;
      left: 16px;
      width: 12px;
      height: 12px;
      border-radius: 50%;
      background: var(--terminal-red);
      box-shadow: 20px 0 0 var(--terminal-yellow), 40px 0 0 var(--terminal-green);
    }

    .terminal-title {
      color: var(--terminal-title);
      font-size: 14px;
      font-weight: 600;
      margin: 0;
      flex-shrink: 1;
      min-width: 0;
      overflow: hidden;
    }

    .terminal-actions {
      display: flex;
      flex-direction: row;
      gap: 10px;
      border: 2px solid red;
      flex: 1;
      justify-content: flex-end;
    }

    .action-button {
      background: var(--bg-primary);
      border: 1px solid var(--border-default);
      border-radius: 4px;
      padding: 8px 12px;
      cursor: pointer;
      transition: all 0.3s ease;
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 500;
    }

    .action-button:hover {
      border-color: var(--accent-blue);
      background: color-mix(in srgb, var(--accent-blue) 5%, transparent);
    }

    .action-button.selected {
      border-color: var(--accent-blue);
      background: color-mix(in srgb, var(--accent-blue) 10%, transparent);
    }

    .action-button .title {
      color: var(--accent-blue);
    }

    .terminal-content {
      min-height: 200px;
      padding: 20px;
      background: var(--terminal-bg);
      color: var(--terminal-text);
      line-height: 1.4;
      overflow-x: auto;
    }
  `;

  _handleActionClick(action) {
    this.dispatchEvent(
      new CustomEvent("action-click", {
        detail: { actionId: action.id, action },
      }),
    );
  }

  render() {
    return html`
      <div class="terminal-panel">
        <div class="terminal-header">
          ${this.title
            ? html`
              <div class="terminal-title">${this.title}</div>
            `
            : ""} ${this.actions && this.actions.length > 0
            ? html`
              <div class="terminal-actions">
                ${this.actions.map((action) =>
                  html`
                    <button
                      class="action-button ${action.selected ? "selected" : ""}"
                      @click="${() => this._handleActionClick(action)}"
                    >
                      <div class="title">${action.title}</div>
                    </button>
                  `
                )}
              </div>
            `
            : ""}
        </div>
        <div class="terminal-content">
          <slot></slot>
        </div>
      </div>
    `;
  }
}

customElements.define("terminal-panel", TerminalPanel);
