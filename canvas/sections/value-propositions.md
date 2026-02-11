## **Value Propositions**

### Emotional Pain Points

- _Frustration with modern stacks_: "Every time I want to build something fun, I
  end up buried in incomprehensible JavaScript tracebacks and brittle Next.js
  configs."
- _Loss of creative flow_: "I can't just make a change and see it work anymore —
  there's always a delay, a rebuild, an error to chase down."
- _Disconnection from the system_: "I don't understand my tools deeply enough to
  feel confident changing them."
- _Lack of ownership_: "My creations feel trapped in frameworks and services I
  don't control."

---

### **2-Minute "Aha" Example** — _Capture a URL, Get a Screenshot Back in Your Stream_

1. **Start**: The customer opens a terminal connected to their personal stream.
2. **Capture**: They run a single command to append a URL to the stream.

   ```bash
   "https://example.com" | .append url
   ```
3. **Automation**: A small, visible actor they can read and understand picks
   up the event, launches a screenshot service, and saves the image to the
   stream.
4. **Feedback**: Within seconds, the stream updates in their terminal — now
   showing the original URL and the screenshot as a new event.
5. **Emotional payoff**:

   - _Instant gratification_: "I did that in two minutes."
   - _Understanding_: "I can see exactly which actor did the work and how."
   - _Control_: "I can swap the screenshot service, change the output format, or
     chain another step without touching anything else."

---

### Other Proposed "Aha" Use Cases (High Level)

- **Clipboard Watcher**: Anything copied to the clipboard automatically appears
  in the stream, optionally transformed or tagged for later use.
- **Discord Bot Listener**: Messages in a specific channel become stream events
  that can trigger automations, like summarization or archiving.
- **GitHub Webhook Trigger**: Local scripts fire in response to repo events via
  smee.io — perfect for dev workflows without cloud CI/CD complexity.
- **MCP Client Generators**: Run a local MCP server as a generator to extend
  capabilities dynamically, without changing core code.
- **Publishing Stream as a Digital Garden**: Append raw thoughts to the stream;
  expand them over time; render the evolving state into a public site with
  minijina + http-nu.
