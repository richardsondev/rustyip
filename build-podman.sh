#!/usr/bin/env bash
# Cross-build RustyIP for every Linux + Windows (GNU) target we can, inside a
# Rust container, placing arch-named binaries in dist/.
#
# Run from the repo root, e.g.:
#   podman run --rm --name rustyip-build \
#       -e CARGO_TARGET_DIR=/tmp/target -e CARGO_BUILD_JOBS=2 \
#       -v "${PWD}:/work" -w /work docker.io/library/rust:1.96 bash build-podman.sh
#
# Toolchain notes:
#   * The `ring` crate compiles C, so each cross target needs its libc dev
#     headers; we use the `crossbuild-essential-*` meta-packages (which pull
#     gcc-<triple> + libc6-dev-<arch>-cross) rather than the bare gcc-<triple>.
#   * gcc-multilib (for i686-unknown-linux-gnu) conflicts with the GNU cross
#     compilers, so toolchains are installed in two phases.
#   * macOS targets are excluded (they require the Apple SDK; build those on a
#     macOS host with build.sh).
set -uo pipefail
export DEBIAN_FRONTEND=noninteractive

declare -a OK=()
declare -a FAIL=()

build_target() {
  local t="$1"; local sub="${2:-}"
  echo ""
  echo "==================== ${t} ${sub:+($sub)} ===================="
  rustup target add "$t" >/dev/null 2>&1 || true
  if bash build.sh "$t" "$sub"; then
    OK+=("$t")
  else
    echo "!!! build FAILED for $t"
    FAIL+=("$t")
  fi
}

echo "[podman-build] updating apt index..."
apt-get update -qq

# Phase 1: native x86_64 + 32-bit x86 (multilib) + Windows (mingw).
# multilib coexists with mingw but NOT with the GNU cross compilers.
echo "[podman-build] phase 1: installing mingw + multilib..."
apt-get install -y --no-install-recommends gcc-mingw-w64 gcc-multilib >/dev/null

build_target x86_64-unknown-linux-gnu
build_target i686-unknown-linux-gnu
build_target x86_64-pc-windows-gnu
build_target i686-pc-windows-gnu

# Phase 2: ARM/AArch64 need the GNU cross toolchains (with target libc headers),
# which conflict with gcc-multilib; swap it out first.
echo "[podman-build] phase 2: swapping multilib -> crossbuild-essential (arm/aarch64)..."
apt-get remove -y gcc-multilib >/dev/null 2>&1 || true
apt-get autoremove -y >/dev/null 2>&1 || true
apt-get install -y --no-install-recommends \
  crossbuild-essential-arm64 \
  crossbuild-essential-armhf \
  crossbuild-essential-armel \
  >/dev/null

build_target aarch64-unknown-linux-gnu
build_target armv7-unknown-linux-gnueabihf
build_target arm-unknown-linux-gnueabi

echo ""
echo "[podman-build] ============ summary ============"
echo "  succeeded (${#OK[@]}): ${OK[*]:-none}"
echo "  failed    (${#FAIL[@]}): ${FAIL[*]:-none}"
echo ""
echo "[podman-build] dist/ contents:"
ls -la dist/ || true

# Succeed overall if at least one target built.
[ "${#OK[@]}" -gt 0 ]
