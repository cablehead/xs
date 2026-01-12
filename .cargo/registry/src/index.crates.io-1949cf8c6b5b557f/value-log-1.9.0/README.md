<p align="center">
  <img src="/logo.png" height="160">
</p>
<p align="center>
  (temporary logo)
</p>

[![CI](https://github.com/fjall-rs/value-log/actions/workflows/test.yml/badge.svg)](https://github.com/fjall-rs/value-log/actions/workflows/test.yml)
[![docs.rs](https://img.shields.io/docsrs/value-log?color=green)](https://docs.rs/value-log)
[![Crates.io](https://img.shields.io/crates/v/value-log?color=blue)](https://crates.io/crates/value-log)
![MSRV](https://img.shields.io/badge/MSRV-1.74.0-blue)
[![dependency status](https://deps.rs/repo/github/fjall-rs/value-log/status.svg)](https://deps.rs/repo/github/fjall-rs/value-log)

Generic value log implementation for key-value separated storage, inspired by RocksDB's BlobDB [[1]](#footnotes) and Titan [[2]](#footnotes) and implemented in safe, stable Rust.

> [!NOTE]
> This crate is intended as a building block for key-value separated storage.
> You probably want to use https://github.com/fjall-rs/fjall instead.

## Features

- Thread-safe API
- 100% safe & stable Rust
- Supports generic KV-index structures (LSM-tree, ...)
- Generic per-blob compression (optional)
- In-memory blob cache for hot data (can be shared between multiple value logs to cap memory usage)
- On-line garbage collection

Keys are limited to 65536 bytes, values are limited to 2^32 bytes.

## Feature flags

### serde

Enables `serde` derives.

*Disabled by default.*

### bytes

Uses [`bytes`](https://github.com/tokio-rs/bytes) as the underlying `Slice` type.

*Disabled by default.*

## Stable disk format

The disk format is stable as of 1.0.0.

## License

All source code is licensed under MIT OR Apache-2.0.

All contributions are to be licensed as MIT OR Apache-2.0.

## Footnotes

[1] https://github.com/facebook/rocksdb/wiki/BlobDB

[2] https://docs.pingcap.com/tidb/stable/titan-overview/

