# n0-watcher

> Watchable values.

A `Watchable` exists to keep track of a value which may change over time.  It allows
observers to be notified of changes to the value.  The aim is to always be aware of the
**last** value, not to observe *every* value change.

In that way, a `Watchable` is like a `tokio::sync::broadcast::Sender`, except that there's no risk
of the channel filling up, but instead you might miss items.

See [the module documentation][https://docs.rs/n0-watcher] for more information.


## License

Copyright 2025 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
