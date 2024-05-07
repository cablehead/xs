# xs

- [fjall](https://github.com/fjall-rs/fjall) index + [cacache](https://github.com/zkat/cacache-rs) CAS, a server runs over a local unix domain socket to
  provide coordination
- client protcol is hyper: http1.1

## notes

- server facilitates watching for updates + managing processes
- want ephemeral events
- put, cat / tail [reverse], get, last, first, next
- want: k/v storage to manage cursors + materialized views + state, ability to subscribe to updates
- xs should be able to manage processes ala [daemontools](http://cr.yp.to/daemontools.html), [runit](https://smarden.org/runit/), [Pueue](https://github.com/Nukesor/pueue)

## Path Traveled

- [xs-3](https://github.com/cablehead/xs-3): [sled](https://github.com/spacejam/sled) index with [cacache](https://github.com/zkat/cacache-rs) CAS v1, no concurrency
- [xs-0](https://github.com/cablehead/xs-0) original experiment.
    -[LMDB](http://www.lmdb.tech/doc/) combined index / content store (pre realizing the event primary content should be
  stored in a CAS)
    - Multi-process concurrent, but polling for subscribe
