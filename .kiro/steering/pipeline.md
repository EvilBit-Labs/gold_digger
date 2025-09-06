---
inclusion: always
---

# Gold Digger Development Pipeline Standards

## Quality Gates (REQUIRED Before Commits)

Run these commands before any commit:

```bash
just fmt-check    # cargo fmt --check (100-char line limit)
just lint         # cargo clippy -- -D warnings (ZERO tolerance)
just test         # cargo nextest run (preferred) or cargo test
just security     # cargo audit (advisory)
just deny-check   # cargo deny check --all-features (license & duplicate checks)
```

## Installation Requirements

### Development Tools

Install required development tools:

```bash
just install-tools  # Installs cargo-nextest, cargo-audit, cargo-deny, cargo-dist
```

Or install cargo-deny individually:

```bash
cargo install cargo-deny --locked
```

## Code Quality Standards

### Formatting & Linting

- **Line limit**: 100 characters (enforced by `rustfmt.toml`)
- **Clippy warnings**: Zero tolerance (`-D warnings`)
- **Error handling**: Use `anyhow` for applications, `thiserror` for libraries
- **Documentation**: Doc comments (`///`) required for all public functions

### Security & Compliance

- **cargo-audit**: Security vulnerability scanning
- **cargo-deny**: License compliance and duplicate dependency detection
  - Enforces allowed license list (MIT, Apache-2.0, BSD variants, etc.)
  - Warns about duplicate crate versions
  - Denies problematic licenses and sources
  - CI uses strict enforcement via `deny.ci.toml`
- **cargo-dist**: Cross-platform release artifacts
  - Generates SHA256 checksums and Cosign signatures
  - Uses github attestation for SBOM generation (CycloneDX format)

### Testing Requirements

- **Runner**: `cargo nextest run`
- **Coverage**: Target ≥80% with `cargo llvm-cov`
- **Cross-platform**: Must pass on macOS, Windows, Linux

## Essential Just Commands

```bash
just setup        # Install development dependencies
just fmt          # Auto-format code
just fmt-check    # Verify formatting (CI-compatible)
just lint         # Run clippy with -D warnings
just test         # Run tests
just deny-check   # Run cargo-deny checks (license & duplicates)
just ci-check     # Full CI validation locally
just build        # Build release artifacts
```

## Commit Standards

- **Format**: Conventional commits (`feat:`, `fix:`, `docs:`, etc.)
- **Scopes**: Use `(cli)`, `(db)`, `(output)`, `(tls)`, `(config)`
- **Automation**: cargo-dist handles versioning; git-cliff handles changelog

## CI/CD Pipeline

### GitHub Actions

- **ci.yml**: PR/push validation (format, lint, test, security, deny)
- **release.yml**: Cross-platform artifacts via cargo-dist (auto-generated, DO NOT EDIT)

### Release Requirements

1. All CI checks pass
2. No critical vulnerabilities
3. License compliance verified
4. Cross-platform binaries (x86_64/aarch64 for Linux, macOS, Windows)
5. SHA256 checksums and Cosign signatures
6. SBOM generation (CycloneDX format)

## Cargo-Deny Configuration

The project uses two cargo-deny configurations:

- **`deny.toml`**: Development configuration with warnings for yanked crates
- **`deny.ci.toml`**: CI configuration with strict enforcement

### Key Policies

- **Licenses**: Allows MIT, Apache-2.0, BSD variants, and other permissive licenses
- **Duplicate Detection**: Warns about multiple versions of the same crate
- **Sources**: Restricts to crates.io and approved Git repositories
- **Security**: Denies yanked crates in CI, warns in development
