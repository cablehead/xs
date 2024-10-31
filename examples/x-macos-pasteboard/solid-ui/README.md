# a Web UI starter to experiment with viewing clipboard history

This is a [`SolidJS`](https://www.solidjs.com) UI for `xs` +
`x-macos-pasteboard`.

<img width="800" alt="image" src="https://github.com/user-attachments/assets/6deac539-8feb-4953-bd2b-ef38a799d8e5">

Requirements:

- [Deno2](https://deno.com)
- [x-macos-pasteboard](https://github.com/cablehead/x-macos-pasteboard)
- [xs](https://github.com/cablehead/xs)

## To run

Start `xs`:

```
xs serve ./store --expose :3021
```

Bootstrap the store:

```bash
# register x-macos-pasteboard as a frame generator
echo "x-macos-pasteboard | lines" | xs append ./store pb.spawn

# register a handler to map raw clipboard data to content
cat handler-pb.map.nu | xs append ./store pb.map.register
```

Start UI:

```
deno task dev
open http://localhost:5173
```

## a base to explore the clipboard for Linux

A motivation for this example is for people to use it as a base to explore the
[clipboard on Linux](https://github.com/cablehead/stacks/issues/50).

Here's how you'd do that. Create a cli similar to `x-macos-pasteboard` that
writes new clipboard entries as jsonl to stdout. The format doesn't matter. Try
and dump as much data as the system will give you.

Replace the bootstrap step with:

```bash
echo "<your-cli> | lines" | xs append ./store pb.spawn
```

That's it! As you copy stuff to the clipboard, you'll see your raw data in the
UI.

You can then start experimenting with mapping the raw data to the `content`
topic. Pick an id of a raw frame and:

```bash
xs get ./store <id> | map | xs append ./store content --meta '{"updates":<id>}'
```

If the target content is an image, include `"content_type":"image"` in the
`--meta` object.

If this is interesting to you, swing by this
[Github issue](https://github.com/cablehead/stacks/issues/50).
