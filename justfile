# Gold Digger Justfile
# Task runner for the MySQL/MariaDB query tool

# Use PowerShell for Windows targets
set windows-shell := ["powershell.exe", "-c"]

# Default recipe (runs linting)
default: lint

# Variables
export RUST_BACKTRACE := "1"
export CARGO_TERM_COLOR := "always"

# =============================================================================
# SETUP & INSTALLATION
# =============================================================================

# Development setup
setup:
    cd {{justfile_dir()}}
    rustup component add rustfmt clippy llvm-tools-preview rust-src
    cargo install cargo-nextest --locked || echo "cargo-nextest already installed"

# Install development tools (extended setup)
install-tools:
    cargo install cargo-llvm-cov --locked || echo "cargo-llvm-cov already installed"
    cargo install cargo-audit --locked || echo "cargo-audit already installed"
    cargo install cargo-deny --locked || echo "cargo-deny already installed"
    cargo install cargo-dist --locked || echo "cargo-dist already installed"

# Install mdBook and plugins for documentation
docs-install:
    cargo install mdbook mdbook-admonish mdbook-mermaid mdbook-linkcheck mdbook-toc mdbook-open-on-gh mdbook-tabs mdbook-i18n-helpers

# =============================================================================
# CODE QUALITY
# =============================================================================

format: fmt

# Format code
fmt:
    cd {{justfile_dir()}}
    pre-commit run -a || true
    cargo fmt
    prettier --write "**/*.{yml,yaml,js,jsx,ts,tsx}" 2>/dev/null || echo "prettier not installed - run 'npm install -g prettier'"

# Check formatting
fmt-check:
    cd {{justfile_dir()}}
    cargo fmt --check

# Run clippy linting
lint:
    cd {{justfile_dir()}}
    cargo clippy --all-targets --release -- -D warnings
    cargo clippy --all-targets --no-default-features --features "json csv additional_mysql_types verbose ssl-rustls" -- -D warnings

# Run clippy with fixes
fix:
    cargo clippy --fix --allow-dirty --allow-staged

# Quick development check
check:
    pre-commit run -a
    just lint
    just test-no-docker

# Quality gates (CI equivalent)
ci-check:
    cd {{justfile_dir()}}
    just fmt-check
    just lint
    just test
    just validate-deps
    just deny-check

# Comprehensive full checks (all non-destructive validation)
full-checks:
    cd {{justfile_dir()}}
    just fmt-check
    just lint
    just test
    just validate-deps
    just audit
    just deny
    just docs-check
    just coverage-llvm
    just build-all
    just validate-cargo-dist

# =============================================================================
# BUILD
# =============================================================================

# Build debug version
build:
    cd {{justfile_dir()}}
    cargo build

# Build release version
build-release:
    cargo build --release

# Build minimal version (no default features)
build-minimal:
    cargo build --release --no-default-features --features "csv,json,ssl-rustls"

# Build all feature combinations
build-all: build build-release build-minimal

# Install locally from workspace
install:
    cargo install --path .

# =============================================================================
# TESTING
# =============================================================================

# Run tests (prefer nextest, fallback to cargo test)
test:
    cd {{justfile_dir()}}
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        echo "Running tests with nextest..."; \
        cargo nextest run --run-ignored all; \
    else \
        echo "nextest not available, falling back to cargo test..."; \
        cargo test -- --include-ignored; \
    fi

# Run tests without Docker tests (non-ignored only)
test-no-docker:
    cd {{justfile_dir()}}
    cargo nextest run || cargo test

# Run integration tests (requires Docker)
test-integration:
    cd {{justfile_dir()}}
    cargo test --features integration_tests -- --ignored

# Run all tests including integration tests
test-all:
    cd {{justfile_dir()}}
    cargo test --features integration_tests -- --include-ignored

# Run tests with coverage (llvm-cov)
coverage:
    cd {{justfile_dir()}}
    cargo llvm-cov --package gold_digger --html

# Run tests with coverage (llvm-cov for CI)
coverage-llvm:
    cd {{justfile_dir()}}
    cargo llvm-cov --workspace --lcov --output-path lcov.info

# Coverage alias for CI naming consistency
cover: coverage-llvm

# Run coverage with threshold check (for CI)
coverage-ci:
    cd {{justfile_dir()}}
    cargo llvm-cov --package gold_digger --json --output-path coverage.json

