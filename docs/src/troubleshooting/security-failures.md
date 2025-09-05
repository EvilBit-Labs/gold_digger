# Security Failure Troubleshooting

This guide provides comprehensive solutions for security scanning failures, vulnerability issues,
and license compliance problems in the Gold Digger project.

## Quick Reference

| Failure Type      | Common Cause              | Quick Fix                        |
| ----------------- | ------------------------- | -------------------------------- |
| Vulnerabilities   | Outdated dependencies     | `cargo update`, check advisories |
| License Issues    | Incompatible licenses     | Review `deny.toml`, update deps  |
| Audit Failures    | Security advisories       | Update vulnerable crates         |
| SARIF Upload      | GitHub integration issues | Check permissions, retry         |
| Policy Violations | cargo-deny rules          | Review and update policies       |

## Vulnerability Scanning Failures

### High/Critical Vulnerabilities

**Error Pattern:**

```
error: 1 vulnerability found!
┌─────────────────────────────────────────────────────────────────────────────┐
│                               Advisory                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ ID       │ RUSTSEC-YYYY-NNNN                                               │
│ Package  │ vulnerable-crate                                                │
│ Version  │ 1.0.0                                                           │
│ Date     │ YYYY-MM-DD                                                      │
│ Title    │ Vulnerability description                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Solutions:**

1. **Update Dependencies:**

   **Prerequisite:** Install cargo-outdated if not already installed:

   ```bash
   cargo install cargo-outdated
   ```

   ```bash
   # Update all dependencies to latest patch versions
   cargo update

   # Update specific vulnerable crate
   cargo update -p vulnerable-crate

   # Check for available updates
   cargo outdated
   ```

2. **Find Dependency Path:**

   ```bash
   # Find which crate depends on the vulnerable one
   cargo tree -i vulnerable-crate

   # Show full dependency path
   cargo tree --format "{p} -> {d}"
   ```

3. **Replace Vulnerable Dependencies:**

   ```toml
   # In Cargo.toml, replace with secure alternative
   [dependencies]
   # old-vulnerable-crate = "1.0"  # Remove this
   secure-alternative = "2.0" # Use this instead
   ```

4. **Pin to Secure Version:**

   ```toml
   # In Cargo.toml, pin to specific secure version
   [dependencies]
   vulnerable-crate = "=1.2.3" # Known secure version
   ```

### Security Advisory Database Issues

**Error Pattern:**

```
error: advisory database is out of date
error: failed to fetch advisory database
```

**Solutions:**

1. **Update Advisory Database:**

   ```bash
   # Update cargo-audit database
   cargo audit --db ~/.cargo/advisory-db

   # Force database update
   rm -rf ~/.cargo/advisory-db
   cargo audit
   ```

2. **Manual Database Update:**

   ```bash
   # Clone advisory database manually
   git clone https://github.com/RustSec/advisory-db ~/.cargo/advisory-db

   # Update existing database
   cd ~/.cargo/advisory-db && git pull
   ```

### Grype Vulnerability Scanning

**Error Pattern:**

```
[0000]  WARN unable to check for vulnerability database update
[0000] ERROR failed to load vulnerability database
```

**Solutions:**

1. **Install/Update Grype:**

   ```bash
   # Install grype
   curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh | sh -s -- -b /usr/local/bin

   # Update grype database
   grype db update
   ```

   **Security Note:** The above install command uses an unpinned script URL. For production
   environments and CI reproducibility, pin to a specific grype release and verify
   checksums/signatures where available:

   ```bash
   # Preferred: Pin to specific release (example: v0.78.0)
   # curl -sSfL https://raw.githubusercontent.com/anchore/grype/v0.78.0/install.sh | sh -s -- -b /usr/local/bin

   # Verify checksum if available (check grype releases for SHA256)
   # echo "expected_checksum  grype" | sha256sum -c
   ```

2. **Run Grype Scan:**

   ```bash
   # Scan current directory
   grype .

   # Fail on high vulnerabilities (includes critical)
   grype . --fail-on high

   # Output SARIF format for GitHub
   grype . -o sarif > grype-results.sarif
   ```

## License Compliance Failures

### Incompatible License Issues

**Error Pattern:**

```
error: license `GPL-3.0` is not allowed
error: license `AGPL-3.0` is not allowed by policy
```

**Solutions:**

1. **Review License Policy:**

   ```bash
   # Check current license configuration
   cat deny.toml

   # Show licenses in use
   cargo deny check licenses
   ```

2. **Update License Policy:**

   ```toml
   # In deny.toml
   [licenses]
   allow = [
     "MIT",
     "Apache-2.0",
     "Apache-2.0 WITH LLVM-exception",
     "BSD-2-Clause",
     "BSD-3-Clause",
     "ISC",
     "Unicode-DFS-2016",
   ]

   # Add exceptions for specific crates if needed
   [[licenses.exceptions]]
   allow = ["GPL-3.0"]
   name = "specific-crate-name"
   ```

3. **Find Alternative Dependencies:**

   ```bash
   # Search for alternatives
   cargo search alternative-crate

   # Check crate licenses (requires cargo-license tool)
   cargo install cargo-license
   cargo license
   ```

### Unknown License Issues

**Error Pattern:**

```
error: license `Unknown` found for crate `some-crate`
error: unable to determine license for dependency
```

**Solutions:**

1. **Clarify Unknown Licenses:**

   ```toml
   # In deny.toml
   [[licenses.clarify]]
   name = "some-crate"
   version = "1.0"
   expression = "MIT" # Manually specify license
   license-files = [
     { path = "LICENSE", hash = 0x12345678 },
   ]
   ```

2. **Contact Crate Authors:**

   ```bash
   # Check crate repository for license information
   cargo info some-crate

   # File issue requesting license clarification
   ```

## cargo-deny Policy Violations

### Banned Dependencies

**Error Pattern:**

```
error: banned crate `banned-crate` found
error: crate `some-crate` is banned
```

**Solutions:**

1. **Review Banned Crates:**

   ```toml
   # In deny.toml
   [bans]
   multiple-versions = "warn"
   wildcards = "allow"

   # Remove or update banned crates
   deny = [
     # { name = "banned-crate" },  # Remove this line
   ]
   ```

2. **Add Exceptions:**

   ```toml
   # In deny.toml
   [[bans.skip]]
   name = "previously-banned-crate"
   version = "1.0"
   ```

### Multiple Version Issues

**Error Pattern:**

```
error: multiple versions of `some-crate` found
error: dependency `some-crate` has multiple versions
```

**Solutions:**

1. **Configure Multiple Version Policy:**

   ```toml
   # In deny.toml
   [bans]
   multiple-versions = "warn" # or "deny" for strict policy

   # Allow specific multiple versions
   [[bans.skip-tree]]
   name = "some-crate"
   version = "1.0"
   ```

2. **Resolve Version Conflicts:**

   ```bash
   # Find duplicate dependencies
   cargo tree --duplicates

   # Update to resolve conflicts
   cargo update
   ```

## SARIF Upload and Integration Issues

### GitHub Security Tab Upload

**Error Pattern:**

```
error: failed to upload SARIF file
error: SARIF upload rejected by GitHub
```

**Solutions:**

1. **Check SARIF Format:**

   ```bash
   # Validate SARIF file format
   jq . results.sarif > /dev/null

   # Check SARIF schema compliance
   # Use online validator or sarif-tools
   ```

2. **Verify GitHub Permissions:**

   ```yaml
   # In workflow file
   permissions:
     contents: read
     security-events: write  # Required for SARIF upload
   ```

3. **Upload SARIF Correctly:**

   ```yaml
   # In GitHub Actions workflow
     - name: Upload SARIF results
       uses: github/codeql-action/upload-sarif@v3
       with:
         sarif_file: results.sarif
         category: security-scan
   ```

### CodeQL Analysis Issues

**Error Pattern:**

```
error: CodeQL analysis failed
error: unable to create CodeQL database
```

**Solutions:**

1. **Check CodeQL Configuration:**

   ```yaml
   # In .github/workflows/security.yml
     - name: Initialize CodeQL
       uses: github/codeql-action/init@v3
       with:
         languages: rust
         config-file: ./.github/codeql/codeql-config.yml
   ```

2. **Build for CodeQL:**

   ```yaml
   # Ensure proper build for analysis
     - name: Autobuild
       uses: github/codeql-action/autobuild@v3

   # Or manual build
     - name: Build for CodeQL
       run: |
         cargo build --release
   ```

## SBOM Generation Issues

### Syft SBOM Generation

**Error Pattern:**

```
error: unable to generate SBOM
error: syft failed to analyze packages
```

**Solutions:**

1. **Install/Update Syft:**

   ```bash
   # Install syft
   curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin

   # Update syft
   syft version
   ```

2. **Generate SBOM:**

   ```bash
   # Generate CycloneDX SBOM
   syft packages . -o cyclonedx-json=sbom.json

   # Generate SPDX SBOM
   syft packages . -o spdx-json=sbom-spdx.json

   # Generate table output for inspection
   syft packages . -o table
   ```

3. **Validate SBOM:**

   ```bash
   # Check SBOM format
   jq . sbom.json > /dev/null

   # Inspect SBOM contents
   jq '.components[] | .name' sbom.json
   ```

## Security Policy Configuration

### cargo-deny Configuration

**Complete deny.toml example:**

```toml
# Security and license policy configuration

