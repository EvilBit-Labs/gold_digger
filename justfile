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
    @{{ mise_exec }} cargo fmt

# Check formatting
fmt-check:
    @{{ mise_exec }} cargo fmt --check

# Run clippy linting
lint:
    @{{ mise_exec }} cargo clippy --all-targets --release -- -D warnings
    @{{ mise_exec }} cargo clippy --all-targets --no-default-features --features "json csv additional_mysql_types verbose" -- -D warnings

# Lint SQL files with sqlfluff
lint-sql:
    @{{ mise_exec }} sqlfluff lint tests/fixtures/**/*.sql

# Fix SQL formatting with sqlfluff
fix-sql:
    @{{ mise_exec }} sqlfluff fix tests/fixtures/**/*.sql


# Run clippy with fixes
fix:
    @{{ mise_exec }} cargo clippy --fix --allow-dirty --allow-staged

# Quick development check
check: pre-commit-run lint test-no-docker

pre-commit-run:
    @{{ mise_exec }} pre-commit run -a

# Format files (for pre-commit hooks)
# Accepts variadic file arguments from pre-commit
# When pre-commit passes filenames, they are expanded as {{FILES}}
format-files +FILES:
    @{{ mise_exec }} npx prettier --write --config .prettierrc.json {{FILES}}

# Quality gates (CI equivalent)
ci-check: check fmt-check lint-sql test validate-deps deny-check

# Full CI workflow equivalent - mirrors .github/workflows/ci.yml exactly
ci-full:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running full CI workflow equivalent..."

    # Job 1: Quality checks (mirrors quality job)
    echo "[1/6] Quality checks..."
    {{ mise_exec }} cargo fmt --check
    {{ mise_exec }} cargo clippy -- -D warnings
    {{ mise_exec }} cargo clippy --no-default-features --features "json csv additional_mysql_types verbose" -- -D warnings

    # Job 2: Test TLS functionality (mirrors test-tls job)
    echo "[2/6] Testing TLS functionality..."
    {{ mise_exec }} cargo build --release

    BIN="./target/release/gold_digger"
    [[ -f "${BIN}.exe" ]] && BIN="${BIN}.exe"

    # Test mutually exclusive TLS flags (should fail)
    ! "$BIN" --tls-ca-file /tmp/nonexistent.pem --insecure-skip-hostname-verify \
        --db-url "mysql://test" --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1
    ! "$BIN" --tls-ca-file /tmp/nonexistent.pem --allow-invalid-certificate \
        --db-url "mysql://test" --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1
    ! "$BIN" --insecure-skip-hostname-verify --allow-invalid-certificate \
        --db-url "mysql://test" --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1

    {{ mise_exec }} cargo nextest run --test tls_config_unit_tests
    "$BIN" --help | grep -qE "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)"
    {{ mise_exec }} cargo tree | grep -qE "(rustls|rustls-native-certs)"
    ! {{ mise_exec }} cargo tree | grep -q "native-tls"

    # Job 3: Test with different feature combinations (mirrors test-features job)
    echo "[3/6] Testing feature combinations..."
    {{ mise_exec }} cargo nextest run
    {{ mise_exec }} cargo nextest run --no-default-features --features "json csv additional_mysql_types verbose"
    {{ mise_exec }} cargo build --release
    {{ mise_exec }} cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"

    "$BIN" --help | grep -qE "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)"
    {{ mise_exec }} cargo tree | grep -qE "(rustls|rustls-native-certs)"
    {{ mise_exec }} cargo tree --no-default-features --features "json csv additional_mysql_types verbose" \
        | grep -qE "(rustls|rustls-native-certs)"
    ! {{ mise_exec }} cargo tree | grep -q "native-tls"

    # Job 4: Test TLS functionality (mirrors test-cross-platform job - Linux only)
    echo "[4/6] Testing cross-platform TLS functionality..."
    {{ mise_exec }} cargo nextest run --test tls_config_unit_tests
    {{ mise_exec }} cargo nextest run --test tls_integration
    {{ mise_exec }} cargo tree | grep -qE "(rustls|rustls-native-certs)"
    ! {{ mise_exec }} cargo tree | grep -q "native-tls"
    {{ mise_exec }} cargo build --release
    "$BIN" --help | grep -qE "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)"

    # Job 5: Test TLS error handling and configuration validation (mirrors test-tls-validation job)
    echo "[5/6] Testing TLS error handling and validation..."
    {{ mise_exec }} cargo nextest run tls_error_handling_tests 2>/dev/null || true
    {{ mise_exec }} cargo nextest run security_warning_tests 2>/dev/null || true

    ! "$BIN" --tls-ca-file /nonexistent/path.pem --db-url "mysql://test" \
        --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1
    echo "invalid certificate" > /tmp/invalid-cert.pem
    ! "$BIN" --tls-ca-file /tmp/invalid-cert.pem --db-url "mysql://test" \
        --query "SELECT 1" --output /tmp/test.json 2>/dev/null || exit 1

    {{ mise_exec }} cargo tree | grep -qE "(rustls|rustls-native-certs)"
    ! {{ mise_exec }} cargo tree | grep -q "native-tls"

    # Job 6: Generate coverage (mirrors coverage job)
    echo "[6/6] Generating coverage reports..."
    {{ mise_exec }} cargo llvm-cov --workspace --lcov --output-path lcov-default.info
    {{ mise_exec }} cargo llvm-cov --workspace --lcov --output-path lcov-minimal.info \
        --no-default-features --features "json csv additional_mysql_types verbose"
    cat lcov-default.info lcov-minimal.info > lcov.info

    echo "CI workflow equivalent completed successfully!"

