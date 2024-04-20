
older xs:

- sled index with cacache CAS v1, no concurrency: https://github.com/cablehead/xs-3
- original, lmdb, so concurrent, but polling for subscribe: https://github.com/cablehead/xs

# a-new-xs

- sled index + cacache CAS, a server runs over a local unix domain socket to
  provide coordination
- client protcol is hyper: http1.1

- designed to be run as a nushell plugin??
- need poc
