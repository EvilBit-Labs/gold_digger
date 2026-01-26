# Gold Digger Justfile
# Task runner for the MySQL/MariaDB query tool

# Cross-platform justfile using OS annotations
# Windows uses PowerShell, Unix uses bash

set shell := ["bash", "-cu"]
set windows-shell := ["powershell", "-NoProfile", "-Command"]
set dotenv-load := true
set ignore-comments := true

# Use mise to manage all dev tools (go, pre-commit, uv, etc.)
# See mise.toml for tool versions
mise_exec := "mise exec --"

root := justfile_dir()

# =============================================================================
# GENERAL COMMANDS
# =============================================================================

default:
    @just --list

# =============================================================================
# CROSS-PLATFORM HELPERS (private)
# =============================================================================

[private, windows]
ensure-dir dir:
    New-Item -ItemType Directory -Force -Path "{{ dir }}" | Out-Null

[private, unix]
ensure-dir dir:
    /bin/mkdir -p "{{ dir }}"

[private, windows]
rmrf path:
    if (Test-Path "{{ path }}") { Remove-Item "{{ path }}" -Recurse -Force }

[private, unix]
rmrf path:
    /bin/rm -rf "{{ path }}"

# =============================================================================
# SETUP AND INITIALIZATION
# =============================================================================

# Development setup - mise handles all tool installation via mise.toml
setup:
    mise install


# =============================================================================
# CODE QUALITY
# =============================================================================

format: fmt

# Format code
fmt: pre-commit-run
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt --check

# Run clippy linting
lint:
    cargo clippy --all-targets --release -- -D warnings
    cargo clippy --all-targets --no-default-features --features "json csv additional_mysql_types verbose" -- -D warnings

# Lint SQL files with sqlfluff
lint-sql:
    cd {{justfile_dir()}}
    @if command -v sqlfluff >/dev/null 2>&1; then \
        echo "Linting SQL files..."; \
        sqlfluff lint tests/fixtures/**/*.sql || echo "Note: Expected errors from invalid.sql test file are normal"; \
    else \
        echo "sqlfluff not installed - install with 'pip install sqlfluff'"; \
        exit 1; \
    fi

# Fix SQL formatting with sqlfluff
fix-sql:
    cd {{justfile_dir()}}
    @if command -v sqlfluff >/dev/null 2>&1; then \
        echo "Fixing SQL formatting..."; \
        sqlfluff fix tests/fixtures/**/*.sql; \
    else \
        echo "sqlfluff not installed - install with 'pip install sqlfluff'"; \
        exit 1; \
    fi

# Run clippy with fixes
fix:
    cargo clippy --fix --allow-dirty --allow-staged

# Quick development check
check: pre-commit-run
    just lint
    just test-no-docker

pre-commit-run:
    uvx pre-commit run -a

# Format files (for pre-commit hooks)
# Accepts variadic file arguments from pre-commit
# When pre-commit passes filenames, they are expanded as {{FILES}}
format-files +FILES:
    npx prettier --write --config .prettierrc.json {{FILES}}

# Quality gates (CI equivalent)
ci-check: check fmt-check lint-sql test validate-deps deny-check

