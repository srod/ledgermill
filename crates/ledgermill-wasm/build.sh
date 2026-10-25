#!/bin/sh
# Builds the npm package into pkg/: the release wasm, its wasm-bindgen glue, package.json, the README and the licences.
# Needs the toolchain in rust-toolchain.toml and wasm-bindgen-cli 0.2.129 on PATH.
# Before the first release an app uses it with `cd pkg && bun link`.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

# The npm package and the crates are released together: one version.
cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p;}' Cargo.toml)
npm_version=$(sed -n 's/^ *"version": "\(.*\)",$/\1/p' crates/ledgermill-wasm/package.json)
if [ -z "$cargo_version" ] || [ "$cargo_version" != "$npm_version" ]; then
  echo "build.sh: Cargo workspace version '$cargo_version' differs from package.json version '$npm_version'" >&2
  exit 1
fi

cargo build --release --target wasm32-unknown-unknown -p ledgermill-wasm
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/ledgermill_wasm.wasm
cp crates/ledgermill-wasm/package.json pkg/package.json
cp README.md LICENSE-APACHE LICENSE-MIT pkg/
