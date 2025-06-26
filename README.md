# RustyIP client

<div align="center">
<img src="icon/rustyip.png" alt="RustyIP Logo" width="128" height="128" />

*A dynamic DNS client with multi-architecture support*

[![Docker Multi-Arch](https://img.shields.io/badge/docker-multi--arch-blue?logo=docker)](https://hub.docker.com/r/richardsondev/rustyip)
[![GitHub Release](https://img.shields.io/github/v/release/richardsondev/rustyip)](https://github.com/richardsondev/rustyip/releases)
[![License](https://img.shields.io/github/license/richardsondev/rustyip)](LICENSE)
[![CI Status](https://img.shields.io/github/actions/workflow/status/richardsondev/rustyip/release.yml)](https://github.com/richardsondev/rustyip/actions)

</div>

## Container Support
Multi-architecture Docker containers are available for:
* **AMD64** (`linux/amd64`) - Standard x86_64 servers and desktops
* **ARM64** (`linux/arm64`) - Apple Silicon, AWS Graviton, Raspberry Pi 4+
* **ARMv7** (`linux/arm/v7`) - Raspberry Pi 2-3, modern ARM devices
* **ARMv6** (`linux/arm/v6`) - Raspberry Pi 1, Zero, older ARM devices
* **i386** (`linux/386`) - Legacy 32-bit x86 systems

## Binary Releases
Optimized native binaries are available for:

### Windows
* **32-bit Windows** (`i686-pc-windows-gnu`)
* **64-bit Windows** (`x86_64-pc-windows-gnu`) - Standard and CPU-optimized variants
* **64-bit Windows (Skylake)** - Intel Skylake+ optimizations
* **64-bit Windows (Znver3)** - AMD Zen 3+ optimizations

### Linux
* **32-bit Linux** (`i686-unknown-linux-gnu`)
* **64-bit Linux** (`x86_64-unknown-linux-gnu`) - Standard and CPU-optimized variants
* **64-bit Linux (Skylake)** - Intel Skylake+ optimizations
* **64-bit Linux (Znver3)** - AMD Zen 3+ optimizations
* **ARM32 ARMv6 Linux** (`arm-unknown-linux-gnueabi`)
* **ARM32 ARMv7 Linux** (`armv7-unknown-linux-gnueabihf`)
* **ARM64 Linux** (`aarch64-unknown-linux-gnu`)
* **MIPS64 Linux** (`mips64-unknown-linux-gnuabi64`)

### macOS
* **64-bit macOS** (`x86_64-apple-darwin`) - Standard and Skylake-optimized variants
* **ARM64 macOS** (`aarch64-apple-darwin`) - Apple Silicon

## Source
https://github.com/richardsondev/rustyip

## Prebuilt image
https://hub.docker.com/r/richardsondev/rustyip

## Usage

### Docker (Recommended)
```bash
# Basic usage - automatically selects your platform architecture
docker run -d --name=RustyIP --restart always \
  -e HOST='your.domain.com' \
  -e KEY='your-api-key' \
  -e TOKEN='your-token' \
  -e HASH='your-hash' \
  -e SLEEP_DURATION='300' \
  richardsondev/rustyip:latest

# Explicit architecture selection
docker run --platform linux/arm64 richardsondev/rustyip:latest

# With automatic updates via Watchtower
docker run -d --name=watchtower \
  -v /var/run/docker.sock:/var/run/docker.sock \
  containrrr/watchtower

docker run -d --name=RustyIP --restart always \
  --label=com.centurylinklabs.watchtower.enable=true \
  -e HOST='your.domain.com' \
  -e KEY='your-api-key' \
  -e TOKEN='your-token' \
  -e HASH='your-hash' \
  -e SLEEP_DURATION='300' \
  richardsondev/rustyip:latest
```

### Docker Compose
```yaml
version: '3.8'
services:
  rustyip:
    image: richardsondev/rustyip:latest
    container_name: RustyIP
    restart: always
    environment:
      - HOST=your.domain.com
      - KEY=your-api-key
      - TOKEN=your-token
      - HASH=your-hash
      - SLEEP_DURATION=300
```

### Native Binary
Download the appropriate binary for your platform from the [releases page](https://github.com/richardsondev/rustyip/releases) and run directly.
