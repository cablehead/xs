Changelog
=========

## [3.6.0](https://github.com/wezm/titlecase/releases/tag/v3.6.0)

- Support hypendated words [#37](https://github.com/wezm/titlecase/pull/37).
  Thanks @carlocorradini

## [3.5.0](https://github.com/wezm/titlecase/releases/tag/v3.5.0)

- Preserve uppercase text in brackets, such as acronyms
  [#35](https://github.com/wezm/titlecase/pull/35).
  Thanks @carlocorradini

## [3.4.0](https://github.com/wezm/titlecase/releases/tag/v3.4.0)

- Add `Titlecase` trait and implementation to allow calling `.titlecase()` on
  `AsRef<str>` types [#31](https://github.com/wezm/titlecase/pull/31). Thanks
  @carlocorradini

## [3.3.0](https://github.com/wezm/titlecase/releases/tag/v3.3.0)

- Introduce `wasm` cargo feature to enable wasm functionality
  to address [#26](https://github.com/wezm/titlecase/issues/26).

## [3.2.0](https://github.com/wezm/titlecase/releases/tag/v3.2.0)

- Introduce `perf` cargo feature tied to the feature of the same
  name in the regex crate.
  - This allows building for wasm with this feature disabled,
    producing a smaller wasm module.

## [3.1.1](https://github.com/wezm/titlecase/releases/tag/v3.1.1)

- Tweak Cargo metadata to make crates.io accept it

## [3.1.0](https://github.com/wezm/titlecase/releases/tag/v3.1.0)

- Add wasm build [#23](https://github.com/wezm/titlecase/pull/23).

## [3.0.0](https://github.com/wezm/titlecase/releases/tag/v3.0.0)

- Update regex dependency [#20](https://github.com/wezm/titlecase/pull/20).
  Slim down `regex` crate default features.
- Remove joinery dependency [#19](https://github.com/wezm/titlecase/pull/19)
- Use OnceLock instead of LazyStatic [#18](https://github.com/wezm/titlecase/pull/18)
- Minimum Supported Rust Version is now 1.70.0 [#17](https://github.com/wezm/titlecase/pull/17)

## [2.2.0](https://github.com/wezm/titlecase/releases/tag/v2.2.0)

- Further reduce allocations and optimise regex use

## [2.1.0](https://github.com/wezm/titlecase/releases/tag/v2.1.0)

- Lowercase small words that are uppercase #7
- Clean up and reduce intermediate allocations #8

## [2.0.0](https://github.com/wezm/titlecase/releases/tag/v2.0.0)

- Update dependencies
- Minimum Supported Rust Version is now 1.40.0

## [1.1.0](https://github.com/wezm/titlecase/releases/tag/v1.1.0)

- Update dependencies
- Add help and version flags to CLI

## [0.10.0](https://github.com/wezm/titlecase/releases/tag/v0.10.0)

- Improve documentation
- Make use of regular expressions more efficient
- Errors encountered by the titlecase tool are now written to stderr

## [0.9.2](https://github.com/wezm/titlecase/releases/tag/v0.9.2)

Fix typos in Cargo.toml

## [0.9.1](https://github.com/wezm/titlecase/releases/tag/0.9.1)

Initial release
