## Issue Description

A clear and concise description of the issue.

## Steps to Reproduce

1. Set environment variables:

   ```bash
   export OUTPUT_FILE="..."
   export DATABASE_URL="..."
   export DATABASE_QUERY="..."
   ```

   ⚠️ Security Note: Before posting this issue, please redact or obfuscate any sensitive credentials in your DATABASE_URL. Replace usernames, passwords, and hostnames with `[REDACTED]` or use a sanitized example like mysql://user:pass@host:3306/db.

2. Run the command:

   ```bash
   cargo run --release
   ```

3. See the issue

## Expected Behavior

A clear and concise description of what you expected to happen.

## Actual Behavior

A clear and concise description of what actually happened.

## Environment

- **OS**: [e.g. macOS 14.0, Ubuntu 22.04, Windows 11]
- **Rust Version**: [e.g. 1.89.0]
- **gold_digger Version**: [e.g. v0.2.5]
- **MySQL/MariaDB Version**: [e.g. MySQL 8.0, MariaDB 10.11]

## Additional Context

Add any other context about the issue here.

## Issue Type

Please select the appropriate template for your issue:

- [Bug Report](https://github.com/EvilBit-Labs/gold_digger/issues/new?template=bug_report.md)
- [Feature Request](https://github.com/EvilBit-Labs/gold_digger/issues/new?template=feature_request.md)
- [Security Report](https://github.com/EvilBit-Labs/gold_digger/issues/new?template=security_report.yml)

---

**Note**: This is a single-maintainer project. Please be patient with response times and provide as much detail as possible to help with issue resolution.
