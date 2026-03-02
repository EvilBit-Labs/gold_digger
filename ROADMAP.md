# Roadmap

See [GitHub Milestones](https://github.com/EvilBit-Labs/gold_digger/milestones) for detailed issue tracking.

## v0.2.x (Current)

- [x] CLI-first design with environment variable fallbacks
- [x] CSV (RFC 4180), JSON, and TSV output formats
- [x] Safe MySQL value conversion with NULL handling
- [x] Rustls-only TLS with platform certificate store integration
- [x] Structured exit codes (0-5) with intelligent error categorization
- [x] Shell completion generation (Bash, Zsh, Fish, PowerShell)
- [x] Configuration debugging with credential redaction
- [x] Query file support (`--query-file`)
- [x] Pretty-print JSON output
- [x] Empty result handling (`--allow-empty`)

## v0.3.0

- [x] Comprehensive testing framework with benchmarking and snapshot testing
- [ ] Refactored CI configuration for improved clarity and stability

## v0.4.0

- [ ] Streaming output for large result sets (memory-efficient row processing)
- [ ] Structured logging with credential redaction
- [ ] Enhanced JSON type inference and output options

## v0.5.0

- [ ] Performance optimizations and connection pooling improvements
- [ ] Additional output format options and customization

## v0.6.0

- [ ] Extended database feature support
- [ ] Advanced query options and configuration

## v1.0.0 - Production Ready

- [ ] Stable CLI with semver guarantees
- [ ] Complete documentation and migration guides
- [ ] Performance parity validation
- [ ] crates.io publication with stable API

## Non-Goals

The following are explicitly out of scope for this project:

- **Interactive database client**: Gold Digger is designed for headless, automated operation
- **Non-MySQL databases**: No support for PostgreSQL, SQLite, or other database engines
- **Query composition or SQL generation**: Users provide their own SQL queries
- **Schema migration or administration**: This is a query-and-export tool only
- **GUI or TUI**: No graphical or terminal user interface
- **Server or daemon mode**: Designed as a single-invocation CLI tool
- **Built-in scheduling**: Use external schedulers (cron, systemd timers, etc.)
- **Real-time streaming or CDC**: Designed for point-in-time query snapshots
