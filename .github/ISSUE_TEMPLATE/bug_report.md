---
name: Bug Report
about: Create a report to help us improve gold_digger
title: '[BUG] '
labels: [bug]
assignees: [UncleSp1d3r]
---

## Bug Description

A clear and concise description of what the bug is.

## Steps to Reproduce

1. Set environment variables:

   ```bash
   export OUTPUT_FILE="..."
   export DATABASE_URL="..."
   export DATABASE_QUERY="..."
   ```

   **⚠️ Security Note**: Before posting this issue, please redact or obfuscate any sensitive credentials in your DATABASE_URL. Replace usernames, passwords, and hostnames with `<redacted>` or use a sanitized example like `mysql://user:pass@host:3306/db`.

2. Run the command:

   ```bash
   cargo run --release
   ```

3. See error

## Expected Behavior

A clear and concise description of what you expected to happen.

## Actual Behavior

A clear and concise description of what actually happened.

## Environment

- **OS**: [e.g. macOS 15.0, Ubuntu 22.04 LTS, Windows 11]
- **Rust Version**: [e.g. 1.89.0]
- **gold_digger Version**: [e.g. v0.2.5]
- **MySQL/MariaDB Version**: [e.g. MySQL 8.0, MariaDB 10.11]

## Build Configuration

```bash
# How did you build gold_digger?
cargo build --release  # Standard build with TLS
# OR
cargo build --no-default-features --features "json csv"  # Minimal build
```

## Error Output

```text
# Paste the full error output here
```

## Additional Context

Add any other context about the problem here, such as:

- Database schema details
- Query complexity
- Output format being used (CSV/JSON/TSV)
- SSL/TLS configuration if applicable

## Checklist

- [ ] I have searched existing issues to avoid duplicates
- [ ] I have provided all required environment information
- [ ] I have included the exact error message
- [ ] I have tested with a minimal reproduction case
