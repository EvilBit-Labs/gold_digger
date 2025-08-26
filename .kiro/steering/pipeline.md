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
```

## Code Quality Standards

### Formatting & Linting

- **Line limit**: 100 characters (enforced by `rustfmt.toml`)
- **Clippy warnings**: Zero tolerance (`-D warnings`)
- **Error handling**: Use `anyhow` for applications, `thiserror` for libraries
- **Documentation**: Doc comments (`///`) required for all public functions

### Testing Requirements

- **Runner**: `cargo nextest run` (preferred) or `cargo test`
- **Coverage**: Target ≥80% with `cargo llvm-cov`
- **Cross-platform**: Must pass on macOS, Windows, Linux

## Essential Just Commands

```bash
just setup        # Install development dependencies
just fmt          # Auto-format code
just fmt-check    # Verify formatting (CI-compatible)
just lint         # Run clippy with -D warnings
just test         # Run tests
just ci-check     # Full CI validation locally
just build        # Build release artifacts
```

## Commit Standards

- **Format**: Conventional commits (`feat:`, `fix:`, `docs:`, etc.)
- **Scopes**: Use `(cli)`, `(db)`, `(output)`, `(tls)`, `(config)`
- **Automation**: cargo-dist handles versioning; git-cliff handles changelog

## CI/CD Pipeline

### GitHub Actions

- **ci.yml**: PR/push validation (format, lint, test, security)
- **release.yml**: Cross-platform artifacts via cargo-dist (auto-generated, DO NOT EDIT)

### Release Requirements

1. All CI checks pass
2. No critical vulnerabilities
3. Cross-platform binaries (x86_64/aarch64 for Linux, macOS, Windows)
4. SHA256 checksums and Cosign signatures
5. SBOM generation (CycloneDX format)
