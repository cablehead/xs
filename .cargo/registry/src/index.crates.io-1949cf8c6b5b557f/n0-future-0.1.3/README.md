# [n0-future](https://m.youtube.com/watch?v=enGlMJZOi2I&t=147s)

[number 0]'s way of doing async rust.

This crate is supposed to fulfill two purposes:
1. Make it easier to grab one library that re-exposes some sane future/stream combinators that don't use too much unsafe code and seem safe without requiring you to install lots of small libraries.
2. Make it easier to write asnyc code that is Wasm-compatible.

## About safer `future`-related code

We re-expose `futures-lite`, `futures-buffered` and `futures-util` (but mostly for `Sink` and its combinators).
If you're wondering why we're not re-exposing/using X Y or Z, please first read our article about some of our challenges with async rust: https://www.iroh.computer/blog/async-rust-challenges-in-iroh

## About easier Wasm-compatible code

Writing code that works in the `wasm*-*-unknown` targets is not easy:
- `std::time::Instant::now()` panics on use
- You can't spawn threads
- If you use `wasm-bindgen` (practically your only option), structs like `JsValue` are `!Send`.

We aim to solve these issues by providing similar-looking APIs that are easy to `#[cfg(...)]` between Wasm and non-wasm targets, ideally not requiring any cfg at all, but instead the cfg-ing is limited to happen inside this library only.

We do this in a couple of ways:
- `n0_future::time` re-exports `tokio::time::Instant` and friends natively, but `web_time::Instant` and friends in Wasm.
- `n0_future::task` re-exports `tokio` with its `spawn`, `JoinHandle`, `JoinSet`, `Sleep`, `Timeout`, `Interval`, etc. utilities, but in Wasm re-exports a very similar API that's based on `wasm-bindgen-futures`.
- Generally, re-exports natively are `Send`, while re-exports in browsers are `!Send`. There's quickly a need for utilities such as `n0_future::boxed` which re-exports `Box<dyn Future + Send>` natively, but just `Box<dyn Future>` in Wasm (and the same for `Stream`).

## Scope

It's entirely possible that we'll expand the scope of this library, that currently is mostly a re-exports crate to a crate that provides our own flavor of async APIs that we deem are safer to use, we write about some of these ideas in this issue: https://github.com/n0-computer/iroh/issues/2979

## Note to Maintainers: Creating a release

- Make sure to have `git-cliff`, `cargo-release` and `cargo-semver-checks` installed.
- Figure out whether this release is major/minor/patch by running `cargo semver-checks check-release --release-type=major/minor/patch` and see which one fits
- Run `cargo release major/minor/patch` to check if the release would go through well.
- Run `cargo release major/minor/patch --execute` to run the release

## License

Copyright 2024 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

[number 0]: https://n0.computer
