# Changelog

All notable changes to this project are documented here. This project follows
[Semantic Versioning](https://semver.org/) starting at 2.0.0.

## [2.0.0]

### Breaking

- **Authentication redesigned to Ed25519 signatures.** Each update is a
  short-lived, EdDSA-signed token (compact JWT) asserting the account and WAN
  IP; the backend stores only the public key. This replaces the previous
  shared-secret hash and is not compatible with older backends.
- Configuration changed: `KEY` is now a base64 Ed25519 private seed and `TOKEN`
  is removed. New optional `KID` selects the key id sent with each update.
- Rust 2024 edition; refreshed dependency stack.

### Added

- `keygen` subcommand to generate an Ed25519 keypair for provisioning.
- Multi-architecture release builds (Windows, Linux, macOS across several
  targets, plus tuned CPU variants) with `.deb` packages for Linux.
- Application icon embedded in the Windows executable.
- Tag-driven release versioning: the release tag is the source of truth.
- SBOM generation on release.

### Security

- `quinn-proto` updated to address advisories over the release cycle.
