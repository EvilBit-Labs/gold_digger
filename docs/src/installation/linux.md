# Linux Installation

Install Gold Digger on Linux distributions.

## Pre-built Binaries (Recommended)

1. Visit the [GitHub Releases](https://github.com/EvilBit-Labs/gold_digger/releases) page
2. Download the latest `gold_digger-linux` file
3. Make it executable and install:

```bash
chmod +x gold_digger-linux
sudo mv gold_digger-linux /usr/local/bin/gold_digger
```

4. Verify installation: `gold_digger --version`

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

**Ubuntu/Debian:**

```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev git
```

**RHEL/CentOS/Fedora:**

```bash
sudo dnf install gcc pkg-config openssl-devel git
# or for older versions:
# sudo yum install gcc pkg-config openssl-devel git
```

**Arch Linux:**

```bash
sudo pacman -S base-devel pkg-config openssl git
```

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

Linux builds use OpenSSL by default. For pure Rust TLS (no OpenSSL dependency):

```bash
# Standard installation with TLS support
cargo install gold_digger

# Or minimal installation without TLS
cargo install gold_digger --no-default-features --features "json,csv,additional_mysql_types,verbose"
```

## Distribution Packages (Coming Soon)

Future releases will include:

- Debian/Ubuntu `.deb` packages
- RHEL/CentOS/Fedora `.rpm` packages
- Arch Linux AUR package

## Verification

Test your installation:

```bash
gold_digger --help
```

## Troubleshooting

### Common Issues

- **Missing OpenSSL development headers**: Install `libssl-dev` or `openssl-devel`
- **Linker errors**: Install `build-essential` or equivalent
- **Permission denied**: Check executable permissions and PATH

### Getting Help

If you encounter issues:

1. Check the [Troubleshooting Guide](../troubleshooting/README.md)
2. Visit the [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues) page
