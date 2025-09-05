# macOS Installation

Install Gold Digger on macOS systems.

## Pre-built Binaries (Recommended)

1. Visit the [GitHub Releases](https://github.com/EvilBit-Labs/gold_digger/releases) page
2. Download the latest `gold_digger-macos` file
3. Make it executable and move to PATH:

```bash
chmod +x gold_digger-macos
sudo mv gold_digger-macos /usr/local/bin/gold_digger
```

4. Verify installation: `gold_digger --version`

## Using Homebrew (Coming Soon)

```bash
# Future release
brew install gold_digger
```

## Using Cargo (Rust Package Manager)

### Prerequisites

Install Rust using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Install Gold Digger

```bash
cargo install gold_digger
```

## Build from Source

### Prerequisites

- Xcode Command Line Tools: `xcode-select --install`
- Rust toolchain (via rustup)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/EvilBit-Labs/gold_digger.git
cd gold_digger

# Build release version
cargo build --release

# The executable will be in target/release/gold_digger
```

## TLS Support

Pure Rust TLS (rustls) by default; no native SecureTransport path.

```bash
# Standard installation (TLS via rustls)
cargo install gold_digger

# Or minimal installation without extra features (TLS still available)
cargo install gold_digger --no-default-features --features "json,csv,additional_mysql_types,verbose"
```

## Verification

Test your installation:

```bash
gold_digger --help
```

## Troubleshooting

### Common Issues

- **Gatekeeper blocking execution**: Right-click → Open, or use `sudo spctl --master-disable`
- **Command not found**: Ensure `/usr/local/bin` is in your PATH
- **Permission denied**: Check file permissions with `ls -la`

### Getting Help

If you encounter issues:

1. Check the [Troubleshooting Guide](../troubleshooting/README.md)
2. Visit the [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues) page
