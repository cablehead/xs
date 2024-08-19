#!/bin/bash

set -euo pipefail

cargo fmt --check
cargo clippy
cargo t
