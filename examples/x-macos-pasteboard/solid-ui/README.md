# a Web UI starter to experiment with viewing clipboard history

Vite + Deno + Solid + TypeScript

Requirements:

- [Deno2](https://deno.com)

## To run

Start `xs`:

```
xs serve ./store --expose :3021
```

Bootstrap the store:

```nushell
use xs.nu *

"x-macos-pasteboard | lines" | .append pb.spawn
open handler-content.nu | .append content.register
```


Start UI:

```
$ deno task dev
$ open http://localhost:5173
```
