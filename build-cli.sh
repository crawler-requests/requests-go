#!/usr/bin/env bash
# =============================================================================
# requests-go CLI 跨平台编译脚本
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/dist"

declare -A TARGETS=(
    ["linux/amd64"]="requests-go-linux-amd64"
    ["linux/arm64"]="requests-go-linux-arm64"
    ["linux/386"]="requests-go-linux-386"
    ["windows/amd64"]="requests-go-windows-amd64.exe"
    ["windows/386"]="requests-go-windows-386.exe"
    ["darwin/amd64"]="requests-go-darwin-amd64"
    ["darwin/arm64"]="requests-go-darwin-arm64"
)

mkdir -p "$OUTPUT_DIR"

echo "Building requests-go CLI for all platforms..."
echo ""

for target in "${!TARGETS[@]}"; do
    goos="${target%/*}"
    goarch="${target#*/}"
    output="$OUTPUT_DIR/${TARGETS[$target]}"

    printf "  %-20s → %s ... " "$target" "$(basename "$output")"
    if GOOS="$goos" GOARCH="$goarch" go build -ldflags="-s -w" -o "$output" "$SCRIPT_DIR/cmd/requests-go/" 2>/dev/null; then
        size=$(ls -lh "$output" | awk '{print $5}')
        echo "OK ($size)"
    else
        echo "SKIPPED (needs CGO/cross-compiler)"
    fi
done

echo ""
echo "Done! Binaries in $OUTPUT_DIR/"
ls -lh "$OUTPUT_DIR/" 2>/dev/null || echo "(no binaries built)"
