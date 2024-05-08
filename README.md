# xs

- [fjall](https://github.com/fjall-rs/fjall) index + [cacache](https://github.com/zkat/cacache-rs) CAS, a server runs over a local unix domain socket to
  provide coordination
- client protocol is [hyper](https://hyper.rs/guides/1/server/echo/): http1.1 only

## notes

- put, cat / tail [reverse], get, last, first, next, previous
- want ephemeral events
- as well as the event stream: a basic k/v store to manage cursors + materialized views
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
