#!/bin/sh
# Builds the npm package into pkg/: the release wasm, its wasm-bindgen glue, types, package.json, the README and the licences.
# Needs the toolchain in rust-toolchain.toml and wasm-bindgen-cli matching Cargo.lock on PATH.
# Before the first release an app uses it with `cd pkg && bun link`.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

# The npm package and the crates are released together: one version.
cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p;}' Cargo.toml)
dep_version=$(sed -n 's/^ledgermill = {.*version = "\([^"]*\)".*}$/\1/p' crates/ledgermill-wasm/Cargo.toml)
npm_version=$(sed -n 's/^ *"version": "\([^"]*\)".*$/\1/p' crates/ledgermill-wasm/package.json)
if [ -z "$cargo_version" ] || [ "$cargo_version" != "$npm_version" ]; then
  echo "build.sh: Cargo workspace version '$cargo_version' differs from package.json version '$npm_version'" >&2
  exit 1
fi
if [ "$cargo_version" != "$dep_version" ]; then
  echo "build.sh: Cargo workspace version '$cargo_version' differs from ledgermill-wasm dependency version '$dep_version'" >&2
  exit 1
fi

lock_wbg=$(awk '/^name = "wasm-bindgen"$/{getline; if (sub(/^version = "/, "") && sub(/"$/, "")) { print; exit }}' Cargo.lock)
cli_wbg=$(wasm-bindgen --version | awk '{print $2}')
if [ -n "$lock_wbg" ] && [ "$lock_wbg" != "$cli_wbg" ]; then
  echo "build.sh: wasm-bindgen CLI '$cli_wbg' differs from Cargo.lock '$lock_wbg'" >&2
  exit 1
fi

rm -rf pkg
cargo build --release --target wasm32-unknown-unknown -p ledgermill-wasm --locked
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/ledgermill_wasm.wasm
cp crates/ledgermill-wasm/package.json pkg/package.json
cp crates/ledgermill-wasm/types.d.ts pkg/types.d.ts
cp README.md LICENSE-APACHE LICENSE-MIT pkg/
