import { css, html, LitElement } from "https://cdn.skypack.dev/lit";

export class CrossStreamTitle extends LitElement {
  static styles = css`
    :host {
      display: inline;
      color: var(--text-primary);
    }

    .stream {
      color: var(--accent-blue);
    }
  `;

  render() {
    return html`
      cross.<span class="stream">stream</span>
    `;
  }
}

customElements.define("cross-stream-title", CrossStreamTitle);
