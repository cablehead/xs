## generators

TLDR:

You can register a snippet of Nushell script by posting an event in the form:

```
<topic>.spawn
```

The snippet should provide "framing", e.g. "tail -F | lines"

Each frame emitted by the snippet will be appended to the event stream with the topic `<topic>.recv`.

More soon...