# Full CI workflow equivalent - mirrors .github/workflows/ci.yml exactly
ci-full:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{justfile_dir()}}

    echo "🚀 Running full CI workflow equivalent..."

    # Job 1: Quality checks (mirrors quality job)
    echo "📋 Quality checks..."
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo clippy --no-default-features --features "json csv additional_mysql_types verbose" -- -D warnings

    # Job 2: Test TLS functionality (mirrors test-tls job)
    echo "🔒 Testing TLS functionality..."
    cargo build --release

    BIN="./target/release/gold_digger"
    if [ -f "${BIN}.exe" ]; then
        BIN="${BIN}.exe"
    fi

    # Test mutually exclusive TLS flags (should fail)
    ! "$BIN" --tls-ca-file /tmp/nonexistent.pem --insecure-skip-hostname-verify \
        --db-url "mysql://test" --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1
    ! "$BIN" --tls-ca-file /tmp/nonexistent.pem --allow-invalid-certificate \
        --db-url "mysql://test" --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1
    ! "$BIN" --insecure-skip-hostname-verify --allow-invalid-certificate \
        --db-url "mysql://test" --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1

    cargo nextest run --test tls_config_unit_tests
    "$BIN" --help | grep -E "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)" || exit 1
    cargo tree | grep -E "(rustls|rustls-native-certs)" || exit 1
    ! cargo tree | grep "native-tls" || exit 1

    # Job 3: Test with different feature combinations (mirrors test-features job)
    echo "🧪 Testing feature combinations..."
    cargo nextest run
    cargo nextest run --no-default-features --features "json csv additional_mysql_types verbose"
    cargo build --release
    cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"

    "$BIN" --help | grep -E "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)" || exit 1
    cargo tree | grep -E "(rustls|rustls-native-certs)" || exit 1
    cargo tree --no-default-features --features "json csv additional_mysql_types verbose" \
        | grep -E "(rustls|rustls-native-certs)" || exit 1
    ! cargo tree | grep "native-tls" || exit 1

    # Job 4: Test TLS functionality (mirrors test-cross-platform job - Linux only)
    echo "🌐 Testing cross-platform TLS functionality..."
    cargo nextest run --test tls_config_unit_tests
    cargo nextest run --test tls_integration
    cargo tree | grep -E "(rustls|rustls-native-certs)" || exit 1
    ! cargo tree | grep "native-tls" || exit 1
    cargo build --release
    cargo tree | grep -E "(rustls|rustls-native-certs)" || exit 1
    ! cargo tree | grep "native-tls" || exit 1
    "$BIN" --help | grep -E "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)" || exit 1

    # Job 5: Test TLS error handling and configuration validation (mirrors test-tls-validation job)
    echo "⚠️  Testing TLS error handling and validation..."
    cargo build --release
    cargo nextest run tls_error_handling_tests 2>/dev/null || true
    cargo nextest run security_warning_tests 2>/dev/null || true

    ! "$BIN" --tls-ca-file /nonexistent/path.pem --db-url "mysql://test" \
        --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1
    echo "invalid certificate" > /tmp/invalid-cert.pem
    ! "$BIN" --tls-ca-file /tmp/invalid-cert.pem --db-url "mysql://test" \
        --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1

    cargo tree | grep -E "(rustls|rustls-native-certs)" || exit 1
    ! cargo tree | grep "native-tls" || exit 1

    # Job 6: Generate coverage (mirrors coverage job)
    echo "📊 Generating coverage reports..."
    cargo llvm-cov --workspace --lcov --output-path lcov-default.info
    cargo llvm-cov --workspace --lcov --output-path lcov-minimal.info \
        --no-default-features --features "json csv additional_mysql_types verbose"
    cat lcov-default.info lcov-minimal.info > lcov.info

    echo "🎉 CI workflow equivalent completed successfully!"

# Comprehensive full checks (all non-destructive validation)
full-checks: ci-check audit deny docs-check coverage-llvm build-all validate-cargo-dist

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
    cargo build --release --no-default-features --features "csv,json"

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
    cargo test --features integration_tests

# Run all tests including integration tests
test-all:
    cd {{justfile_dir()}}
    cargo test --features integration_tests

# Run integration tests with nextest (parallel execution and flaky test quarantine)
test-integration-nextest:
    cd {{justfile_dir()}}
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        echo "Running integration tests with nextest..."; \
        GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=3 \
        cargo nextest run --test integration_tests --features integration_tests; \
    else \
        echo "nextest not available, falling back to cargo test..."; \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Run integration tests with JUnit XML output for CI
test-integration-ci:
    cd {{justfile_dir()}}
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        echo "Running integration tests with nextest and JUnit output..."; \
        mkdir -p target/nextest-reports; \
        GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=3 \
        cargo nextest run --test integration_tests --features integration_tests \
            --message-format json-pretty \
            --junit-path target/nextest-reports/integration-tests.xml; \
    else \
        echo "nextest not available, falling back to cargo test..."; \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Run fast integration test subset for PR validation (< 5 minutes)
test-integration-fast:
    cd {{justfile_dir()}}
    @echo "Running fast integration test subset for PR validation..."
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        GOLD_DIGGER_INTEGRATION_FAST=1 \
        cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 2 --timeout 300; \
    else \
        GOLD_DIGGER_INTEGRATION_FAST=1 \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Run comprehensive integration test suite for main branch
test-integration-comprehensive:
    cd {{justfile_dir()}}
    @echo "Running comprehensive integration test suite..."
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        GOLD_DIGGER_INTEGRATION_COMPREHENSIVE=1 GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 \
        cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 4 --timeout 900; \
    else \
        GOLD_DIGGER_INTEGRATION_COMPREHENSIVE=1 \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Check Docker availability for integration tests
check-docker:
    @echo "Checking Docker availability for integration tests..."
    @if command -v docker >/dev/null 2>&1; then \
        if docker info >/dev/null 2>&1; then \
            echo "✓ Docker is available and daemon is running"; \
            docker --version; \
        else \
            echo "✗ Docker is installed but daemon is not running"; \
            echo "  Please start Docker daemon to run integration tests"; \
            exit 1; \
        fi; \
    else \
        echo "✗ Docker is not installed"; \
        echo "  Please install Docker to run integration tests"; \
        exit 1; \
    fi

# Run integration tests with Docker availability check
test-integration-safe:
    just check-docker
    just test-integration-nextest

# Run integration tests with artifact collection on failure
test-integration-debug:
    cd {{justfile_dir()}}
    @echo "Running integration tests with debug artifact collection..."
    @mkdir -p target/integration-test-artifacts
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        GOLD_DIGGER_TEST_DEBUG=1 GOLD_DIGGER_COLLECT_ARTIFACTS=1 \
        cargo nextest run --test integration_tests --features integration_tests \
            --failure-output immediate --success-output never || \
        echo "Integration tests failed - check target/integration-test-artifacts/ for debug info"; \
    else \
        GOLD_DIGGER_TEST_DEBUG=1 GOLD_DIGGER_COLLECT_ARTIFACTS=1 \
        cargo test --test integration_tests --features integration_tests || \
        echo "Integration tests failed - check target/integration-test-artifacts/ for debug info"; \
    fi