[graph]
targets = [
  { triple = "x86_64-unknown-linux-gnu" },
  { triple = "x86_64-pc-windows-msvc" },
  { triple = "x86_64-apple-darwin" },
]

[licenses]
allow = [
  "MIT",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unicode-DFS-2016",
]

# Deny copyleft licenses
deny = [
  "GPL-2.0",
  "GPL-3.0",
  "AGPL-1.0",
  "AGPL-3.0",
]

# License exceptions for specific crates (if needed)
# Note: Gold Digger uses rustls exclusively - no OpenSSL dependencies

[bans]
multiple-versions = "warn"
wildcards = "allow"

# Banned crates (security or policy reasons)
deny = [
  # Note: Gold Digger uses rustls exclusively - no OpenSSL dependencies
]

# Skip certain crates from bans
[[bans.skip]]
name = "windows-sys" # Multiple versions expected

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

# Ignore specific advisories (use sparingly)
ignore = [
  # "RUSTSEC-YYYY-NNNN",  # Example: ignore specific advisory
]
```

### Security Scanning Automation

**GitHub Actions security workflow:**

```yaml
name: Security Scan

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: 0 2 * * 1    # Weekly on Monday

jobs:
  security:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install security tools
        run: |
          cargo install cargo-audit --locked
          cargo install cargo-deny --locked
          curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh | sh -s -- -b /usr/local/bin
          curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin

      - name: Run cargo audit
        run: cargo audit --json > audit-results.json

      - name: Run cargo deny
        run: cargo deny check --format json > deny-results.json

      - name: Generate SBOM
        run: syft packages . -o cyclonedx-json=sbom.json

      - name: Run vulnerability scan
        run: grype . -o sarif > grype-results.sarif

      - name: Upload SARIF results
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: grype-results.sarif
          category: grype-scan
