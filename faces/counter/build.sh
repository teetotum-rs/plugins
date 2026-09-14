#!/bin/sh
# Builds the face, signs it and checks it as the firmware will: counter.wasm.
#
# Needs `rustup target add wasm32v1-none` and `cargo install teetotum-pack --features cli`.
# The key is `$TEETOTUM_KEY`, else `~/.config/teetotum/face-key.pem`, made on first use.
set -eu
cd "$(dirname "$0")"
cargo build --release
cp target/wasm32v1-none/release/counter.wasm counter.wasm
teetotum-pack sign counter.wasm
teetotum-pack check counter.wasm
