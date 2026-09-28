#!/usr/bin/env bash
# Packages a Unix alya release archive (binary + README + LICENSE + SHA-256).
# Usage: package-unix-release.sh <version> <platform> <target> <bin>
set -euo pipefail

VERSION="$1"
PLATFORM="$2"
TARGET="$3"
BIN="$4"

PACKAGE_NAME="alya-${VERSION}-${PLATFORM}"
mkdir "${PACKAGE_NAME}"
cp "target/${TARGET}/release/${BIN}" "${PACKAGE_NAME}/"
cp README.md LICENSE "${PACKAGE_NAME}/"
tar -czvf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
shasum -a 256 "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256"