# Benchmark (when criterion tests exist)
bench:
    cargo bench || echo "No benchmarks found"

# Profile release build
profile:
    cargo build --release

# =============================================================================
# SECURITY
# =============================================================================

# Security audit
audit:
    cargo audit

# Check for license/security issues (local development - tolerant)
deny:
    cargo deny check || echo "cargo-deny not installed - run 'just install-tools'"

# Check for license/security issues with all features
deny-check:
    cargo deny check || echo "cargo-deny not installed - run 'just install-tools'"

# Check for license/security issues (CI strict enforcement)
deny-ci:
    cargo deny check --config deny.ci.toml

# Comprehensive security scanning (combines audit, deny, and grype)
security:
    just audit
    just deny-ci
    @if command -v grype >/dev/null 2>&1; then \
    grype . --fail-on high || echo "High or critical vulnerabilities found"; \
    else \
    echo "grype not installed - install with:"; \
    echo "   curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh | sh -s -- -b /usr/local/bin"; \
    fi

# =============================================================================
# DEPENDENCIES & VALIDATION
# =============================================================================

# Validate TLS dependency tree
validate-deps:
    @echo "Validating TLS dependencies..."
    @if ! cargo tree -e=no-dev -f "{p} {f}" | grep -q "rustls"; then \
    echo "ERROR: rustls not found in standard build"; \
    cargo tree -e=no-dev -f "{p} {f}"; \
    exit 1; \
    fi
    @if cargo tree -e=no-dev -f "{p} {f}" | grep -q "native-tls"; then \
    echo "ERROR: native-tls found in build (should be rustls-only)"; \
    cargo tree -e=no-dev -f "{p} {f}"; \
    exit 1; \
    fi
    @echo "✓ Standard build includes rustls TLS support"
    @echo "✓ No native-tls dependencies found"
    @echo "✓ TLS validation passed"

# Check for outdated dependencies
outdated:
    cargo outdated || echo "Install cargo-outdated: cargo install cargo-outdated"

# Update dependencies
update:
    cargo update

# =============================================================================
# DOCUMENTATION
# =============================================================================

# Build complete documentation (mdBook + rustdoc)
docs-build:
    #!/usr/bin/env bash
    set -euo pipefail
    # Build rustdoc
    cargo doc --no-deps --document-private-items --target-dir docs/book/api-temp
    # Move rustdoc output to final location
    mkdir -p docs/book/api
    cp -r docs/book/api-temp/doc/* docs/book/api/
    rm -rf docs/book/api-temp
    # Build mdBook
    cd docs && mdbook build

# Serve documentation locally with live reload
docs-serve:
    cd docs && mdbook serve --open

# Clean documentation artifacts
docs-clean:
    rm -rf docs/book target/doc

# Check documentation (build + link validation + formatting)
docs-check:
    cd docs && mdbook build
    @just fmt-check

# Generate and serve documentation
[unix]
docs:
    cd docs && mdbook serve --open

[windows]
docs:
    @echo "mdbook requires a Unix-like environment to serve"

# =============================================================================
# RUNNING & DEVELOPMENT
# =============================================================================

# Run with example environment variables
run OUTPUT_FILE DATABASE_URL DATABASE_QUERY:
    OUTPUT_FILE={{OUTPUT_FILE}} DATABASE_URL={{DATABASE_URL}} DATABASE_QUERY={{DATABASE_QUERY}} cargo run --release

# Run with safe example (casting to avoid panics)
run-safe:
    DB_URL=sqlite://dummy.db API_KEY=dummy NODE_ENV=testing APP_ENV=safe cargo run --release

# Development server (watch for changes) - requires cargo-watch
watch:
    cargo watch -x "run --release" || echo "Install cargo-watch: cargo install cargo-watch"

# =============================================================================
# UTILITIES & INFORMATION
# =============================================================================

# Show feature matrix
features:
    @echo "Available feature combinations:"
    @echo ""
    @echo "Standard build:"
    @echo "  cargo build --release"
    @echo ""
    @echo "Minimal build (fewer features):"
    @echo "  cargo build --no-default-features --features \"csv json\""
    @echo ""
    @echo "All MySQL types:"
    @echo "  cargo build --release --features \"default additional_mysql_types\""

# Check current version
version:
    @echo "Current version information:"
    @echo "Cargo.toml version: $(grep '^version' Cargo.toml | cut -d'"' -f2)"
    @echo "CHANGELOG.md version: $(grep -m1 '## \[v' CHANGELOG.md | sed 's/.*\[v/v/' | sed 's/\].*//')"
    @echo ""
    @echo "Note: Versions may be out of sync - check WARP.md for details"

