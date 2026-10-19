#!/bin/sh
# Build the actual Rust runtime. Assets are packaged separately, never checked in.
set -eu
cd "$(dirname "$0")/../.."
: "${1:?usage: build_wasm.sh OUTPUT_DIRECTORY}"
mkdir -p "$1"
OUT=$(cd "$1" && pwd)
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/debug/opensource.wasm --target web --out-dir "$OUT"
cp tools/web/wasm.html "$OUT/index.html"
printf 'Built Rust WASM into %s; add a private game.zip from browser_bundle.\n' "$OUT"
