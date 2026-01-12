#!/usr/bin/env bash
set -e

# target platforms
TARGETS=(
    x86_64-unknown-linux-gnu
    aarch64-unknown-linux-gnu
    x86_64-unknown-freebsd
    aarch64-linux-android
    x86_64-linux-android
)

# cross build
for target in "${TARGETS[@]}"; do
    echo "==> Building for $target..."
    if cross build --target "$target"; then
        echo "✅ Success: $target"
    else
        echo "❌ Failed: $target"
        exit 1
    fi
done

echo ""
echo "✅ All builds succeeded!"
