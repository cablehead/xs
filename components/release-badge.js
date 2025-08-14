import { css, html, LitElement } from "https://cdn.skypack.dev/lit";

export class ReleaseBadge extends LitElement {
  static properties = {
    version: { type: String },
    releaseDate: { type: String, attribute: "release-date" },
    releaseUrl: { type: String, attribute: "release-url" },
  };

  constructor() {
    super();
    this.version = "v0.5.0";
    // Set to 3 hours ago from current time
    const now = new Date();
    const threeHoursAgo = new Date(now.getTime() - 3 * 60 * 60 * 1000);
    this.releaseDate = threeHoursAgo.toISOString();
    this.releaseUrl = "https://github.com/cablehead/xs/releases/tag/v0.5.0";
  }

  static styles = css`
    :host {
      display: block;
      text-align: center;
    }

    .release-badge {
      display: inline-grid;
      grid-template-columns: auto 1fr auto;
      grid-template-rows: auto auto;
      gap: 0.5em 1em;
      border-radius: 6px;
      padding: 0.75em 1em;
      text-decoration: none;
      font-size: inherit;
      transition: all 0.3s ease;
      align-items: center;
    }

    .release-badge:hover {
      box-shadow:
        0 0 15px rgba(59, 130, 246, 0.5),
        0 0 30px rgba(59, 130, 246, 0.2),
        inset 0 1px 0 rgba(59, 130, 246, 0.3);
      transform: translateY(-1px);
    }

    .tag-icon {
      width: 18px;
      height: 18px;
      display: inline-block;
      background: var(--accent-blue);
      mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath d='M1 7.775V2.75C1 1.784 1.784 1 2.75 1h5.025c.464 0 .91.184 1.238.513l6.25 6.25a1.75 1.75 0 0 1 0 2.474l-5.026 5.026a1.75 1.75 0 0 1-2.474 0l-6.25-6.25A1.75 1.75 0 0 1 1 7.775Zm1.5 0c0 .066.026.13.073.177l6.25 6.25a.25.25 0 0 0 .354 0l5.025-5.025a.25.25 0 0 0 0-.354l-6.25-6.25a.25.25 0 0 0-.177-.073H2.75a.25.25 0 0 0-.25.25ZM6 5a1 1 0 1 1 0 2 1 1 0 0 1 0-2Z'/%3E%3C/svg%3E")
        no-repeat center;
      mask-size: contain;
    }

    .release-text {
      color: var(--text-primary);
      font-weight: 500;
    }

    .latest-badge {
      background: transparent;
      color: var(--accent-blue);
      border: 1px solid var(--accent-blue);
      padding: 0.3em 0.6em;
      border-radius: 16px;
      font-size: 0.8em;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .release-time {
      color: var(--text-muted);
      grid-column: 2;
      grid-row: 2;
      justify-self: start;
    }

    .grid-empty {
      grid-column: 1;
      grid-row: 2;
    }

    .grid-empty:last-child {
      grid-column: 3;
      grid-row: 2;
    }

    .tag-icon {
      grid-column: 1;
      grid-row: 1;
    }

    .release-text {
      grid-column: 2;
      grid-row: 1;
    }

    .latest-badge {
      grid-column: 3;
      grid-row: 1;
    }
  `;

  connectedCallback() {
    super.connectedCallback();
    this.updateTime();
    // Update time every minute
    this.timeInterval = setInterval(() => this.updateTime(), 60000);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this.timeInterval) {
      clearInterval(this.timeInterval);
    }
  }

  updateTime() {
    const releaseDate = new Date(this.releaseDate);
    const now = new Date();
    const diffMs = now - releaseDate;

    const days = Math.floor(diffMs / (1000 * 60 * 60 * 24));
    const hours = Math.floor(diffMs / (1000 * 60 * 60));
    const minutes = Math.floor(diffMs / (1000 * 60));

    let timeText;
    if (days > 0) {
      timeText = days === 1 ? "1 day ago" : `${days} days ago`;
    } else if (hours > 0) {
      timeText = hours === 1 ? "1 hour ago" : `${hours} hours ago`;
    } else if (minutes > 0) {
      timeText = minutes === 1 ? "1 minute ago" : `${minutes} minutes ago`;
    } else {
      timeText = "just now";
    }

    this.timeText = timeText;
    this.requestUpdate();
  }

  render() {
    return html`
      <a href="${this
        .releaseUrl}" class="release-badge" target="_blank" rel="noopener">
        <span class="tag-icon"></span>
        <span class="release-text">Current release ${this.version}</span>
        <span class="latest-badge">Latest</span>
        <span class="grid-empty"></span>
        <span class="release-time">${this.timeText || ""}</span>
        <span class="grid-empty"></span>
      </a>
    `;
  }
}

customElements.define("release-badge", ReleaseBadge);
