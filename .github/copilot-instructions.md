# GitHub Copilot Instructions for Gold Digger

## Purpose and Scope

These instructions are specifically for **GitHub Copilot AI Coding Assistant**. For comprehensive project details, see:

- **[WARP.md](../WARP.md)** - Authoritative development guide with architecture, TLS, and commands
- **[AGENTS.md](../AGENTS.md)** - Rules of engagement for AI assistants
- **[project_spec/requirements.md](../project_spec/requirements.md)** - Complete requirements and roadmap

**Project Summary:** Gold Digger is a Rust MySQL/MariaDB query tool that outputs structured data (CSV/JSON/TSV) via CLI and environment variables for headless automation workflows.

## Reviewer Preference and PR Etiquette

**⚠️ IMPORTANT:** This project uses **CodeRabbit.ai** as the primary reviewer. GitHub Copilot should:

- Provide code suggestions and diffs only
- **DO NOT** enable "Copilot for Pull Requests" auto-reviews
- Use conventional commits (e.g., `feat:`, `fix:`, `docs:`)
- Suggest small, focused changes for single-maintainer workflow
- Use `gh` CLI for GitHub interactions in examples

## 🚨 Critical Safety Rules - Database Value Conversion

**NEVER use `mysql::from_value::<String>()` directly - it WILL PANIC on NULL or non-string types.**

**Use these safe helpers instead:**

```rust
use mysql::{from_value_opt, Value};

/// Safe conversion for CSV/TSV output
fn mysql_value_to_string(v: &Value) -> String {
    match v {
        Value::NULL => "".to_string(),
        Value::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Value::Int(i) => i.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Date(year, month, day, hour, minute, second, micro) => {
            if *micro > 0 {
                format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}", year, month, day, hour, minute, second, micro)
            } else {
                format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second)
            }
        },
        Value::Time(neg, days, hours, minutes, seconds, micros) => {
            let sign = if *neg { "-" } else { "" };
            if *micros > 0 {
                format!("{}{}:{:02}:{:02}:{:02}.{:06}", sign, days * 24 + *hours as u32, minutes, seconds, micros)
            } else {
                format!("{}{}:{:02}:{:02}:{:02}", sign, days * 24 + *hours as u32, minutes, seconds)
            }
        },
    }
}

/// Safe conversion for JSON output (preserves native types where possible)
fn mysql_value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::NULL => serde_json::Value::Null,
        Value::Int(i) => serde_json::json!(*i),
        Value::UInt(u) => serde_json::json!(*u),
        Value::Float(f) => serde_json::json!(*f),
        Value::Double(d) => serde_json::json!(*d),
        Value::Bytes(b) => {
            // Prefer UTF-8 string if valid; otherwise hex for determinism
            match std::str::from_utf8(b) {
                Ok(s) => serde_json::Value::String(s.to_owned()),
                Err(_) => serde_json::Value::String(format!("0x{}", hex::encode(b))),
            }
        },
        Value::Date(y, m, d, hh, mm, ss, micros) => {
            serde_json::Value::String(format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{:06}Z", micros))
        },
        Value::Time(is_neg, d, h, m, s, micros) => serde_json::Value::String(format!(
            "{}{}:{:02}:{:02}:{:02}.{:06}",
            if *is_neg { "-" } else { "" },
            d * 24 + h,
            m,
            s,
            micros,
        )),
    }
}
```

**Usage:** CSV/TSV → `mysql_value_to_string()`, JSON → `mysql_value_to_json()`

**SQL Queries:** ⚠️ **WARNING**: `CAST(column AS CHAR)` can corrupt binary data or produce mojibake for text in lossy encodings. Use safer alternatives:

- **BLOB/BINARY columns**: Use `HEX(column)` or `TO_BASE64(column)` for lossless binary representation
- **Text columns**: Use `CAST(column AS CHAR CHARACTER SET utf8mb4)` or `CONVERT(column USING utf8mb4)` to specify explicit encoding
- **Numeric/Date columns**: `CAST(column AS CHAR)` is generally safe for these types

## Security Requirements (NEVER VIOLATE)

- **NEVER** log `DATABASE_URL` or credentials - always redact
- **NEVER** make external service calls at runtime (offline-first)
- Use `anyhow::Result<T>` and `?` for error propagation
- Respect system umask for output files
- No hardcoded secrets; use environment variables only

