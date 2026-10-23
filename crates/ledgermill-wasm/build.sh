#!/bin/sh
# Builds the npm package into pkg/: the release wasm, its wasm-bindgen glue and package.json.
# Needs the toolchain in rust-toolchain.toml and wasm-bindgen-cli 0.2.129 on PATH.
# Before the first release an app uses it with `cd pkg && bun link`.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
cargo build --release --target wasm32-unknown-unknown -p ledgermill-wasm
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/ledgermill_wasm.wasm
cp crates/ledgermill-wasm/package.json pkg/package.json
