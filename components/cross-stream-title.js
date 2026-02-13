import { css, html, LitElement } from "https://esm.sh/lit";

export class CrossStreamTitle extends LitElement {
  static styles = css`
    :host {
      display: inline;
      color: var(--text-primary);
    }

    .stream {
      background: linear-gradient(
        45deg,
        var(--accent-blue) 0%,
        var(--accent-blue-light) 30%,
        var(--accent-blue-lighter) 60%,
        var(--accent-blue-light) 80%,
        var(--accent-blue) 100%
      );
      background-size: 200% 200%;
      animation: plasma-flow 3s ease-in-out infinite;
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      text-shadow:
        0 0 10px var(--accent-blue),
        0 0 20px var(--accent-blue),
        0 0 30px var(--accent-blue);
      }

      @keyframes plasma-flow {
        0%, 100% {
          background-position: 0% 50%;
        }
        50% {
          background-position: 100% 50%;
        }
      }
    `;

    render() {
      return html`
        cross.<span class="stream">stream</span>
      `;
    }
  }

  customElements.define("cross-stream-title", CrossStreamTitle);
