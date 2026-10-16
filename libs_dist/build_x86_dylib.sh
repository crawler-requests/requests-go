#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CORE_DIR="$ROOT_DIR/rust/requests-core"

echo "Building requests-darwin-amd64.dylib ..."
cd "$CORE_DIR"
cargo build --release --target x86_64-apple-darwin

if [ -f "target/x86_64-apple-darwin/release/librequests_core.dylib" ]; then
    cp "target/x86_64-apple-darwin/release/librequests_core.dylib" "$SCRIPT_DIR/requests-darwin-amd64.dylib"
    echo "Done: requests-darwin-amd64.dylib"
else
    echo "Failed: dylib not found"
fi
