---
name: warn-syft-usage
enabled: true
event: all
pattern: \bsyft\b
action: warn
---

**Do not use Syft for SBOM generation.**

This project uses `cargo cyclonedx --format json` instead. Syft scans the filesystem and picks up stale `Cargo.lock` files in `megalinter-reports/`, producing false positive vulnerabilities.

**Correct SBOM workflow:**

1. `cargo cyclonedx --format json` - generate SBOM from project's Cargo.lock only
2. `grype sbom:gold_digger.cdx.json` - scan for vulnerabilities
