#!/usr/bin/env bash
# Builds lawliet-runtime as a Tauri sidecar and bundles Armonia into a
# distributable binary for the HOST platform. Run this on the OS you want to
# target — Tauri does not reliably cross-compile the GUI shell (WebView2/MSVC
# on Windows, WebKit on Linux/mac), so a Windows build must run on Windows.
set -euo pipefail

# repo root = script dir
cd "$(dirname "$0")"

TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
EXT=""
case "$TRIPLE" in
  *windows*) EXT=".exe" ;;
esac

echo ">> Building lawliet-runtime ($TRIPLE)"
cargo build --release -p lawliet-runtime

DEST="armonia/src-tauri/binaries"
mkdir -p "$DEST"
cp "target/release/lawliet-runtime$EXT" "$DEST/lawliet-runtime-$TRIPLE$EXT"
echo ">> Sidecar staged at $DEST/lawliet-runtime-$TRIPLE$EXT"

echo ">> Bundling Armonia"
cd armonia
npm install
npm run tauri build -- --config src-tauri/tauri.bundle.conf.json

echo ">> Done. Bundles are in target/release/bundle/"
