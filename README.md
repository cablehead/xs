# xs (cross-stream) [![CI](https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg)](https://github.com/cablehead/xs/actions/workflows/ci.yml) [![Discord](https://img.shields.io/discord/1182364431435436042?logo=discord)](https://discord.com/invite/YNbScHBHrh)

`xs` is an event stream store for personal, local-first use. Think of it like
[`sqlite`](https://sqlite.org/cli.html), but specializing in the
[event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) use case.

Read [here](https://cablehead.github.io/xs/getting-started/installation/) to
[get started](https://cablehead.github.io/xs/getting-started/installation/) or
[join our Discord](https://discord.com/invite/YNbScHBHrh) to ask questions.

## Built with 🙏💚

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [Nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)
