import { css, html, LitElement } from "https://cdn.skypack.dev/lit";

export class FeatureCard extends LitElement {
  static properties = {
    type: { type: String },
    cardTitle: { type: String, attribute: "card-title" },
    description: { type: String },
  };

  constructor() {
    super();
    this.type = "";
    this.cardTitle = "";
    this.description = "";
  }

  connectedCallback() {
    super.connectedCallback();
    // Prevent browser tooltip
    this.removeAttribute("title");
  }

  static styles = css`
    :host {
      display: block;
    }

    .feature-card {
      background: var(--bg-tertiary);
      padding: 1.5em;
      border-radius: 8px;
      border: 1px solid var(--border-default);
      border-left: 4px solid var(--card-accent);
      position: relative;
    }

    .feature-card h3 {
      color: var(--card-accent);
      margin-bottom: 0.75em;
      font-size: 1.3em;
      font-weight: inherit;
      display: flex;
      align-items: center;
      gap: 0.75em;
    }

    .feature-card h3::before {
      content: "";
      width: 20px;
      height: 20px;
      background: var(--card-accent);
      border-radius: var(--icon-radius, 2px);
      flex-shrink: 0;
      mask: var(--icon-mask);
      mask-size: contain;
      mask-repeat: no-repeat;
      mask-position: center;
    }

    .feature-card p {
      color: var(--text-secondary);
      line-height: 1.5;
      margin: 0;
    }

    /* Card type styles */
    :host([type="event-store"]) {
      --card-accent: #3b82f6;
      --icon-radius: 2px;
      --icon-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'%3E%3Cpath fill='currentColor' d='M4 4h12v2H4V4zm0 4h12v2H4V8zm0 4h12v2H4v-2zm0 4h12v2H4v-2z'/%3E%3C/svg%3E");
    }

    :host([type="generators"]) {
      --card-accent: #10b981;
      --icon-radius: 50%;
      --icon-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'%3E%3Cpath fill='currentColor' d='M10 3a1 1 0 011 1v5.586l3.707-3.707a1 1 0 111.414 1.414L12.414 11H18a1 1 0 110 2h-5.586l3.707 3.707a1 1 0 11-1.414 1.414L11 14.414V20a1 1 0 11-2 0v-5.586l-3.707 3.707a1 1 0 01-1.414-1.414L7.586 13H2a1 1 0 110-2h5.586L3.879 7.293a1 1 0 011.414-1.414L9 9.586V4a1 1 0 011-1z'/%3E%3C/svg%3E");
    }

    :host([type="actors"]) {
      --card-accent: #f59e0b;
      --icon-radius: 3px;
      --icon-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'%3E%3Cpath fill='currentColor' d='M11.827 2.173a1 1 0 00-1.414 0L8 4.586 6.586 3.172a1 1 0 00-1.414 1.414L6.586 6 4.172 8.414a1 1 0 101.414 1.414L8 7.414l2.414 2.414a1 1 0 001.414-1.414L9.414 6l2.414-2.414a1 1 0 000-1.414z'/%3E%3Cpath fill='currentColor' d='M15.828 12.172a1 1 0 00-1.414 1.414L16 15.172l-1.586 1.586a1 1 0 101.414 1.414L18.242 16l1.586 1.586a1 1 0 001.414-1.414L19.656 14.586l1.586-1.586a1 1 0 00-1.414-1.414L18.242 13.172l-1.586-1.586a1 1 0 00-1.414 0z'/%3E%3C/svg%3E");
    }

    :host([type="commands"]) {
      --card-accent: #8b5cf6;
      --icon-radius: 2px;
      --icon-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'%3E%3Cpath fill='currentColor' d='M2 3a1 1 0 011-1h14a1 1 0 011 1v14a1 1 0 01-1 1H3a1 1 0 01-1-1V3zm2 2v12h12V5H4zm2 2h2v2H6V7zm4 0h4v2h-4V7zm-4 4h8v2H6v-2z'/%3E%3C/svg%3E");
    }

    :host([type="local-first"]) {
      --card-accent: #ef4444;
      --icon-radius: 2px;
      --icon-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'%3E%3Cpath fill='currentColor' d='M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z'/%3E%3C/svg%3E");
    }

    :host([type="nushell"]) {
      --card-accent: #06b6d4;
      --icon-radius: 50%;
      --icon-mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'%3E%3Cpath fill='currentColor' d='M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V4zM3 10a1 1 0 011-1h6a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1v-2zM14 9a1 1 0 00-1 1v2a1 1 0 001 1h2a1 1 0 001-1v-2a1 1 0 00-1-1h-2zM3 16a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1v-2z'/%3E%3C/svg%3E");
    }
  `;

  render() {
    return html`
      <div class="feature-card" title="">
        <h3>${this.cardTitle}</h3>
        <p>${this.description}</p>
      </div>
    `;
  }
}

customElements.define("feature-card", FeatureCard);
