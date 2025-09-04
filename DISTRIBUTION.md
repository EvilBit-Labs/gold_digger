# Distribution Guide

This document describes the cross-platform distribution setup for Gold Digger using cargo-dist.

## Overview

Gold Digger uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) to provide standardized cross-platform distribution with:

- **Cross-platform binaries** for Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), and Windows
- **Automated installers** (shell script, PowerShell, MSI, Homebrew)
- **Signed artifacts** with GitHub attestation
- **Complete SBOMs** (Software Bill of Materials) for all artifacts
- **Automated checksums** (SHA256) for integrity verification

## Supported Platforms

### Target Platforms

- **Linux**: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- **macOS**: `x86_64-apple-darwin`, `aarch64-apple-darwin`
- **Windows**: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`

### Installers

1. **Shell Script** (`gold_digger-installer.sh`)

   - Cross-platform installer for Linux and macOS
   - Automatically detects architecture and downloads appropriate binary
   - Installs to `~/.local/bin` by default

2. **PowerShell Script** (`gold_digger-installer.ps1`)

   - Windows installer with automatic architecture detection
   - Installs to user's local application directory
   - Adds to PATH automatically

3. **Homebrew Formula**

   - Available via `EvilBit-Labs/tap/gold-digger`
   - Supports both Intel and Apple Silicon Macs
   - Automatic dependency management

4. **MSI Installer** (Windows)

   - Native Windows installer package
   - Supports both x86_64 and ARM64 architectures
   - Integrates with Windows Add/Remove Programs

## Installation Methods

### Quick Install (Recommended)

```bash
# Linux/macOS (shell script)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/gold_digger-installer.sh | sh

# Windows (PowerShell)
powershell -c "irm https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/gold_digger-installer.ps1 | iex"
```

### Package Managers

```bash
# Homebrew (macOS/Linux)
brew tap EvilBit-Labs/tap
brew install gold-digger

# Chocolatey (Windows) — Planned
# choco install gold-digger

# Scoop (Windows) — Planned
# scoop bucket add evilbit-labs https://github.com/EvilBit-Labs/scoop-bucket
# scoop install gold-digger
```

### Manual Download

1. Visit [GitHub Releases](https://github.com/EvilBit-Labs/gold_digger/releases)
2. Download the appropriate archive for your platform:
   - Linux: `gold_digger-x86_64-unknown-linux-gnu.tar.gz`
   - macOS: `gold_digger-x86_64-apple-darwin.tar.gz`
   - Windows: `gold_digger-x86_64-pc-windows-msvc.zip`
3. Extract and place the binary in your PATH

## Security Features

### Artifact Signing

All release artifacts are signed using GitHub attestation:

```bash
# Verify a downloaded binary using GitHub CLI
gh attestation verify gold_digger-x86_64-unknown-linux-gnu

# Or verify using the GitHub web interface
# Navigate to the release page and check the "Security" tab
```

### Software Bill of Materials (SBOM)

Each release includes comprehensive SBOMs in CycloneDX format:

```bash
# Download and inspect SBOM
curl -L -o sbom.json https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/gold_digger-x86_64-unknown-linux-gnu.sbom.cdx.json

# View dependencies
jq '.components[] | {name: .name, version: .version, type: .type}' sbom.json
```

### Checksum Verification

All artifacts include SHA256 checksums:

```bash
# Download checksums
curl -L -o SHA256SUMS https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/SHA256SUMS

# Verify downloaded binary
sha256sum -c SHA256SUMS --ignore-missing
```

## Development

### Local Testing

```bash
# Install cargo-dist
cargo install cargo-dist --locked

# Test configuration
just dist-check

# Plan a release (shows what would be built)
just dist-plan

# Build artifacts locally
just dist-build

# Generate installers
just dist-generate
```

### Configuration

The distribution configuration is defined in `cargo-dist.toml`:

```toml
# Key configuration options
cargo-dist-version = "0.22.1"
ci = "github"
installers = ["shell", "powershell", "homebrew", "msi"]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "aarch64-pc-windows-msvc",
]
```

### Release Process

1. **Automated Releases**: cargo-dist integrates with the existing release workflow
2. **Triggered by Tags**: Release artifacts are built when version tags are pushed
3. **Security Integration**: All artifacts are signed and include SBOMs
4. **Multi-platform**: Builds happen on native runners for each platform

### Workflow Integration

The cargo-dist workflow (`.github/workflows/cargo-dist.yml`) integrates with the existing release process:

1. **Main Release Workflow** (`release.yml`) handles core functionality
2. **cargo-dist Workflow** (`cargo-dist.yml`) generates distribution artifacts
3. **Security Integration** ensures all artifacts are signed and include SBOMs
4. **Coordinated Execution** via workflow triggers

## Troubleshooting

### Common Issues

1. **Architecture Detection**: Installers automatically detect architecture, but you can override:

   ```bash
   # Force specific architecture
   curl -L https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/gold_digger-aarch64-unknown-linux-gnu.tar.gz
   ```

2. **Permission Issues**: Installers may need elevated permissions:

   ```bash
   # Install to user directory (no sudo required)
   curl ... | sh -s -- --to ~/.local/bin
   ```

3. **Homebrew Issues**:

   ```bash
   # Update tap if formula is outdated
   brew tap --repair
   brew update
   ```

### Getting Help

- **Issues**: [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EvilBit-Labs/gold_digger/discussions)
- **Security**: See [SECURITY.md](SECURITY.md) for security-related issues

## Future Enhancements

- **Additional Package Managers**: Chocolatey, Scoop, APT/YUM repositories
- **Container Images**: Docker images for containerized deployments
- **Verification Tools**: Enhanced verification scripts and tools
- **Auto-updates**: Built-in update mechanisms for installed binaries
