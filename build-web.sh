#!/bin/sh
# Build the WebAssembly version into web-dist/ and package it as web-dist.zip.
# Requires: rustup target add wasm32-unknown-unknown, and `zip`.
set -e; cd "$(dirname "$0")"
cargo build --release -p kindling --target wasm32-unknown-unknown
# The JS glue ships inside the crates; look in every registry index dir (there can be more than
# one, e.g. crates.io's git and sparse indexes) and take the newest matching version.
find_js() { # crate-glob js-file
    f=$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/$1/js/$2 2>/dev/null | sort -V | tail -1)
    [ -n "$f" ] || { echo "build-web.sh: $2 not found in any $1 under the cargo registry" >&2; exit 1; }
    echo "$f"
}
GL=$(find_js 'miniquad-0.4.*' gl.js)
JSU=$(find_js 'sapp-jsutils-0.1.*' sapp_jsutils.js)
URL=$(find_js 'quad-url-0.1.*' quad-url.js)
rm -rf web-dist; mkdir -p web-dist
cp target/wasm32-unknown-unknown/release/kindling.wasm web-dist/
cp "$GL" web-dist/gl.js
cp "$JSU" web-dist/
cp "$URL" web-dist/
cp web/index.html web/civ_io.js web-dist/
rm -f web-dist.zip; zip -rq web-dist.zip web-dist
