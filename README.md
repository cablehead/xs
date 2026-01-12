<!-- LOGO -->
<h1>
<p align="center">
  <a href="https://cablehead.github.io/xs/">
    <img src="https://github.com/user-attachments/assets/f0c019ad-885d-4837-b72b-ef6ff1f85c0f" alt="Logo">
  </a>
  <br><br>
  cross.stream
</h1>
  <p align="center">
    Local-first event streaming for building reactive workflows and automation.
    <br />
    <a href="#about">About</a>
    ·
    <a href="https://cablehead.github.io/xs/">Documentation</a>
    ·
    <a href="https://discord.com/invite/YNbScHBHrh">Connect</a>
    ·
    <a href="#built-with-">Built with</a>
  </p>
</p>

<p align="center">
  <a href="https://github.com/cablehead/xs/actions/workflows/ci.yml">
    <img src="https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://discord.com/invite/YNbScHBHrh">
    <img src="https://img.shields.io/discord/1182364431435436042?logo=discord" alt="Discord">
  </a>
  <a href="https://crates.io/crates/cross-stream">
    <img src="https://img.shields.io/crates/v/cross-stream.svg" alt="Crates">
  </a>
  <a href="https://docs.rs/cross-stream">
    <img src="https://docs.rs/cross-stream/badge.svg" alt="Docs.rs">
  </a>
</p>

## What's an event streaming store?

If you think of an "event" like a frame in a movie—a small package on a
timeline—an event streaming store is a database designed to record these frames
in strict order, append-only, so they can be replayed or reacted to later.

For example, you might append a frame every time a message is
[posted in a specific Discord channel](examples/discord-bot). You can then
[`.cat`](https://cablehead.github.io/xs/reference/xs-nu/#cat) the stream to
review all captured messages, and—if you're in a
[Nushell](https://www.nushell.sh) session—use pipelines to filter, aggregate, or
process them with a CLI tool.

## Quick Start

See the
[installation guide](https://cablehead.github.io/xs/getting-started/installation/)
to get started.

## Features

- **Reactive Workflows**: Build handlers that automatically respond to events as
  they flow through the stream
- **CLI Integration**: Turn any command-line tool into a streaming component
  with generators
- **Nushell Native**: First-class integration with Nushell for powerful data
  processing pipelines
- **Real-time Streaming**: Subscribe to live event feeds and build responsive
  applications
- **Content Addressable**: Efficient storage and deduplication of large payloads
- **Local-first**: Your data stays on your machine, no cloud dependencies
  required

## Naming Conventions

xs follows industry best practices for naming consistency across CLI, API, and
internal components. See the
[Naming Schema Documentation](docs/naming-schema/NAMING_SCHEMA.md) for a
comprehensive guide to terminology including:

- **Temporal Navigation**: `head` (most recent), `from-latest`, `from-id`
- **Stream Operations**: `append`, `cat`, `follow`, `remove`
- **Data Structures**: frames, streams, topics, contexts
- **CLI/API Consistency**: How commands map across layers

For migrating from deprecated flags like `--tail` and `--last-id`, see the
[Quick Reference](docs/naming-schema/NAMING_QUICK_REFERENCE.md).

## Connect

Join our [Discord](https://discord.com/invite/YNbScHBHrh) to ask questions or
share ideas.

## Built with 🙏💚

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [Nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)
- [iroh](https://www.iroh.computer): QUIC-based P2P transport for exposing and
  accessing streams
