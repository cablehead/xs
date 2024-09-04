#!/bin/bash

set -euo pipefail

cargo fmt --check
cargo clippy -- -D warnings
cargo t
