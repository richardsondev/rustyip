#!/usr/bin/env bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

# === System Setup ===
echo "[*] Updating system packages..."
apt-get update -y
apt-get upgrade -y

echo "[*] Installing base build tools..."
apt-get install -y --no-install-recommends \
  build-essential \
  pkg-config \
  libssl-dev \
  zlib1g-dev \
  ca-certificates \
  curl \
  git \
  clang \
  cmake \
  python3 \
  python3-pip \
  jq \
  lld

# === Rust Setup (x86_64-unknown-linux-gnu only) ===
if ! command -v rustup >/dev/null 2>&1; then
  echo "[*] Installing Rust via rustup..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
else
  echo "[*] rustup present; updating..."
  rustup self update || true
fi

echo "[*] Configuring Rust toolchain..."
rustup toolchain install stable --profile minimal
rustup default stable
rustup target add x86_64-unknown-linux-gnu || true

# Remove any other installed Rust targets to keep env x64-only
mapfile -t INSTALLED < <(rustup target list --installed || true)
for t in "${INSTALLED[@]}"; do
  if [[ "$t" != "x86_64-unknown-linux-gnu" ]]; then
    rustup target remove "$t" || true
  fi
done

# Optional: prefer lld; default linker is cc
mkdir -p "$HOME/.cargo"
cat > "$HOME/.cargo/config.toml" <<'EOI'
[target.x86_64-unknown-linux-gnu]
linker = "cc"
# To use lld, uncomment the next line:
# rustflags = ["-C", "link-arg=-fuse-ld=lld"]
EOI

# === Verification ===
echo "[*] Verifying installation..."
source "$HOME/.cargo/env"
rustc --version
cargo --version
echo "[*] Rust targets installed:"
rustup target list --installed

echo "[✓] Environment setup complete."