# Show project status
status:
    @echo "Gold Digger Project Status:"
    @echo ""
    @echo "Architecture: Environment variable driven, structured output"
    @echo "Current: v0.2.6 (check version discrepancy)"
    @echo "Target: v1.0 with CLI interface"
    @echo "Maintainer: EvilBit-Labs"
    @echo ""
    @echo "Critical Issues:"
    @echo "  • Type conversion panics on NULL/non-string values"
    @echo "  • No dotenv support (use exported env vars)"
    @echo "  • Non-deterministic JSON output"
    @echo "  • Pattern matching bug in src/main.rs:59"
    @echo ""
    @echo "cargo-dist: Automated versioning and distribution enabled"
    @echo "See WARP.md for detailed information"

# Clean build artifacts
clean:
    cargo clean

# =============================================================================
# SBOM & SECURITY
# =============================================================================

# Generate Software Bill of Materials (SBOM) for local inspection
sbom:
    @if command -v cargo-cyclonedx >/dev/null 2>&1 || cargo cyclonedx --help >/dev/null 2>&1; then \
    cargo cyclonedx --override-filename sbom.json; \
    cargo tree --format "{p} {f}" | head -20; \
    elif command -v syft >/dev/null 2>&1; then \
    syft packages . -o cyclonedx-json=sbom.json; \
    syft packages . -o table; \
    else \
    echo "Neither cargo-cyclonedx nor syft installed"; \
    echo ""; \
    echo "Install cargo-cyclonedx (preferred):"; \
    echo "   cargo install cargo-cyclonedx"; \
    echo ""; \
    echo "Or install syft:"; \
    echo "   curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin"; \
    echo ""; \
    echo "Alternative: Use cargo tree for dependency inspection:"; \
    cargo tree --format "{p} {f}"; \
    fi

# =============================================================================
# CARGO-DIST & DISTRIBUTION
# =============================================================================

# Initialize cargo-dist configuration
dist-init:
    @echo "Initializing cargo-dist configuration..."
    @if command -v cargo-dist >/dev/null 2>&1; then \
    echo "Running cargo-dist init..."; \
    cargo dist init --yes; \
    echo "cargo-dist initialized successfully"; \
    echo "Configuration written to cargo-dist.toml"; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    exit 1; \
    fi

# Plan cargo-dist release (dry-run)
dist-plan:
    @if command -v cargo-dist >/dev/null 2>&1; then \
    cargo dist plan; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    exit 1; \
    fi

# Build cargo-dist artifacts locally
dist-build:
    @if command -v cargo-dist >/dev/null 2>&1; then \
    cargo dist build; \
    find target/distrib -type f -name "*" | head -10 || echo "  (no artifacts found)"; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    exit 1; \
    fi

# Generate cargo-dist installers
dist-generate:
    @if command -v cargo-dist >/dev/null 2>&1; then \
    cargo dist generate; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    exit 1; \
    fi

# Validate cargo-dist configuration
dist-check:
    @if command -v cargo-dist >/dev/null 2>&1; then \
    if cargo dist plan >/dev/null 2>&1; then \
    echo "cargo-dist configuration check passed"; \
    else \
    echo "cargo-dist configuration check failed"; \
    exit 1; \
    fi; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    exit 1; \
    fi

# Validate cargo-dist configuration
validate-cargo-dist:
    @test -f dist-workspace.toml && echo "dist-workspace.toml exists" || echo "Missing: dist-workspace.toml"
    @if command -v cargo-dist >/dev/null 2>&1; then \
    if cargo dist plan >/dev/null 2>&1; then \
    echo "cargo-dist configuration is valid"; \
    else \
    echo "cargo-dist configuration is invalid"; \
    exit 1; \
    fi; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    fi

# =============================================================================
# ACT & GITHUB ACTIONS TESTING
# =============================================================================

