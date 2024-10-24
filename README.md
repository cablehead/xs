# xs (cross-stream) [![CI](https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg)](https://github.com/cablehead/xs/actions/workflows/ci.yml)

[![Discord](https://img.shields.io/discord/1182364431435436042?logo=discord)](https://discord.com/invite/YNbScHBHrh)
-- Come hang out and play

`xs` is an event stream store designed for personal, local-first use. Think of
it like the [`sqlite3` cli](https://sqlite.org/cli.html), but crafted
specifically for
[event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html).

The focus is on fun and playfulness. Event sourcing provides an
[immediate connection to what you're creating](https://youtu.be/a-OyoVcbwWE?si=kfuJ0KkSGlN21GBL&t=121),
making the process feel alive. `xs` encourages experimentation, allowing you to
make messes and explore freely—then gives you the means to organize and make
sense of it all.

> "You don't so much run it, as poke _at_ it."

![overview](./docs/overview.png)

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
[Nushell](https://www.nushell.sh), but since many are more familiar with `bash`,
here are the very basics that work just fine from `bash`.

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
% echo "content" | xs append ./store my-topic --meta '{"type": "text/plain"}'
{"topic":"my-topic","id":"03clswrgmmkkoqnotna38ldvl","hash":"sha256-Q0copBCnj1b8G1iZw1k0NuYasMcx6QctleltspAgXlM=","meta":{"type":"text/plain"},"ttl":"forever"}
```

To fetch the contents of the stream, use the `cat` command:

```bash
% xs cat ./store/
{"topic":"xs.start","id":"03clswlaih9x17izyzqy5jg7n","hash":null,"meta":{"expose":null},"ttl":null}
{"topic":"my-topic","id":"03clswrgmmkkoqnotna38ldvl","hash":"sha256-Q0copBCnj1b8G1iZw1k0NuYasMcx6QctleltspAgXlM=","meta":{"type":"text/plain"},"ttl":"forever"}
```

`xs` generates a few meta events, such as `xs.start`, which is emitted whenever
the process managing the store starts.

You can also see the `my-topic` event we just appended, along with a `hash`,
which represents the hash of the stored content. You can retrieve this content
from the Content-Addressable Storage (CAS) using:

```bash
% xs cas ./store/ sha256-Q0copBCnj1b8G1iZw1k0NuYasMcx6QctleltspAgXlM=
content
```

## Built with 🙏💚

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [Nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)
