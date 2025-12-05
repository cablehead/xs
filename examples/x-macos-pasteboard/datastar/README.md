# a Web UI starter to experiment with viewing clipboard history

This is a [`Datastar`](https://data-star.dev/) UI for `xs` +
`x-macos-pasteboard`.

Requirements:

- minijina-cli
- http-nu
- [x-macos-pasteboard](https://github.com/cablehead/x-macos-pasteboard)
- [xs](https://github.com/cablehead/xs)

## To run

Start `xs`:

```
xs serve ./store --expose :3021
```

Bootstrap the store:

```nushell
# register x-macos-pasteboard as a generator
r#'{ run: {|| x-macos-pasteboard | lines } }'# | .append pb.spawn

# register a handler to map raw clipboard data to content
cat ../handler-pb.map.nu | .append pb.map.register
```
