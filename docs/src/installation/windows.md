# Windows Installation

Install Gold Digger on Windows systems.

## Pre-built Binaries (Recommended)

1. Visit the [GitHub Releases](https://github.com/EvilBit-Labs/gold_digger/releases) page
2. Download the latest `gold_digger-windows.exe` file
3. Move the executable to a directory in your PATH
4. Open Command Prompt or PowerShell and verify: `gold_digger --version`

## Using Cargo (Rust Package Manager)

### Prerequisites

Install Rust from [rustup.rs](https://rustup.rs/):

```powershell
# Download and run rustup-init.exe
# Follow the installation prompts
```

### Install Gold Digger

```powershell
cargo install gold_digger
```

## Build from Source

### Prerequisites

- Git for Windows
- Rust toolchain (via rustup)
- Visual Studio Build Tools or Visual Studio Community

### Build Steps

```powershell
# Clone the repository
git clone https://github.com/EvilBit-Labs/gold_digger.git
cd gold_digger

# Build release version
cargo build --release

# The executable will be in target/release/gold_digger.exe
```

## TLS Support

Windows builds use the native SChannel TLS implementation by default. For pure Rust TLS:

```powershell
# Standard installation with TLS support
cargo install gold_digger

# Or minimal installation without TLS
cargo install gold_digger --no-default-features --features "json,csv,additional_mysql_types,verbose"
```

## Verification

Test your installation:

```powershell
gold_digger --help
```

## Troubleshooting

### Common Issues

- **Missing Visual C++ Redistributable**: Install from Microsoft
- **PATH not updated**: Restart your terminal after installation
- **Antivirus blocking**: Add exception for the executable

### Getting Help

If you encounter issues:

1. Check the [Troubleshooting Guide](../troubleshooting/README.md)
2. Visit the [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues) page
