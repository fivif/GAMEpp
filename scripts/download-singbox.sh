#!/bin/bash
# Download sing-box for bundling
set -e

OS="${1:-$(uname -s)}"
ARCH="${2:-$(uname -m)}"
DEST="${3:-src-tauri/bin}"

mkdir -p "$DEST"

case "$OS" in
  Darwin|darwin|macos)
    URL="https://github.com/SagerNet/sing-box/releases/latest/download/sing-box-darwin-arm64.tar.gz"
    BINARY="sing-box"
    ;;
  Linux|linux)
    URL="https://github.com/SagerNet/sing-box/releases/latest/download/sing-box-linux-amd64.tar.gz"
    BINARY="sing-box"
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows|windows)
    URL="https://github.com/SagerNet/sing-box/releases/latest/download/sing-box-windows-amd64.zip"
    BINARY="sing-box.exe"
    ;;
  *) echo "Unknown OS: $OS"; exit 1 ;;
esac

echo "Downloading sing-box for $OS..."
TMP=$(mktemp -d)
curl -sL "$URL" -o "$TMP/sing-box.archive"

if [[ "$URL" == *.zip ]]; then
  unzip -qo "$TMP/sing-box.archive" -d "$TMP"
else
  tar xzf "$TMP/sing-box.archive" -C "$TMP"
fi

# Find the binary in extracted files
BIN=$(find "$TMP" -name "$BINARY" -type f | head -1)
if [ -z "$BIN" ]; then
  echo "ERROR: sing-box binary not found in archive"
  ls -la "$TMP"/*/
  exit 1
fi

cp "$BIN" "$DEST/$BINARY"
chmod +x "$DEST/$BINARY"
rm -rf "$TMP"
echo "sing-box installed to $DEST/$BINARY"