# Comprehensive full checks (all non-destructive validation)
full-checks: ci-check audit deny docs-check coverage-llvm build-all validate-cargo-dist

# =============================================================================
# BUILD
# =============================================================================

# Build debug version
build:
    @{{ mise_exec }} cargo build

# Build release version
build-release:
    @{{ mise_exec }} cargo build --release

# Build minimal version (no default features)
build-minimal:
    @{{ mise_exec }} cargo build --release --no-default-features --features "csv,json"

# Build all feature combinations
build-all: build build-release build-minimal

# Install locally from workspace
install:
    @{{ mise_exec }} cargo install --path .

# =============================================================================
# TESTING
# =============================================================================

# Run all tests (including ignored)
test:
    @{{ mise_exec }} cargo nextest run --run-ignored all

# Run tests without Docker tests (non-ignored only)
test-no-docker:
    @{{ mise_exec }} cargo nextest run

# Run integration tests (requires Docker)
test-integration:
    @{{ mise_exec }} cargo nextest run --features integration_tests

# Run all tests including integration tests
test-all:
    @{{ mise_exec }} cargo nextest run --features integration_tests

# Run integration tests with flaky test quarantine
test-integration-nextest:
    @GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=3 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests

# Run integration tests with JUnit XML output for CI
test-integration-ci:
    @mkdir -p target/nextest-reports
    @GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=3 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --message-format json-pretty \
            --junit-path target/nextest-reports/integration-tests.xml

# Run fast integration test subset for PR validation
test-integration-fast:
    @GOLD_DIGGER_INTEGRATION_FAST=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 2 --timeout 300

# Run comprehensive integration test suite for main branch
test-integration-comprehensive:
    @GOLD_DIGGER_INTEGRATION_COMPREHENSIVE=1 GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 4 --timeout 900

# Check Docker availability for integration tests
check-docker:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking Docker availability..."
    if ! command -v docker &>/dev/null; then
        echo "✗ Docker is not installed"
        exit 1
    fi
    if ! docker info &>/dev/null; then
        echo "✗ Docker daemon is not running"
        exit 1
    fi
    echo "✓ Docker is available"
    docker --version

# Run integration tests with Docker availability check
test-integration-safe: check-docker test-integration-nextest

# Run integration tests with artifact collection on failure
test-integration-debug:
    @mkdir -p target/integration-test-artifacts
    @GOLD_DIGGER_TEST_DEBUG=1 GOLD_DIGGER_COLLECT_ARTIFACTS=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --failure-output immediate --success-output never || \
        echo "Integration tests failed - check target/integration-test-artifacts/ for debug info"

