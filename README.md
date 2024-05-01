older xs:

- original, lmdb index/store (pre realizing the event primary content should be
  stored in a CAS), so concurrent, but polling for subscribe:
  https://github.com/cablehead/xs
- sled index with cacache CAS v1, no concurrency: https://github.com/cablehead/xs-3

# a-new-xs

- sled index + cacache CAS, a server runs over a local unix domain socket to
  provide coordination
- client protcol is hyper: http1.1
- designed to be run as a nushell plugin??
    - need poc: what does it look like calling back into the host `nu`


## notes

- [xs](https://github.com/cablehead/xs): current version is a PoC
- currently it just provides a persistent lightweight, flexible event stream
- current version uses LMDB and server-less polling to pick up new items
- want: switch to sled + a server that listens on a local unix domain socket
- server facilitates watching for updates + managing processes
- want ephemeral events
- put, cat / tail [reverse], get, last, first, next
- want: k/v storage to manage cursors + materialized views + state, ability to subscribe to updates
- xs should be able to manage processes ala [daemontools](http://cr.yp.to/daemontools.html), [runit](https://smarden.org/runit/), [Pueue](https://github.com/Nukesor/pueue)
