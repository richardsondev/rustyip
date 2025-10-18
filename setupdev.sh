#!/usr/bin/env bash
set -euo pipefail

# === System Setup ===
echo "[*] Updating system packages..."
apt-get update -y
apt-get upgrade -y

echo "[*] Installing base build tools..."
apt-get install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  curl \
  git \
  clang \
  cmake \
  python3 \
  python3-pip \
  jq

# === Rust Setup ===
if ! command -v rustc &>/dev/null; then
  echo "[*] Installing Rust via rustup..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  source "$HOME/.cargo/env"
else
  echo "[*] Rust already installed, updating..."
  rustup update
fi

echo "[*] Adding common Rust targets..."
targets=(
  i686-unknown-linux-gnu
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  armv7-unknown-linux-gnueabihf
  mips64-unknown-linux-gnuabi64
)
for t in "${targets[@]}"; do
  rustup target add "$t" || true
done

# === Cross Compilation Toolchains ===
echo "[*] Installing cross-compilation packages..."
apt-get install -y \
  gcc-aarch64-linux-gnu \
  gcc-arm-linux-gnueabihf \
  gcc-mips64-linux-gnuabi64 \
  gcc-mingw-w64 \
  gcc-multilib \
  crossbuild-essential-arm64 \
  binutils-arm-linux-gnueabi

# === Optional Developer Utilities ===
echo "[*] Installing developer tools..."
apt-get install -y \
  ripgrep \
  fd-find \
  lld \
  llvm \
  vim \
  tree

# === Verification ===
echo "[*] Verifying installation..."
rustc --version
cargo --version
echo "[*] Rust targets:"
rustup target list --installed

echo "[✓] Environment setup complete."
