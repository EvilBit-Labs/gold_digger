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

For support, questions, and understanding response times, see the [SUPPORT.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/SUPPORT.md) file in the repository. Key resources include:

1. [Troubleshooting Guide](troubleshooting/README.md) for common issues and solutions
2. [Configuration Documentation](usage/configuration.md) for setup and options
3. [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues) for bug reports and feature requests
4. [SECURITY.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/SECURITY.md) for security vulnerability reporting

## Project Resources

Gold Digger is an open source project with transparent governance and planning:

- **[SUPPORT.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/SUPPORT.md)**: Get help and understand support policies and response times
- **[GOVERNANCE.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/GOVERNANCE.md)**: Learn about project structure, roles, and decision-making processes
- **[ROADMAP.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/ROADMAP.md)**: View planned features and release milestones
- **[CONTRIBUTING.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/CONTRIBUTING.md)**: Guidelines for contributing code and documentation

Let's get started with Gold Digger!