# Local GitHub Actions Testing (requires act)
act-setup:
    @which act || echo "Please install act: brew install act (or see https://github.com/nektos/act)"
    docker pull catthehacker/ubuntu:act-22.04 || echo "Could not pull Docker image - act may not work without it"

# Run CI workflow locally (dry-run)
act-ci-dry:
    act -W .github/workflows/ci.yml --dryrun

# Run CI workflow locally (full execution)
act-ci:
    act -W .github/workflows/ci.yml

# Run push workflow locally (dry-run)
act-push-dry:
    act push --dryrun

# Run push workflow locally (full execution)
act-push:
    act push

# Run release workflow dry-run (requires tag parameter)
act-release-dry TAG:
    @echo "Running release workflow dry-run for tag: {{TAG}}"
    @echo "This simulates the full release pipeline without actually creating releases"
    act push --input tag={{TAG}} -W .github/workflows/release.yml --dryrun

# Test cargo-dist workflow locally
act-cargo-dist-dry:
    @echo "Running cargo-dist workflow dry-run..."
    @echo "This simulates the cargo-dist workflow without creating releases"
    @if command -v cargo-dist >/dev/null 2>&1; then \
    echo "Running cargo-dist plan..."; \
    cargo dist plan; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    fi

# Test cargo-dist with sample conventional commits
act-cargo-dist-test:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "feat: add new output format support" > test-commit-feat.txt
    echo "fix: resolve connection timeout issue" > test-commit-fix.txt
    echo "docs: update README with new examples" > test-commit-docs.txt
    echo "feat!: migrate to new CLI interface" > test-commit-breaking.txt

# Test cargo-dist integration with release workflow
act-cargo-dist-integration TAG:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v cargo-dist >/dev/null 2>&1; then \
    cargo dist plan; \
    else \
    echo "cargo-dist not installed - run 'just install-tools' first"; \
    fi
    act workflow_dispatch --input tag={{TAG}} -W .github/workflows/release.yml --dryrun

# List all available GitHub Actions workflows
act-list:
    act --list

# Test specific workflow job
act-job JOB:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{justfile_dir()}}
    act -j {{JOB}} --dryrun

# Clean act cache and containers
act-clean:
    -docker ps -a | grep "act-" | awk '{print $1}' | xargs docker rm -f
    -docker images | grep "act-" | awk '{print $3}' | xargs docker rmi -f

# =============================================================================
# RELEASE & VALIDATION
# =============================================================================

# Release preparation checklist
release-check:
    just ci-check
    just audit
    just build-all
    just act-ci-dry
    just dist-plan
    just act-cargo-dist-integration v0.2.7

# Release simulation for local testing
[unix]
release-dry:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! git diff-index --quiet HEAD --; then
    echo "Warning: Working directory has uncommitted changes"
    fi
    just build-release
    BINARY_PATH="target/release/gold_digger"
    if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Binary not found at $BINARY_PATH"
    exit 1
    fi
    if command -v syft >/dev/null 2>&1; then
    syft packages . -o cyclonedx-json=sbom-test.json
    else
    echo '{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}' > sbom-test.json
    fi
    if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$BINARY_PATH" > checksums-test.txt
    sha256sum sbom-test.json >> checksums-test.txt
    elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$BINARY_PATH" > checksums-test.txt
    shasum -a 256 sbom-test.json >> checksums-test.txt
    else
    touch checksums-test.txt
    fi

[windows]
release-dry:
    just build-release
    $BINARY_PATH = "target\release\gold_digger.exe"
    if (-not (Test-Path $BINARY_PATH)) {
        Write-Error "Binary not found at $BINARY_PATH"
        exit 1
    }
    if (Get-Command syft -ErrorAction SilentlyContinue) {
        syft packages . -o cyclonedx-json=sbom-test.json
    } else {
        '{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}' | Out-File -FilePath sbom-test.json -Encoding UTF8
    }
    (Get-FileHash -Path $BINARY_PATH -Algorithm SHA256).Hash | Out-File -FilePath checksums-test.txt
    (Get-FileHash -Path sbom-test.json -Algorithm SHA256).Hash | Add-Content -Path checksums-test.txt

# =============================================================================
# HELP & DOCUMENTATION
# =============================================================================

