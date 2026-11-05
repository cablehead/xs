#!/bin/bash

set -euo pipefail

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo t

# Check documentation
(cd docs && npx astro check && npm run build)