```rust
// ❌ NEVER do this
println!("Connecting to {}", database_url);

// ✅ Always redact
println!("Connecting to database...");
```

## Architecture Snapshot

### Current (v0.2.x)

- CLI-first with env fallbacks; missing some required flags
- Fetches ALL rows into memory (no streaming); exits 1 on empty result
- Extension dispatch: `.csv`/`.json`/else→TSV
- `from_value<String>` usage causes panic on NULL/non-string (must be fixed)
- `exit(-1)` → exit code 255 (non-standard)

### Target (v1.0 per requirements)

- Proper clap CLI, config precedence, `--query-file`/`--format` flags
- Standard exit codes (0=success, 1=no rows, 2=config error, ...)
- Safe type conversion; deterministic JSON; pretty-print option
- Streaming support for large result sets; structured logging with redaction

*See [WARP.md](../WARP.md) and [project_spec/requirements.md](../project_spec/requirements.md) for details.*

## TLS and Feature Flags Quick Guide

### TLS Options (Rustls Only)

- **Default:** `ssl-rustls` feature - Pure Rust TLS implementation using rustls-tls-ring
- **Note:** The project has been converted to use only rustls; native-tls support has been removed

### Current TLS Implementation

The project includes a dedicated `src/tls.rs` module with:

- `TlsConfig` struct for programmatic TLS configuration
- `create_tls_connection()` function for TLS-enabled connections
- Enhanced error handling with specific TLS error types
- Credential redaction for safe error logging

### Build Examples

```bash
# Default build (rustls TLS)
cargo build --release

# Standard build (TLS always available)
cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"

# Minimal build (no TLS)
cargo build --release --no-default-features --features "json csv"
```

### TLS Configuration

TLS is configured programmatically using the `TlsConfig` struct and `create_tls_connection()` function. URL-based ssl-mode parameters are not supported by the mysql crate.

```rust
use gold_digger::tls::{TlsConfig, create_tls_connection};

// Basic TLS connection
let pool = create_tls_connection(database_url, Some(TlsConfig::new()))?;

// TLS with custom CA certificate
let tls_config = TlsConfig::new()
    .with_ca_cert_path("/path/to/ca.pem")
    .with_skip_domain_validation(false)
    .with_accept_invalid_certs(false);
let pool = create_tls_connection(database_url, Some(tls_config))?;
```

## Quick Reference Commands

```bash
# Formatting (enforced)
cargo fmt --check

# Linting (enforced)
cargo clippy -- -D warnings

# Build
cargo build --release

# Run (environment-driven - NO dotenv support)
OUTPUT_FILE=/tmp/out.json \
DATABASE_URL="mysql://user:pass@host:3306/db" \
DATABASE_QUERY="SELECT CAST(id AS CHAR) as id FROM table LIMIT 5" \
cargo run --release

# Test
cargo test
```

## Common Mistakes to Avoid

1. **Using `from_value<String>()` directly** (panics on NULL/non-string)
2. **Assuming dotenv support** (not implemented - export env vars explicitly)
3. **Pattern bug:** `Some(&_)` vs `Some(_)` in extension dispatch
4. **Logging DATABASE_URL or secrets** (always redact)
5. **Relying on vendored OpenSSL** (removed - use `ssl-rustls`)
6. **Non-standard exit codes** (`exit(-1)` → 255 rather than defined codes)

## Before You Suggest Changes - Checklist for Copilot

- [ ] Use safe database value conversion helpers (never `from_value<String>`)
- [ ] Redact all credentials in logs and examples
- [ ] Use rustls-only TLS implementation (ssl-rustls feature)
- [ ] Use conventional commit format
- [ ] Suggest small, focused diffs for single-maintainer workflow
- [ ] Use `gh` CLI for GitHub interactions
- [ ] Link to WARP.md/AGENTS.md for comprehensive context

## References

- **[WARP.md](../WARP.md)** - Complete development guide (architecture, TLS, commands, Mermaid diagrams)
- **[AGENTS.md](../AGENTS.md)** - AI assistant rules of engagement
- **[project_spec/requirements.md](../project_spec/requirements.md)** - Requirements roadmap and feature gaps

---

**Maintainer:** UncleSp1d3r • **Primary Reviewer:** CodeRabbit.ai • **No auto-commits** • **Use `gh` CLI**
