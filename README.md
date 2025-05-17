# xs (cross-stream) [![CI](https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg)](https://github.com/cablehead/xs/actions/workflows/ci.yml) [![Discord](https://img.shields.io/discord/1182364431435436042?logo=discord)](https://discord.com/invite/YNbScHBHrh)

`xs` is a local-first event stream store for personal projects.
Think of it like [`sqlite`](https://sqlite.org/cli.html) but specializing in the
[event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) use case.

See the [documentation](https://cablehead.github.io/xs/) for detailed
installation instructions, tutorials and examples.

<img src="https://github.com/user-attachments/assets/12c9cce5-44ab-4a64-ab1c-d83bf6c28cad" style="height:400px;" alt="Pixel art heroes cross proton streams, saving gritty, shadowy Toronto street beneath glowing CN Tower backdrop.">

## Quick start

```sh
# install
brew install cablehead/tap/cross-stream
# or:
cargo install cross-stream --locked

# start a server
xs serve ./store

# in another window
echo "hello" | xs append ./store notes
xs cat ./store
```

## Features

- Local-first append-only store
- Content-addressable storage for large payloads
- Real-time subscriptions to new events
- Generators and handlers for background processing

## Connect

Join our [Discord](https://discord.com/invite/YNbScHBHrh) to ask questions or share ideas.

## Built with 🙏💚

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [Nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)