```

## Monitoring and Alerting

### Automated Security Monitoring

```bash
# Set up automated security checks
echo "0 2 * * 1 cd /path/to/project && just security" | crontab -

# Create security monitoring script
cat > security-monitor.sh << 'EOF'
#!/bin/bash
cd /path/to/gold_digger
just security
if [ $? -ne 0 ]; then
    echo "Security scan failed!" | mail -s "Security Alert" admin@example.com
fi
EOF
```

### Security Metrics Tracking

```bash
# Track vulnerability counts over time
cargo audit --json | jq '.vulnerabilities | length' > vuln-count.txt

# Track dependency counts
cargo tree --format "{p}" | wc -l > dep-count.txt

# Track license compliance
cargo deny check licenses --format json | jq '.licenses | length' > license-count.txt
```

## Prevention Strategies

### Regular Security Maintenance

```bash
# Weekly security updates
cargo update
just security

# Monthly dependency review
cargo outdated
cargo tree --duplicates

# Quarterly policy review
# Review and update deny.toml
# Update license allowlist
# Review banned crates list
```

### Security-First Development

```bash
# Pre-commit security checks
pre-commit install
# Add security hooks to .pre-commit-config.yaml

# Development workflow
just audit    # Before committing
just deny     # Check policies
just security # Full security scan
```

### Dependency Management

```bash
# Choose secure dependencies
cargo search --limit 5 crate-name
# Check crate security history
# Review crate maintenance status

# Pin critical dependencies
# Use exact versions for security-critical crates
# Regular security updates
```

## Getting Help

### Useful Commands

```bash
# Security scanning
just security          # Comprehensive security scan
cargo audit            # Security advisory check
cargo deny check       # Policy compliance check
grype .               # Vulnerability scanning

# Dependency analysis
cargo tree            # Dependency tree
cargo tree -i crate   # Reverse dependencies
cargo outdated        # Check for updates

# License checking
cargo license         # Show all licenses
cargo deny check licenses  # License compliance
```

### Resources

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-audit Documentation](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- [cargo-deny Documentation](https://embarkstudios.github.io/cargo-deny/)
- [Grype Documentation](https://github.com/anchore/grype)
- [Syft Documentation](https://github.com/anchore/syft)
- [GitHub Security Features](https://docs.github.com/en/code-security)

### Emergency Response

**Critical Vulnerability Response:**

1. **Immediate Assessment:**

   ```bash
   # Check vulnerability details
   cargo audit --json | jq '.vulnerabilities[]'

   # Find affected code paths
   cargo tree -i vulnerable-crate
   ```

2. **Quick Mitigation:**

   ```bash
   # Update vulnerable dependency
   cargo update -p vulnerable-crate

   # Or pin to secure version
   # Edit Cargo.toml with secure version
   ```

3. **Verification:**

   ```bash
   # Verify fix
   cargo audit
   just security

   # Test functionality
   just test
   ```

4. **Communication:**

   - Document the vulnerability and fix
   - Update security advisories
   - Notify stakeholders if needed