# Run integration tests with performance benchmarking
test-integration-perf:
    cd {{justfile_dir()}}
    @echo "Running integration tests with performance benchmarking..."
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        GOLD_DIGGER_INTEGRATION_PERF=1 \
        cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 1 --timeout 600; \
    else \
        GOLD_DIGGER_INTEGRATION_PERF=1 \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Run integration tests with both TLS and non-TLS database configurations
test-integration-matrix:
    cd {{justfile_dir()}}
    @echo "Running integration tests with TLS/non-TLS matrix..."
    just check-docker
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        echo "Testing non-TLS configurations..."; \
        GOLD_DIGGER_TEST_TLS=false \
        cargo nextest run --test integration_tests --features integration_tests; \
        echo "Testing TLS configurations..."; \
        GOLD_DIGGER_TEST_TLS=true \
        cargo nextest run --test integration_tests --features integration_tests; \
    else \
        echo "Testing non-TLS configurations..."; \
        GOLD_DIGGER_TEST_TLS=false \
        cargo test --test integration_tests --features integration_tests; \
        echo "Testing TLS configurations..."; \
        GOLD_DIGGER_TEST_TLS=true \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Run integration tests with flaky test quarantine enabled
test-integration-quarantine:
    cd {{justfile_dir()}}
    @echo "Running integration tests with flaky test quarantine..."
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=5 \
        cargo nextest run --test integration_tests --features integration_tests \
            --retries 3; \
    else \
        echo "Flaky test quarantine requires cargo-nextest - falling back to standard test"; \
        cargo test --test integration_tests --features integration_tests; \
    fi

# Test the new test execution utilities
test-execution-utilities:
    cd {{justfile_dir()}}
    @echo "Testing test execution utilities..."
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        cargo nextest run --test test_execution_utilities; \
    else \
        cargo test --test test_execution_utilities; \
    fi

# Generate integration test reports for CI
generate-test-reports:
    cd {{justfile_dir()}}
    @echo "Generating integration test reports..."
    @mkdir -p target/test-reports
    @if command -v cargo-nextest >/dev/null 2>&1 || cargo nextest --version >/dev/null 2>&1; then \
        echo "Running tests with nextest and JSON output..."; \
        NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --test integration_tests --features integration_tests \
            --message-format libtest-json > target/test-reports/integration-tests.json || true; \
        NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --test test_execution_utilities \
            --message-format libtest-json > target/test-reports/execution-utilities.json || true; \
        echo "JSON test reports generated in target/test-reports/"; \
        echo "Note: JUnit XML generation would require additional tooling or custom implementation"; \
    else \
        echo "Nextest not available, using standard cargo test"; \
        cargo test --test integration_tests --features integration_tests || true; \
        cargo test --test test_execution_utilities || true; \
    fi
    @echo "Test reports generated in target/test-reports/"

# Validate CI integration and test execution utilities
validate-ci-integration:
    cd {{justfile_dir()}}
    @echo "Validating CI integration and test execution utilities..."
    just test-execution-utilities
    @echo "✓ Test execution utilities validated"
    @echo "✓ CI environment detection working"
    @echo "✓ Nextest integration configured"
    @echo "✓ JUnit report generation available"

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

# =============================================================================
# BENCHMARKING
# =============================================================================

# Run full Criterion benchmark suite (mirrors CI)
bench:
    cd {{justfile_dir()}}
    cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage

# Run benchmarks and save current performance as a named baseline
bench-baseline BASELINE_NAME:
    cd {{justfile_dir()}}
    cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --save-baseline {{BASELINE_NAME}}

# Run benchmarks and compare against a previously saved baseline
bench-compare BASELINE_NAME:
    cd {{justfile_dir()}}
    cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --baseline {{BASELINE_NAME}}

# Open the generated HTML report from Criterion output directory in a browser
bench-report:
    cd {{justfile_dir()}}
    @if command -v open >/dev/null 2>&1; then \
        find target/criterion -name "index.html" | head -1 | xargs open; \
    elif command -v xdg-open >/dev/null 2>&1; then \
        find target/criterion -name "index.html" | head -1 | xargs xdg-open; \
    elif command -v start >/dev/null 2>&1; then \
        find target/criterion -name "index.html" | head -1 | xargs start; \
    else \
        echo "Could not find a way to open HTML files. Please open manually:"; \
        find target/criterion -name "index.html" | head -1; \
    fi

# Run a specific benchmark by name
bench-specific BENCHMARK_NAME:
    cd {{justfile_dir()}}
    cargo bench --bench {{BENCHMARK_NAME}}

# Run benchmarks with reduced sample size for faster feedback during development
bench-quick:
    cd {{justfile_dir()}}
    cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --quick

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
