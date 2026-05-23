#!/bin/bash
set -euo pipefail

targets=(
    x86_64-unknown-linux-gnu
    aarch64-unknown-linux-gnu
    x86_64-apple-darwin
    aarch64-apple-darwin
    x86_64-pc-windows-gnu
)

dist="$(dirname "$0")/dist"
mkdir -p "$dist"

for t in "${targets[@]}"; do
    echo "=== Building $t ==="
    cargo zigbuild --release --target "$t"

    ext=""
    [[ $t == *windows* ]] && ext=".exe"
    cp "target/$t/release/atuin-export$ext" "$dist/atuin-export-$t$ext"
done

echo "=== Done ==="