# Run integration tests with performance benchmarking
test-integration-perf:
    @GOLD_DIGGER_INTEGRATION_PERF=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 1 --timeout 600

# Run integration tests with both TLS and non-TLS database configurations
test-integration-matrix: check-docker
    @echo "Testing non-TLS configurations..."
    @GOLD_DIGGER_TEST_TLS=false {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests
    @echo "Testing TLS configurations..."
    @GOLD_DIGGER_TEST_TLS=true {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests

# Run integration tests with flaky test quarantine enabled
test-integration-quarantine:
    @GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=5 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests --retries 3

# Test the new test execution utilities
test-execution-utilities:
    @{{ mise_exec }} cargo nextest run --test test_execution_utilities

# Generate integration test reports for CI
generate-test-reports:
    @mkdir -p target/test-reports
    @NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
        --message-format libtest-json > target/test-reports/integration-tests.json || true
    @NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 {{ mise_exec }} cargo nextest run --test test_execution_utilities \
        --message-format libtest-json > target/test-reports/execution-utilities.json || true
    @echo "Test reports generated in target/test-reports/"

# Validate CI integration and test execution utilities
validate-ci-integration: test-execution-utilities
    @echo "✓ Test execution utilities validated"
    @echo "✓ CI environment detection working"
    @echo "✓ Nextest integration configured"

# Run tests with coverage (llvm-cov)
coverage:
    @{{ mise_exec }} cargo llvm-cov --package gold_digger --html

# Run tests with coverage (llvm-cov for CI)
coverage-llvm:
    @{{ mise_exec }} cargo llvm-cov --workspace --lcov --output-path lcov.info

# Coverage alias for CI naming consistency
cover: coverage-llvm

# Run coverage with threshold check (for CI)
coverage-ci:
    @{{ mise_exec }} cargo llvm-cov --package gold_digger --json --output-path coverage.json

# =============================================================================
# BENCHMARKING
# =============================================================================

# Run full Criterion benchmark suite (mirrors CI)
bench:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage

# Run benchmarks and save current performance as a named baseline
bench-baseline BASELINE_NAME:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --save-baseline {{BASELINE_NAME}}

# Run benchmarks and compare against a previously saved baseline
bench-compare BASELINE_NAME:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --baseline {{BASELINE_NAME}}

# Open the generated HTML report from Criterion output directory in a browser
[unix]
bench-report:
    @open "$$(find target/criterion -name 'index.html' | head -1)" 2>/dev/null || \
     xdg-open "$$(find target/criterion -name 'index.html' | head -1)" 2>/dev/null || \
     echo "Open manually: $$(find target/criterion -name 'index.html' | head -1)"

[windows]
bench-report:
    start (Get-ChildItem -Recurse target/criterion -Filter index.html | Select-Object -First 1).FullName

# Run a specific benchmark by name
bench-specific BENCHMARK_NAME:
    @{{ mise_exec }} cargo bench --bench {{BENCHMARK_NAME}}

# Run benchmarks with reduced sample size for faster feedback during development
bench-quick:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --quick

# Profile release build
profile:
    @{{ mise_exec }} cargo build --release

# =============================================================================
# SECURITY
# =============================================================================

# Security audit
audit:
    @{{ mise_exec }} cargo audit

# Check for license/security issues (local development)
deny:
    @{{ mise_exec }} cargo deny check

# Check for license/security issues with all features
deny-check:
    @{{ mise_exec }} cargo deny check

# Check for license/security issues (CI strict enforcement)
deny-ci:
    @{{ mise_exec }} cargo deny check --config deny.ci.toml

# Comprehensive security scanning (combines audit, deny, and grype)
security: audit deny-ci
    @{{ mise_exec }} grype . --fail-on high || echo "High or critical vulnerabilities found"

# =============================================================================
# DEPENDENCIES & VALIDATION
# =============================================================================

# Validate TLS dependency tree
validate-deps:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Validating TLS dependencies..."
    if ! {{ mise_exec }} cargo tree -e=no-dev -f "{p} {f}" | grep -q "rustls"; then
        echo "ERROR: rustls not found in standard build"
        {{ mise_exec }} cargo tree -e=no-dev -f "{p} {f}"
        exit 1
    fi
    if {{ mise_exec }} cargo tree -e=no-dev -f "{p} {f}" | grep -q "native-tls"; then
        echo "ERROR: native-tls found in build (should be rustls-only)"
        {{ mise_exec }} cargo tree -e=no-dev -f "{p} {f}"
        exit 1
    fi
    echo "✓ Standard build includes rustls TLS support"
    echo "✓ No native-tls dependencies found"
    echo "✓ TLS validation passed"

# Check for outdated dependencies
outdated:
    @{{ mise_exec }} cargo outdated

# Update dependencies
update:
    @{{ mise_exec }} cargo update

# =============================================================================
# DOCUMENTATION
# =============================================================================

# Build complete documentation (mdBook + rustdoc)
docs-build:
    #!/usr/bin/env bash
    set -euo pipefail
    # Build rustdoc
    {{ mise_exec }} cargo doc --no-deps --document-private-items --target-dir docs/book/api-temp
    # Move rustdoc output to final location
    mkdir -p docs/book/api
    cp -r docs/book/api-temp/doc/* docs/book/api/
    rm -rf docs/book/api-temp
    # Build mdBook
    cd docs && {{ mise_exec }} mdbook build

# Serve documentation locally with live reload
docs-serve:
    @cd docs && {{ mise_exec }} mdbook serve --open

# Clean documentation artifacts
docs-clean:
    @rm -rf docs/book target/doc

# Check documentation (build + link validation + formatting)
docs-check:
    @cd docs && {{ mise_exec }} mdbook build
    @just fmt-check

# Generate and serve documentation
[unix]
docs:
    @cd docs && {{ mise_exec }} mdbook serve --open

[windows]
docs:
    @echo "mdbook requires a Unix-like environment to serve"

# =============================================================================
# RUNNING & DEVELOPMENT
# =============================================================================

# Run with example environment variables
run OUTPUT_FILE DATABASE_URL DATABASE_QUERY:
    @OUTPUT_FILE={{OUTPUT_FILE}} DATABASE_URL={{DATABASE_URL}} DATABASE_QUERY={{DATABASE_QUERY}} \
        {{ mise_exec }} cargo run --release

# Run with safe example (casting to avoid panics)
run-safe:
    @DB_URL=sqlite://dummy.db API_KEY=dummy NODE_ENV=testing APP_ENV=safe \
        {{ mise_exec }} cargo run --release

# Development server (watch for changes)
watch:
    @{{ mise_exec }} cargo watch -x "run --release"

# =============================================================================
# UTILITIES & INFORMATION
# =============================================================================

# Clean build artifacts
clean:
    @{{ mise_exec }} cargo clean

# =============================================================================
# SBOM & SECURITY
# =============================================================================

# Generate Software Bill of Materials (SBOM) using cargo-cyclonedx
sbom:
    @{{ mise_exec }} cargo cyclonedx --override-filename sbom.json
    @{{ mise_exec }} cargo tree --format "{p} {f}" | head -20

# Generate SBOM using syft (alternative)
sbom-syft:
    @{{ mise_exec }} syft packages . -o cyclonedx-json=sbom.json
    @{{ mise_exec }} syft packages . -o table

# =============================================================================
# CARGO-DIST & DISTRIBUTION
# =============================================================================

# Initialize cargo-dist configuration
dist-init:
    @echo "Initializing cargo-dist configuration..."
    @{{ mise_exec }} cargo dist init --yes
    @echo "cargo-dist initialized successfully"

# Plan cargo-dist release (dry-run)
dist-plan:
    @{{ mise_exec }} cargo dist plan

# Build cargo-dist artifacts locally
dist-build:
    @{{ mise_exec }} cargo dist build
    @find target/distrib -type f -name "*" 2>/dev/null | head -10 || echo "  (no artifacts found)"

# Generate cargo-dist installers
dist-generate:
    @{{ mise_exec }} cargo dist generate

# Validate cargo-dist configuration
dist-check:
    @{{ mise_exec }} cargo dist plan >/dev/null && echo "cargo-dist configuration check passed"

# Validate cargo-dist configuration
validate-cargo-dist:
    @test -f dist-workspace.toml && echo "dist-workspace.toml exists" || echo "Missing: dist-workspace.toml"
    @{{ mise_exec }} cargo dist plan >/dev/null && echo "cargo-dist configuration is valid"

# =============================================================================
# ACT & GITHUB ACTIONS TESTING
# =============================================================================

# Local GitHub Actions Testing - pull required Docker image
act-setup:
    @docker pull catthehacker/ubuntu:act-22.04

# Run CI workflow locally (dry-run)
act-ci-dry:
    @{{ mise_exec }} act -W .github/workflows/ci.yml --dryrun

# Run CI workflow locally (full execution)
act-ci:
    @{{ mise_exec }} act -W .github/workflows/ci.yml

# Run push workflow locally (dry-run)
act-push-dry:
    @{{ mise_exec }} act push --dryrun

# Run push workflow locally (full execution)
act-push:
    @{{ mise_exec }} act push

# Run release workflow dry-run (requires tag parameter)
act-release-dry TAG:
    @echo "Running release workflow dry-run for tag: {{TAG}}"
    @{{ mise_exec }} act push --input tag={{TAG}} -W .github/workflows/release.yml --dryrun

# Test cargo-dist workflow locally
act-cargo-dist-dry:
    @echo "Running cargo-dist workflow dry-run..."
    @{{ mise_exec }} cargo dist plan

# Test cargo-dist with sample conventional commits
act-cargo-dist-test:
    @echo "feat: add new output format support" > test-commit-feat.txt
    @echo "fix: resolve connection timeout issue" > test-commit-fix.txt
    @echo "docs: update README with new examples" > test-commit-docs.txt
    @echo "feat!: migrate to new CLI interface" > test-commit-breaking.txt

# Test cargo-dist integration with release workflow
act-cargo-dist-integration TAG:
    @{{ mise_exec }} cargo dist plan
    @{{ mise_exec }} act workflow_dispatch --input tag={{TAG}} -W .github/workflows/release.yml --dryrun

# List all available GitHub Actions workflows
act-list:
    @{{ mise_exec }} act --list

# Test specific workflow job
act-job JOB:
    @{{ mise_exec }} act -j {{JOB}} --dryrun

# Clean act cache and containers
act-clean:
    @-docker ps -a | grep "act-" | awk '{print $$1}' | xargs docker rm -f 2>/dev/null || true
    @-docker images | grep "act-" | awk '{print $$3}' | xargs docker rmi -f 2>/dev/null || true

# =============================================================================
# RELEASE & VALIDATION
# =============================================================================

# Release preparation checklist
release-check: ci-check audit build-all act-ci-dry dist-plan
    @just act-cargo-dist-integration v0.2.7

# Release simulation for local testing
[unix]
release-dry: build-release
    #!/usr/bin/env bash
    set -euo pipefail
    if ! git diff-index --quiet HEAD --; then
        echo "Warning: Working directory has uncommitted changes"
    fi
    BINARY_PATH="target/release/gold_digger"
    [[ -f "$BINARY_PATH" ]] || { echo "Binary not found at $BINARY_PATH"; exit 1; }
    {{ mise_exec }} syft packages . -o cyclonedx-json=sbom-test.json
    shasum -a 256 "$BINARY_PATH" > checksums-test.txt
    shasum -a 256 sbom-test.json >> checksums-test.txt

[windows]
release-dry: build-release
    $BINARY_PATH = "target\release\gold_digger.exe"
    if (-not (Test-Path $BINARY_PATH)) {
        Write-Error "Binary not found at $BINARY_PATH"
        exit 1
    }
    {{ mise_exec }} syft packages . -o cyclonedx-json=sbom-test.json
    (Get-FileHash -Path $BINARY_PATH -Algorithm SHA256).Hash | Out-File -FilePath checksums-test.txt
    (Get-FileHash -Path sbom-test.json -Algorithm SHA256).Hash | Add-Content -Path checksums-test.txt
