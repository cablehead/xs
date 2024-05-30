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

- event stream: 
    - [x] append
    - [x] cat
        - [x] last-id
        - [x] follow
    - [ ] tac
        - [ ] last-id
    - [x] get
    - [ ] last
    - [ ] first
    - [ ] next?
    - [ ] previous?
- [x] cas, get
- ephemeral events / content
- as well as the event stream: a k/v store fo cursors and materialized views
- ability to subscribe to updates
    - [x] to both events (`cat --follow`)
    - [ ] and materialized views
- should be able to manage processes ala [daemontools](http://cr.yp.to/daemontools.html), [runit](https://smarden.org/runit/), [Pueue](https://github.com/Nukesor/pueue)
    - or: simply runs snippets of [nushell](https://github.com/nushell/nushell.git) on new event
    - the snippets are registered via the event stream
- server facilitates watching for updates + managing processes
- [ ] builtin http server:
    - [ ] You can optionally serve HTTP requests from your store. Requests are
    written to the event stream as `http.request` and then the connection
    watches the event stream for a `http.response`.
    - [ ] You can register event handlers that subscribe to `http.request`
      events and emit `http.response` events.

## Path Traveled

- [xs-3](https://github.com/cablehead/xs-3): [sled](https://github.com/spacejam/sled) index with [cacache](https://github.com/zkat/cacache-rs) CAS, no concurrency
- [xs-0](https://github.com/cablehead/xs-0) original experiment.
    -[LMDB](http://www.lmdb.tech/doc/) combined index / content store (pre realizing the event primary content should be
  stored in a CAS)
    - Multi-process concurrent, but polling for subscribe
