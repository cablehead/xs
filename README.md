# xs (cross-stream) [![CI](https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg)](https://github.com/cablehead/xs/actions/workflows/ci.yml)

`xs` is an event stream store for personal, local-first use. Think of it like
[`sqlite`](https://sqlite.org/cli.html), but specializing in the
[event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) use case.

The focus is on fun and playfulness. Event sourcing provides an
[immediate connection to what you're creating](https://youtu.be/a-OyoVcbwWE?si=kfuJ0KkSGlN21GBL&t=121),
making the process feel alive. `xs` encourages experimentation, allowing you to
make messes and explore freely—then gives you tools to organize and make sense
of it all.

![overview](./docs/overview.png)

> "You don't so much run it, as poke _at_ it."

[![Discord](https://img.shields.io/discord/1182364431435436042?logo=discord)](https://discord.com/invite/YNbScHBHrh)
Come hang out and play

## Installation

You can install the tool with:

```sh
cargo install cross-stream
```

or

```sh
brew install cablehead/tap/cross-stream
```

## Usage

```sh
Usage: xs <COMMAND>

Commands:
  serve   Provides an API to interact with a local store
  cat     `cat` the event stream
  append  Append an event to the stream
  cas     Retrieve content from Content-Addressable Storage
  remove  Remove an item from the stream
  head    Get the head frame for a topic
  get     Get a frame by ID
  help    Print this message or the help of the given subcommand(s)
```

Unlike `sqlite`, which operates directly on the file system, xs requires a
running process to manage access to the local store. This enables features like
subscribing to real-time updates from the event stream.

```bash
% xs serve ./store
11:27:54.464 9zalp xs.start
```

### Basics

**Note:** `xs` is designed to be orchestrated with
[`Nushell`](https://www.nushell.sh), but since many are more familiar with
`bash`, here are the very basics that work just fine from `bash`.

To append items to the stream, use:

```bash
% xs append ./store <topic>
```

The content for the event can be provided via stdin and, if present, will be
stored in Content-Addressable Storage (CAS). You can also append events without
content. Additionally, you can attach arbitrary metadata to an event using the
`--meta` flag, which accepts metadata in JSON format.

For example:

```bash
% echo "content" | xs append ./store my-topic --meta '{"type": "text/plain"}' | jq
{
  "topic": "my-topic",
  "id": "03cq29mdmmkfze8p1plry4maj",
  "hash": "sha256-7XACtDnprIRfIjV9giusFERzD722AW0+yUMil7nsn3M=",
  "meta": {
    "type": "text/plain"
  },
  "ttl": "forever"
}
```

To fetch the contents of the stream, use the `cat` command:

```bash
% xs cat ./store/ | jq
{
  "topic": "xs.start",
  "id": "03cq29gqsg8ijbkob4krv93k3",
  "hash": null,
  "meta": {
    "expose": null
  },
  "ttl": null
}
{
  "topic": "my-topic",
  "id": "03cq29mdmmkfze8p1plry4maj",
  "hash": "sha256-7XACtDnprIRfIjV9giusFERzD722AW0+yUMil7nsn3M=",
  "meta": {
    "type": "text/plain"
  },
  "ttl": "forever"
}
```

`xs` generates a few meta events, such as `xs.start`, which is emitted whenever
the process managing the store starts.

You can also see the `my-topic` event we just appended, along with a `hash`,
which represents the hash of the stored content.

You can retrieve this content from the Content-Addressable Storage (CAS) using:

```bash
% xs cas ./store/ sha256-7XACtDnprIRfIjV9giusFERzD722AW0+yUMil7nsn3M=
content
```

To append another event to `my-topic`, you can run:

```bash
% echo "more content" | xs append ./store my-topic --meta '{"type": "text/plain"}' | jq
{
  "topic": "my-topic",
  "id": "03cq29ul7bhxrcaeh2ssrvcw1",
  "hash": "sha256-LCMWc3yTE5Vt/ACD2joqYs4ln2ZITz4mRA8NGwLdQSg=",
  "meta": {
    "type": "text/plain"
  },
  "ttl": "forever"
}
```

Now, to quickly access the most recent event associated with `my-topic`, you can
use the `head` command:

```bash
% xs head ./store/ my-topic | jq
{
  "topic": "my-topic",
  "id": "03cq29ul7bhxrcaeh2ssrvcw1",
  "hash": "sha256-LCMWc3yTE5Vt/ACD2joqYs4ln2ZITz4mRA8NGwLdQSg=",
  "meta": {
    "type": "text/plain"
  },
  "ttl": "forever"
}
```

The `head` command retrieves the latest event (or "head") for a specific topic.
If you have multiple events under the same topic, `head` will always return the
latest one.

To get the content of the latest version:

```bash
% xs head ./store/ my-topic | jq -r .hash | xargs xs cas ./store/
more content
```

To retrieve a specific event by its ID, use the `get` command.

For example, to get the event with ID `03clswrgmmkkoqnotna38ldvl`:

```bash
% xs get ./store/ 03clswrgmmkkoqnotna38ldvl | jq
{
  "topic": "my-topic",
  "id": "03cq29ul7bhxrcaeh2ssrvcw1",
  "hash": "sha256-LCMWc3yTE5Vt/ACD2joqYs4ln2ZITz4mRA8NGwLdQSg=",
  "meta": {
    "type": "text/plain"
  },
  "ttl": "forever"
}
```

### The basics with [`Nushell`](https://www.nushell.sh)

Here's how the previous basics example looks using Nushell. To get started, run
the following module import:

```nushell
$ use xs.nu *
```

This will add some `.command` conveniences to your session. The commands default
to working with a `./store` in your current directory. You can customize this by
setting `$env.XSPWD`.

Appending looks like this:

```nushell
$ "content" | .append my-topic --meta {type: "text/plain"}
───────┬─────────────────────────────────────────────────────
 topic │ my-topic
 id    │ 03cq29mdmmkfze8p1plry4maj
 hash  │ sha256-7XACtDnprIRfIjV9giusFERzD722AW0+yUMil7nsn3M=
       │ ──────┬────────────
 meta  │  type │ text/plain
       │ ──────┴────────────
 ttl   │ forever
───────┴─────────────────────────────────────────────────────
```

To `.cat` the stream:

```nushell
$ .cat
─#─┬──topic───┬────────────id─────────────┬────────────────────────hash─────────────────────────┬────────meta─────────┬───ttl───
 0 │ xs.start │ 03cq29gqsg8ijbkob4krv93k3 │                                                     │ ────────┬──         │
   │          │                           │                                                     │  expose │           │
   │          │                           │                                                     │ ────────┴──         │
 1 │ my-topic │ 03cq29mdmmkfze8p1plry4maj │ sha256-7XACtDnprIRfIjV9giusFERzD722AW0+yUMil7nsn3M= │ ──────┬──────────── │ forever
   │          │                           │                                                     │  type │ text/plain  │
   │          │                           │                                                     │ ──────┴──────────── │
───┴──────────┴───────────────────────────┴─────────────────────────────────────────────────────┴─────────────────────┴─────────
```

We have the full expressiveness of Nushell—for example, we can get the content
hash of the last frame on the stream using:

```nushell
$ .cat | last | $in.hash
sha256-7XACtDnprIRfIjV9giusFERzD722AW0+yUMil7nsn3M=
```

And then use the `.cas` command to retrieve the content:

```nushell
$ .cat | last | .cas $in.hash
content
```

We can also retrieve the content from a frame by piping it directly to `.cas`:

```nushell
$ .cat | last | .cas
content
```

Continuing the basic example, we append an additional `my-topic` frame:

```nushell
$ "more content" | .append my-topic --meta {type: "text/plain"}
───────┬─────────────────────────────────────────────────────
 topic │ my-topic
 id    │ 03cq29ul7bhxrcaeh2ssrvcw1
 hash  │ sha256-LCMWc3yTE5Vt/ACD2joqYs4ln2ZITz4mRA8NGwLdQSg=
       │ ──────┬────────────
 meta  │  type │ text/plain
       │ ──────┴────────────
 ttl   │ forever
───────┴─────────────────────────────────────────────────────
```

And use `.head` to retrieve the latest version:

```nushell
$ .head my-topic
───────┬─────────────────────────────────────────────────────
 topic │ my-topic
 id    │ 03cq29ul7bhxrcaeh2ssrvcw1
 hash  │ sha256-LCMWc3yTE5Vt/ACD2joqYs4ln2ZITz4mRA8NGwLdQSg=
       │ ──────┬────────────
 meta  │  type │ text/plain
       │ ──────┴────────────
 ttl   │ forever
───────┴─────────────────────────────────────────────────────
```

To get the content of the latest version:

```nushell
$ .head my-topic | .cas
more content
```

Finally, we have the `.get` command:

```nushell
$ .get 03cq29ul7bhxrcaeh2ssrvcw1
───────┬─────────────────────────────────────────────────────
 topic │ my-topic
 id    │ 03cq29ul7bhxrcaeh2ssrvcw1
 hash  │ sha256-LCMWc3yTE5Vt/ACD2joqYs4ln2ZITz4mRA8NGwLdQSg=
       │ ──────┬────────────
 meta  │  type │ text/plain
       │ ──────┴────────────
 ttl   │ forever
───────┴─────────────────────────────────────────────────────
```

## Built with 🙏💚

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [Nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)