# Show help
help:
    @echo "Gold Digger Justfile Commands:"
    @echo ""
    @echo "Development:"
    @echo "  setup          Set up development environment"
    @echo "  install-tools  Install additional development tools"
    @echo "  build         Build debug version"
    @echo "  build-release Build release version"
    @echo "  build-all     Build all feature combinations"
    @echo "  install       Install locally from workspace"
    @echo ""
    @echo "Code Quality:"
    @echo "  format           Format code"
    @echo "  fmt-check     Check formatting"
    @echo "  lint          Run clippy linting"
    @echo "  fix           Run clippy with automatic fixes"
    @echo "  check         Quick development checks"
    @echo "  ci-check      Full CI equivalent checks"
    @echo "  full-checks   Comprehensive validation (all non-destructive checks)"
    @echo "  deny-check    Run cargo-deny checks (license & duplicates)"
    @echo ""
    @echo "Testing:"
    @echo "  test          Run tests with nextest (including ignored Docker tests)"
    @echo "  test-no-docker Run tests with nextest (excluding Docker tests)"
    @echo "  test-integration Run integration tests (requires Docker)"
    @echo "  test-all      Run all tests including integration tests"
    @echo "  coverage      Run tests with coverage report"
    @echo "  coverage-llvm Run tests with llvm-cov (CI compatible)"
    @echo "  cover         Alias for coverage-llvm (CI naming consistency)"
    @echo "  bench         Run benchmarks"
    @echo ""
    @echo "Security:"
    @echo "  audit         Security audit"
    @echo "  deny          License and security checks"
    @echo "  deny-check    License and security checks with all features"
    @echo "  security      Comprehensive security scanning (audit + deny + grype)"
    @echo "  sbom          Generate Software Bill of Materials for inspection"
    @echo "  validate-deps Validate TLS dependency tree"
    @echo ""
    @echo "Running:"
    @echo "  run OUTPUT_FILE DATABASE_URL DATABASE_QUERY  Run with custom env vars"
    @echo "  run-safe      Run with safe example query"
    @echo "  watch         Watch for changes (requires cargo-watch)"
    @echo ""
    @echo "Local GitHub Actions Testing (requires act):"
    @echo "  act-setup     Set up act and pull Docker images"
    @echo "  act-ci-dry    Run CI workflow dry-run (simulation)"
    @echo "  act-ci        Run CI workflow locally (full execution)"
    @echo "  act-release-dry TAG  Simulate release workflow for tag"
    @echo "  act-cargo-dist-dry  Simulate cargo-dist workflow"
    @echo "  act-cargo-dist-test  Test with sample conventional commits"
    @echo "  act-cargo-dist-integration TAG  Test cargo-dist + release integration"
    @echo "  act-list      List all available workflows"
    @echo "  act-job JOB   Test specific workflow job"
    @echo "  act-clean     Clean act cache and containers"
    @echo ""
    @echo "Documentation:"
    @echo "  docs-install  Install mdBook and plugins"
    @echo "  docs-build    Build complete documentation (mdBook + rustdoc)"
    @echo "  docs-serve    Serve documentation locally with live reload"
    @echo "  docs-clean    Clean documentation artifacts"
    @echo "  docs-check    Check documentation (build + validation + formatting)"
    @echo "  docs          Generate and open rustdoc only"
    @echo ""
    @echo "Maintenance:"
    @echo "  clean         Clean build artifacts"
    @echo "  outdated      Check for outdated dependencies"
    @echo "  update        Update dependencies"
    @echo "  features      Show available feature combinations"
    @echo "  version       Show version information"
    @echo "  status        Show project status and critical issues"
    @echo ""
    @echo "Release:"
    @echo "  release-check Pre-release checklist and validation"
    @echo "  release-dry   Simulate release process locally"
    @echo "  validate-cargo-dist  Validate cargo-dist configuration"
    @echo ""
    @echo "Distribution (cargo-dist):"
    @echo "  dist-init     Initialize cargo-dist configuration"
    @echo "  dist-plan     Plan cargo-dist release (dry-run)"
    @echo "  dist-build    Build cargo-dist artifacts locally"
    @echo "  dist-generate Generate cargo-dist installers"
    @echo "  dist-check    Validate cargo-dist configuration"
    @echo ""
    @echo "For detailed project information, see WARP.md, AGENTS.md, or .cursor/rules/"
