[`x-macos-pasteboard`](https://github.com/cablehead/x-macos-pasteboard) is a
micro-cli that watches your macOS pasteboard and emits the raw contents to
stdout as jsonl.

To install:

```sh
brew install cablehead/tap/x-macos-pasteboard
```

You can use it as a [generator](../../docs/generators.md) for `xs` to append the
contents of your pasteboard to an event stream.

```nushell
"x-macos-pasteboard | lines" | .append pb.spawn
```

You can then subscribe to new pasteboard events with:

```nushell
.cat -f | where topic == "pb.recv" | each { .cas | from json }
```

Note this is the _raw_ pasteboard data. For the most common case of copying text, you can get the text with:

```nushell
.cat | where topic  == "pb.recv" | each {|x|
    $x | .cas | from json | get types."public.utf8-plain-text"? | if ($in | is-not-empty) {
        decode base64 }}
```

Coming soon(tm): notes on working with the variety of data different macOS apps put on the pasteboard.
