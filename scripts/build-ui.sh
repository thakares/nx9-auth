#!/usr/bin/env bash
# Build the Dioxus web UI into ui/dist for serving by nx9-auth.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
WASM_OUT="$TARGET/wasm32-unknown-unknown/release/nx9-auth-ui.wasm"
DIST="$ROOT/ui/dist"

echo "==> Building nx9-auth-ui (wasm32-unknown-unknown, release)"
cargo build --manifest-path ui/Cargo.toml --target-dir "$TARGET" --target wasm32-unknown-unknown --release

if [ ! -f "$WASM_OUT" ] && [ -f "$ROOT/ui/target/wasm32-unknown-unknown/release/nx9-auth-ui.wasm" ]; then
  WASM_OUT="$ROOT/ui/target/wasm32-unknown-unknown/release/nx9-auth-ui.wasm"
fi

WBG_VER="$(cargo tree -p nx9-auth-ui -i wasm-bindgen --depth 0 2>/dev/null | head -1 | sed -n 's/.*v\([0-9.]*\).*/\1/p')"
WBG_VER="${WBG_VER:-0.2.125}"

if ! command -v wasm-bindgen >/dev/null 2>&1 || ! wasm-bindgen --version 2>/dev/null | grep -q "$WBG_VER"; then
  echo "==> Ensuring wasm-bindgen ${WBG_VER}"
  TMP="${TMPDIR:-/tmp}/nx9-wbg"
  mkdir -p "$TMP"
  URL="https://github.com/rustwasm/wasm-bindgen/releases/download/${WBG_VER}/wasm-bindgen-${WBG_VER}-x86_64-unknown-linux-musl.tar.gz"
  if curl -fsSL "$URL" -o "$TMP/wbg.tar.gz"; then
    tar -xzf "$TMP/wbg.tar.gz" -C "$TMP"
    WBG="$(find "$TMP" -name wasm-bindgen -type f | head -1)"
  else
    WBG="wasm-bindgen"
  fi
else
  WBG="wasm-bindgen"
fi

echo "==> Packaging with wasm-bindgen ($("$WBG" --version 2>/dev/null || true))"
rm -rf "$DIST"
mkdir -p "$DIST/assets"
"$WBG" "$WASM_OUT" \
  --out-dir "$DIST" \
  --out-name nx9_auth_ui \
  --target web \
  --no-typescript

cp -f "$ROOT/ui/assets/style.css" "$DIST/assets/style.css"
cp -f "$ROOT/ui/assets/boot.js" "$DIST/assets/boot.js"
cp -f "$ROOT/ui/assets/favicon.svg" "$DIST/assets/favicon.svg"
# Use the canonical index with absolute module paths + error surface
cp -f "$ROOT/ui/index.html" "$DIST/index.html"

# Also place next to the release binary for single-binary-adjacent deploys
RELEASE_UI="$TARGET/release/ui/dist"
if [ -d "$TARGET/release" ]; then
  mkdir -p "$RELEASE_UI"
  cp -a "$DIST/." "$RELEASE_UI/"
  echo "==> Also copied to $RELEASE_UI"
fi

echo "==> UI assets ready in $DIST"
ls -lah "$DIST"
# Quick sanity: required files
for f in index.html nx9_auth_ui.js nx9_auth_ui_bg.wasm assets/style.css; do
  if [ ! -e "$DIST/$f" ]; then
    echo "ERROR: missing $DIST/$f" >&2
    exit 1
  fi
done
echo "==> Sanity check OK"
