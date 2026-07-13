#!/usr/bin/env bash
# Build RustyIP for a single target and place the binary in dist/ with the
# architecture (Rust target triple) in the filename.
#
# Usage:   build.sh <rust-target> [subname]
# Env:     RUSTFLAGS   extra rustc flags (optional)
#          NIGHTLY=1   build with nightly + -Z build-std (for tier-3 targets)
#          DIST_DIR    output directory (default: dist)
#
# This is the single source of truth for artifact naming, shared by the CI
# workflow and the local/podman build scripts.
set -euo pipefail

TARGET="${1:?usage: build.sh <rust-target> [subname]}"
SUBNAME="${2:-}"
NIGHTLY="${NIGHTLY:-}"
DIST_DIR="${DIST_DIR:-dist}"

EXT=""
case "$TARGET" in
  *windows*) EXT=".exe" ;;
esac

SUFFIX=""
[ -n "$SUBNAME" ] && SUFFIX="-$SUBNAME"

OUT="RustyIP-${TARGET}${SUFFIX}${EXT}"
mkdir -p "$DIST_DIR"

TARGET_DIR="${CARGO_TARGET_DIR:-target}"

echo "[build] target=$TARGET subname=${SUBNAME:-<none>} flags=${RUSTFLAGS:-<none>} nightly=${NIGHTLY:-0}"
if [ -n "$NIGHTLY" ]; then
  cargo +nightly build -Z build-std --release --target "$TARGET"
else
  cargo build --release --target "$TARGET"
fi

cp "${TARGET_DIR}/${TARGET}/release/RustyIP${EXT}" "${DIST_DIR}/${OUT}"
echo "[build] -> ${DIST_DIR}/${OUT}"
