# xs

[![CI](https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg)](https://github.com/cablehead/xs/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/discord/1182364431435436042?logo=discord)](https://discord.com/invite/YNbScHBHrh)

```
Status: WIP  [████............ 20%]
```

An event stream store for personal, local-first use. Kinda like the
[`sqlite3` cli](https://sqlite.org/cli.html), but specializing in the [event
sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) use case.

![screenshot](./docs/screenshot.png)

Built with:

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API
  over a local Unix domain socket for subscriptions, etc.
- [nushell](https://www.nushell.sh): for scripting and [interop](https://utopia.rosano.ca/interoperable-visions/)

## desired features

- event stream: append, cat / tail [reverse], get, last, first, next, previous
- ephemeral events
- as well as the event stream: a k/v store fo cursors and materialized views
- ability to subscribe to updates
    - to both events
    - and materialized views
- should be able to manage processes ala [daemontools](http://cr.yp.to/daemontools.html), [runit](https://smarden.org/runit/), [Pueue](https://github.com/Nukesor/pueue)
    - or: simply runs snippets of [nushell](https://github.com/nushell/nushell.git) on new event
    - the snippets are registered via the event stream
- server facilitates watching for updates + managing processes

## Path Traveled

- [xs-3](https://github.com/cablehead/xs-3): [sled](https://github.com/spacejam/sled) index with [cacache](https://github.com/zkat/cacache-rs) CAS, no concurrency
- [xs-0](https://github.com/cablehead/xs-0) original experiment.
    -[LMDB](http://www.lmdb.tech/doc/) combined index / content store (pre realizing the event primary content should be
  stored in a CAS)
    - Multi-process concurrent, but polling for subscribe
