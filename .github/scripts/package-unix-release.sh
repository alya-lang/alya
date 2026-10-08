#!/usr/bin/env bash
# Packages a Unix alya release archive (binaries + README + LICENSE + SHA-256).
# Ships both `alya` and the standalone `alya-lsp` server in one archive.
# Usage: package-unix-release.sh <version> <platform> <target> <bin>
#   <bin> is the main compiler binary name (`alya`); the LSP binary name
#   (`alya-lsp`) is derived from it.
set -euo pipefail

VERSION="$1"
PLATFORM="$2"
TARGET="$3"
BIN="$4"

PACKAGE_NAME="alya-${VERSION}-${PLATFORM}"
mkdir "${PACKAGE_NAME}"
cp "target/${TARGET}/release/${BIN}" "${PACKAGE_NAME}/"
# Standalone LSP server: same archive so editors can fetch one file and
# use `alya-lsp` for editing plus `alya` for run/build/test.
if [ "${BIN}" = "alya" ]; then
  LSP_BIN="alya-lsp"
else
  LSP_BIN="${BIN%alya}alya-lsp"
fi
if [ ! -f "target/${TARGET}/release/${LSP_BIN}" ]; then
  # Manifests (scoop, winget) promise this file: never ship without it.
  echo "error: missing standalone LSP binary target/${TARGET}/release/${LSP_BIN}" >&2
  exit 1
fi
cp "target/${TARGET}/release/${LSP_BIN}" "${PACKAGE_NAME}/"
cp README.md LICENSE "${PACKAGE_NAME}/"
tar -czvf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
shasum -a 256 "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256"
