import { LitElement, html, css } from 'https://cdn.skypack.dev/lit';

export class TerminalPanel extends LitElement {
  static properties = {
    title: { type: String },
    actions: { type: Boolean }
  };

  static styles = css`
    :host {
      display: block;
    }
    
    .terminal-panel {
      background: #1e1e1e;
      border: 1px solid #30363d;
      border-radius: 8px;
      overflow: hidden;
      font-family:
        "Courier New", "Courier", "Lucida Console", "Liberation Mono",
        "DejaVu Sans Mono", "Bitstream Vera Sans Mono", "Monaco", monospace;
    }
    
    .terminal-header {
      background: #21262d;
      border-bottom: 1px solid #30363d;
      padding: 8px 20px 8px 85px;
      display: flex;
      align-items: center;
      gap: 20px;
      position: relative;
      min-height: 44px;
      box-sizing: border-box;
    }
    
    .terminal-header::before {
      content: "";
      position: absolute;
      top: 16px;
      left: 16px;
      width: 12px;
      height: 12px;
      border-radius: 50%;
      background: #ff5f56;
      box-shadow: 20px 0 0 #ffbd2e, 40px 0 0 #27ca3f;
    }
    
    .terminal-title {
      color: #ffa657;
      font-size: 14px;
      font-weight: 600;
      margin: 0;
    }
    
    .terminal-actions {
      display: flex;
      gap: 10px;
      margin-left: auto;
    }
    
    .terminal-content {
      min-height: 200px;
      padding: 20px;
      background: #1e1e1e;
      color: #c9d1d9;
      line-height: 1.4;
      overflow-x: auto;
    }
  `;

  render() {
    return html`
      <div class="terminal-panel">
        <div class="terminal-header">
          ${this.title ? html`<div class="terminal-title">${this.title}</div>` : ''}
          ${this.actions ? html`
            <div class="terminal-actions">
              <slot name="actions"></slot>
            </div>
          ` : ''}
        </div>
        <div class="terminal-content">
          <slot></slot>
        </div>
      </div>
    `;
  }
}

customElements.define('terminal-panel', TerminalPanel);