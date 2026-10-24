#!/bin/sh
# Build the WebAssembly version into web-dist/ and package it as web-dist.zip.
# Requires: rustup target add wasm32-unknown-unknown, and `zip`.
set -e; cd "$(dirname "$0")"
cargo build --release -p kindling --target wasm32-unknown-unknown
R=$(ls -d ~/.cargo/registry/src/*/ | head -1)
rm -rf web-dist; mkdir -p web-dist
cp target/wasm32-unknown-unknown/release/kindling.wasm web-dist/
cp "$R"/miniquad-0.4.*/js/gl.js web-dist/gl.js
cp "$R"/sapp-jsutils-0.1.*/js/sapp_jsutils.js web-dist/
cp "$R"/quad-url-0.1.*/js/quad-url.js web-dist/
cp web/index.html web/civ_io.js web-dist/
rm -f web-dist.zip; zip -rq web-dist.zip web-dist
