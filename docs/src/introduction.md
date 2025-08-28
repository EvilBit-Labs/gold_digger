# Gold Digger Documentation

Welcome to the Gold Digger documentation! Gold Digger is a fast, secure MySQL/MariaDB query tool written in Rust that exports structured data in multiple formats.

## What is Gold Digger?

Gold Digger is a command-line tool designed for extracting data from MySQL and MariaDB databases with structured output support. It provides:

- **Multiple Output Formats**: Export data as CSV, JSON, or TSV
- **Security First**: Built-in TLS (rustls) support and credential protection
- **Type Safety**: Rust-powered reliability with proper NULL handling
- **CLI-First Design**: Environment variable support with CLI override capability

## Quick Navigation

- **New to Gold Digger?** Start with our [Quick Start Guide](usage/quick-start.md)
- **Need to install?** Check our [Installation Guide](installation/README.md)
- **Looking for examples?** Browse our [Usage Examples](usage/examples.md)
- **Developer?** Visit the [API Reference](development/api-reference.md)
- **Having issues?** See our [Troubleshooting Guide](troubleshooting/README.md)

## Key Features

- **CLI-First Design**: Command-line flags with environment variable fallbacks
- **Safe Type Handling**: Automatic NULL and type conversion without panics
- **Multiple Output Formats**: CSV (RFC 4180), JSON with type inference, TSV
- **Secure by Default**: Automatic credential redaction with TLS support
- **Structured Exit Codes**: Proper error codes for automation and scripting
- **Shell Integration**: Completion support for Bash, Zsh, Fish, PowerShell
- **Configuration Debugging**: JSON config dump with credential protection
- **Cross-Platform**: Works on Windows, macOS, and Linux

## Getting Help

If you encounter issues or have questions:

1. Check the [Troubleshooting Guide](troubleshooting/README.md)
2. Review the [Configuration Documentation](usage/configuration.md)
3. Visit the [GitHub Repository](https://github.com/UncleSp1d3r/gold_digger)

Let's get started with Gold Digger!
