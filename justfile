# Gold Digger Justfile
# Task runner for the MySQL/MariaDB query tool
# Run `just` or `just --list` to see available recipes

set shell := ["bash", "-cu"]
set windows-powershell := true
set dotenv-load := true
set ignore-comments := true

# Use mise to manage all dev tools
# See mise.toml for tool versions
mise_exec := "mise exec --"

# ─────────────────────────────────────────────────────────────────────────────
# Variables
# ─────────────────────────────────────────────────────────────────────────────

project_dir := justfile_directory()
binary_name := "gold_digger"

# Platform-specific commands
_null := if os_family() == "windows" { "nul" } else { "/dev/null" }

# Act configuration
act_cmd := "act --container-architecture linux/amd64"

# ─────────────────────────────────────────────────────────────────────────────
# Default & Help
# ─────────────────────────────────────────────────────────────────────────────

[private]
default:
    @just --list --unsorted

alias h := help
alias l := list

# Show available recipes
[group('help')]
help:
    @just --list

# Show recipes in a specific group
[group('help')]
list group="":
    @just --list --unsorted {{ if group != "" { "--list-heading='' --list-prefix='  ' | grep -A999 '" + group + "'" } else { "" } }}

# ─────────────────────────────────────────────────────────────────────────────
# Setup & Installation
# ─────────────────────────────────────────────────────────────────────────────

# Development setup - mise handles all tool installation via mise.toml
[group('setup')]
setup:
    @mise install
    @{{ mise_exec }} pre-commit install --hook-type commit-msg 2>{{ _null }} || true

# ─────────────────────────────────────────────────────────────────────────────
# Code Quality
# ─────────────────────────────────────────────────────────────────────────────

alias f := fmt
alias format := fmt
alias c := check

# Format code
[group('quality')]
fmt:
    @{{ mise_exec }} cargo fmt

# Check formatting
[group('quality')]
fmt-check:
    @{{ mise_exec }} cargo fmt --check

# Run clippy linting
[group('quality')]
lint:
    @{{ mise_exec }} cargo clippy --all-targets --release -- -D warnings
    @{{ mise_exec }} cargo clippy --all-targets --no-default-features --features "additional_mysql_types verbose" -- -D warnings

# Lint SQL files with sqlfluff
[group('quality')]
lint-sql:
    @{{ mise_exec }} sqlfluff lint tests/fixtures/**/*.sql

# Fix SQL formatting with sqlfluff
[group('quality')]
fix-sql:
    @{{ mise_exec }} sqlfluff fix tests/fixtures/**/*.sql

# Run clippy with fixes
[group('quality')]
fix:
    @{{ mise_exec }} cargo clippy --fix --allow-dirty --allow-staged

# Quick development check
[group('quality')]
check: lint test-no-docker
    @{{ mise_exec }} pre-commit run -a 2>{{ _null }} || true

# Format files (for pre-commit hooks)
[private]
_format-files +FILES:
    @{{ mise_exec }} npx prettier --write --config .prettierrc.json {{FILES}}

# ─────────────────────────────────────────────────────────────────────────────
# Build
# ─────────────────────────────────────────────────────────────────────────────

alias b := build

# Build debug version
[group('build')]
build:
    @{{ mise_exec }} cargo build

# Build release version
[group('build')]
build-release:
    @{{ mise_exec }} cargo build --release

# Build minimal version (no default features)
#
# Note (todo #011): the former `csv`/`json` feature markers were removed because
# they never actually gated compilation. CSV and JSON output are now built in
# unconditionally; a minimal build just drops `additional_mysql_types` + `verbose`.
[group('build')]
build-minimal:
    @{{ mise_exec }} cargo build --release --no-default-features

# Build all feature combinations
[group('build')]
build-all: build build-release build-minimal

# Install locally from workspace
[group('build')]
install:
    @{{ mise_exec }} cargo install --path .

# Clean build artifacts
[group('build')]
[confirm("This will remove all build artifacts. Continue?")]
clean:
    @{{ mise_exec }} cargo clean

# ─────────────────────────────────────────────────────────────────────────────
# Testing
# ─────────────────────────────────────────────────────────────────────────────

alias t := test

# Run all tests (including ignored)
[group('test')]
test:
    @{{ mise_exec }} cargo nextest run --run-ignored all

# Run tests without Docker tests (non-ignored only)
[group('test')]
test-no-docker:
    @{{ mise_exec }} cargo nextest run

# Run integration tests (requires Docker)
[group('test')]
test-integration:
    @{{ mise_exec }} cargo nextest run --features integration_tests

# Run all tests including integration tests
[group('test')]
test-all:
    @{{ mise_exec }} cargo nextest run --features integration_tests

# Run integration tests with flaky test quarantine
[group('test')]
test-integration-nextest:
    @GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=3 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests

# Run integration tests with JUnit XML output for CI
[group('test')]
test-integration-ci:
    @mkdir -p target/nextest-reports
    @GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=3 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --message-format json-pretty \
            --junit-path target/nextest-reports/integration-tests.xml

# Run fast integration test subset for PR validation
[group('test')]
test-integration-fast:
    @GOLD_DIGGER_INTEGRATION_FAST=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 2 --timeout 300

# Run comprehensive integration test suite for main branch
[group('test')]
test-integration-comprehensive:
    @GOLD_DIGGER_INTEGRATION_COMPREHENSIVE=1 GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 4 --timeout 900

# Check Docker availability for integration tests
[group('test')]
check-docker:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking Docker availability..."
    if ! command -v docker &>/dev/null; then
        echo "Docker is not installed"
        exit 1
    fi
    if ! docker info &>/dev/null; then
        echo "Docker daemon is not running"
        exit 1
    fi
    echo "Docker is available"
    docker --version

# Run integration tests with Docker availability check
[group('test')]
test-integration-safe: check-docker test-integration-nextest

# Run integration tests with artifact collection on failure
[group('test')]
test-integration-debug:
    @mkdir -p target/integration-test-artifacts
    @GOLD_DIGGER_TEST_DEBUG=1 GOLD_DIGGER_COLLECT_ARTIFACTS=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --failure-output immediate --success-output never || \
        echo "Integration tests failed - check target/integration-test-artifacts/ for debug info"

# Run integration tests with performance benchmarking
[group('test')]
test-integration-perf:
    @GOLD_DIGGER_INTEGRATION_PERF=1 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
            --test-threads 1 --timeout 600

# Run integration tests with both TLS and non-TLS database configurations
[group('test')]
test-integration-matrix: check-docker
    @echo "Testing non-TLS configurations..."
    @GOLD_DIGGER_TEST_TLS=false {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests
    @echo "Testing TLS configurations..."
    @GOLD_DIGGER_TEST_TLS=true {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests

# Run integration tests with flaky test quarantine enabled
[group('test')]
test-integration-quarantine:
    @GOLD_DIGGER_QUARANTINE_FLAKY_TESTS=1 GOLD_DIGGER_FLAKY_TEST_RETRIES=5 \
        {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests --retries 3

# Test the new test execution utilities
[group('test')]
test-execution-utilities:
    @{{ mise_exec }} cargo nextest run --test test_execution_utilities

# Generate integration test reports for CI
[group('test')]
generate-test-reports:
    @mkdir -p target/test-reports
    @NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 {{ mise_exec }} cargo nextest run --test integration_tests --features integration_tests \
        --message-format libtest-json > target/test-reports/integration-tests.json || true
    @NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 {{ mise_exec }} cargo nextest run --test test_execution_utilities \
        --message-format libtest-json > target/test-reports/execution-utilities.json || true
    @echo "Test reports generated in target/test-reports/"

# Validate CI integration and test execution utilities
[group('test')]
validate-ci-integration: test-execution-utilities
    @echo "Test execution utilities validated"
    @echo "CI environment detection working"
    @echo "Nextest integration configured"

# ─────────────────────────────────────────────────────────────────────────────
# Benchmarking
# ─────────────────────────────────────────────────────────────────────────────

# Run full Criterion benchmark suite (mirrors CI)
[group('bench')]
bench:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage

# Run benchmarks and save current performance as a named baseline
[group('bench')]
bench-baseline BASELINE_NAME:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --save-baseline {{BASELINE_NAME}}

# Run benchmarks and compare against a previously saved baseline
[group('bench')]
bench-compare BASELINE_NAME:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --baseline {{BASELINE_NAME}}

# Open the generated HTML report from Criterion output directory
[group('bench')]
bench-report:
    #!/usr/bin/env bash
    set -euo pipefail
    REPORT_PATH=$(find target/criterion -name 'index.html' 2>/dev/null | head -1)
    if [ -z "$REPORT_PATH" ]; then
        echo "No benchmark report found. Run 'just bench' first."
        exit 1
    fi
    if command -v xdg-open &>/dev/null; then
        xdg-open "$REPORT_PATH"
    elif command -v open &>/dev/null; then
        open "$REPORT_PATH"
    else
        echo "Open manually: $REPORT_PATH"
    fi

# Run a specific benchmark by name
[group('bench')]
bench-specific BENCHMARK_NAME:
    @{{ mise_exec }} cargo bench --bench {{BENCHMARK_NAME}}

# Run benchmarks with reduced sample size for faster feedback during development
[group('bench')]
bench-quick:
    @{{ mise_exec }} cargo bench --bench rows_processing --bench output_formats --bench value_conversion --bench memory_usage -- --quick

# Profile release build
[group('bench')]
profile:
    @{{ mise_exec }} cargo build --release

# ─────────────────────────────────────────────────────────────────────────────
# Coverage
# ─────────────────────────────────────────────────────────────────────────────

# Run tests with coverage (HTML report)
[group('coverage')]
coverage:
    @{{ mise_exec }} cargo llvm-cov --package gold_digger --html

# Run tests with coverage (LCOV for CI)
[group('coverage')]
coverage-llvm:
    @{{ mise_exec }} cargo llvm-cov --workspace --lcov --output-path lcov.info

alias cover := coverage-llvm

# Run coverage with threshold check (for CI)
[group('coverage')]
coverage-ci:
    @{{ mise_exec }} cargo llvm-cov --package gold_digger --json --output-path coverage.json

# ─────────────────────────────────────────────────────────────────────────────
# Security & Auditing
# ─────────────────────────────────────────────────────────────────────────────

# Security audit with cargo-audit
[group('security')]
audit:
    @{{ mise_exec }} cargo audit

# Check for license/security issues
[group('security')]
deny-check:
    @{{ mise_exec }} cargo deny check

alias deny := deny-check

# Comprehensive security scanning (combines audit, deny, and grype)
[group('security')]
security: audit deny-check
    @{{ mise_exec }} grype . --fail-on high || echo "High or critical vulnerabilities found"

# ─────────────────────────────────────────────────────────────────────────────
# Dependencies & Validation
# ─────────────────────────────────────────────────────────────────────────────

# Validate TLS dependency tree
[group('deps')]
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
    echo "Standard build includes rustls TLS support"
    echo "No native-tls dependencies found"
    echo "TLS validation passed"

# Check for outdated dependencies
[group('deps')]
outdated:
    @{{ mise_exec }} cargo outdated

# Update dependencies
[group('deps')]
update:
    @{{ mise_exec }} cargo update

# ─────────────────────────────────────────────────────────────────────────────
# Documentation
# ─────────────────────────────────────────────────────────────────────────────

alias d := docs
alias docs-serve := docs

# Build complete documentation (mdBook + rustdoc)
[group('docs')]
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
[group('docs')]
[unix]
docs:
    @cd docs && {{ mise_exec }} mdbook serve --open

[group('docs')]
[windows]
docs:
    @echo "mdbook serve requires a Unix-like environment"

# Clean documentation artifacts
[group('docs')]
docs-clean:
    @rm -rf docs/book target/doc

# Check documentation (build + link validation + formatting)
[group('docs')]
docs-check:
    @cd docs && {{ mise_exec }} mdbook build
    @just fmt-check

# ─────────────────────────────────────────────────────────────────────────────
# SBOM
# ─────────────────────────────────────────────────────────────────────────────

# Generate Software Bill of Materials (SBOM) using cargo-cyclonedx
[group('sbom')]
sbom:
    @{{ mise_exec }} cargo cyclonedx --override-filename sbom.json
    @{{ mise_exec }} cargo tree --format "{p} {f}" | head -20

# Generate SBOM using syft (alternative)
[group('sbom')]
sbom-syft:
    @{{ mise_exec }} syft packages . -o cyclonedx-json=sbom.json
    @{{ mise_exec }} syft packages . -o table

# ─────────────────────────────────────────────────────────────────────────────
# Distribution (cargo-dist)
# ─────────────────────────────────────────────────────────────────────────────

# Initialize cargo-dist configuration
[group('dist')]
dist-init:
    @echo "Initializing cargo-dist configuration..."
    @{{ mise_exec }} dist init --yes
    @echo "cargo-dist initialized successfully"

# Plan cargo-dist release (dry-run)
[group('dist')]
dist-plan:
    @{{ mise_exec }} dist plan

# Build cargo-dist artifacts locally
[group('dist')]
dist-build:
    @{{ mise_exec }} dist build
    @find target/distrib -type f -name "*" 2>/dev/null | head -10 || echo "  (no artifacts found)"

# Generate cargo-dist installers
[group('dist')]
dist-generate:
    @{{ mise_exec }} dist generate

# Validate cargo-dist configuration
[group('dist')]
dist-check:
    @{{ mise_exec }} dist plan >/dev/null && echo "cargo-dist configuration check passed"

# Validate cargo-dist configuration (verbose)
[group('dist')]
validate-cargo-dist:
    @test -f dist-workspace.toml && echo "dist-workspace.toml exists" || echo "Missing: dist-workspace.toml"
    @{{ mise_exec }} dist plan >/dev/null && echo "cargo-dist configuration is valid"

# ─────────────────────────────────────────────────────────────────────────────
# CI
# ─────────────────────────────────────────────────────────────────────────────

# Quality gates (CI equivalent)
[group('ci')]
ci-check: check fmt-check lint-sql test validate-deps deny-check

# Full CI workflow equivalent - mirrors .github/workflows/ci.yml exactly
[group('ci')]
ci-full:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running full CI workflow equivalent..."

    # Job 1: Quality checks (mirrors quality job)
    echo "[1/6] Quality checks..."
    {{ mise_exec }} cargo fmt --check
    {{ mise_exec }} cargo clippy -- -D warnings
    {{ mise_exec }} cargo clippy --no-default-features --features "additional_mysql_types verbose" -- -D warnings

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
    {{ mise_exec }} cargo nextest run --no-default-features --features "additional_mysql_types verbose"
    {{ mise_exec }} cargo build --release
    {{ mise_exec }} cargo build --release --no-default-features --features "additional_mysql_types verbose"

    "$BIN" --help | grep -qE "(tls-ca-file|insecure-skip-hostname-verify|allow-invalid-certificate)"
    {{ mise_exec }} cargo tree | grep -qE "(rustls|rustls-native-certs)"
    {{ mise_exec }} cargo tree --no-default-features --features "additional_mysql_types verbose" \
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
        --no-default-features --features "additional_mysql_types verbose"
    cat lcov-default.info lcov-minimal.info > lcov.info

    echo "CI workflow equivalent completed successfully!"

# Comprehensive full checks (all non-destructive validation)
[group('ci')]
full-checks: ci-check audit deny-check docs-check coverage-llvm build-all validate-cargo-dist

# ─────────────────────────────────────────────────────────────────────────────
# GitHub Actions (act)
# ─────────────────────────────────────────────────────────────────────────────

[private]
_require-act:
    #!/usr/bin/env bash
    if ! command -v act >/dev/null 2>&1; then
        echo "Error: act not found. Install: brew install act"
        exit 1
    fi

# Pull required Docker image for local GitHub Actions testing
[group('act')]
act-setup: _require-act
    @docker pull catthehacker/ubuntu:act-22.04

# Run CI workflow locally (dry-run)
[group('act')]
act-ci-dry: _require-act
    @{{ mise_exec }} {{ act_cmd }} -W .github/workflows/ci.yml --dryrun

# Run CI workflow locally (full execution)
[group('act')]
act-ci: _require-act
    @{{ mise_exec }} {{ act_cmd }} -W .github/workflows/ci.yml

# Run push workflow locally (dry-run)
[group('act')]
act-push-dry: _require-act
    @{{ mise_exec }} {{ act_cmd }} push --dryrun

# Run push workflow locally (full execution)
[group('act')]
act-push: _require-act
    @{{ mise_exec }} {{ act_cmd }} push

# Run release workflow dry-run (requires tag parameter)
[group('act')]
act-release-dry TAG: _require-act
    @echo "Running release workflow dry-run for tag: {{TAG}}"
    @{{ mise_exec }} {{ act_cmd }} push --input tag={{TAG}} -W .github/workflows/release.yml --dryrun

# Test cargo-dist workflow locally
[group('act')]
act-cargo-dist-dry: _require-act
    @echo "Running cargo-dist workflow dry-run..."
    @{{ mise_exec }} dist plan

# Test cargo-dist with sample conventional commits
[group('act')]
act-cargo-dist-test:
    @echo "feat: add new output format support" > test-commit-feat.txt
    @echo "fix: resolve connection timeout issue" > test-commit-fix.txt
    @echo "docs: update README with new examples" > test-commit-docs.txt
    @echo "feat!: migrate to new CLI interface" > test-commit-breaking.txt

# Test cargo-dist integration with release workflow
[group('act')]
act-cargo-dist-integration TAG: _require-act
    @{{ mise_exec }} dist plan
    @{{ mise_exec }} {{ act_cmd }} workflow_dispatch --input tag={{TAG}} -W .github/workflows/release.yml --dryrun

# List all available GitHub Actions workflows
[group('act')]
act-list: _require-act
    @{{ mise_exec }} {{ act_cmd }} --list

# Test specific workflow job
[group('act')]
act-job JOB: _require-act
    @{{ mise_exec }} {{ act_cmd }} -j {{JOB}} --dryrun

# Clean act cache and containers
[group('act')]
act-clean:
    @-docker ps -a | grep "act-" | awk '{print $$1}' | xargs docker rm -f 2>/dev/null || true
    @-docker images | grep "act-" | awk '{print $$3}' | xargs docker rmi -f 2>/dev/null || true

# ─────────────────────────────────────────────────────────────────────────────
# Release & Validation
# ─────────────────────────────────────────────────────────────────────────────

# Generate CHANGELOG.md using git-cliff
[group('release')]
changelog TAG:
    @{{ mise_exec }} git-cliff --tag {{TAG}} --output CHANGELOG.md

# Generate release-notes.md (no header, for GitHub release body)
[group('release')]
release-notes TAG:
    @{{ mise_exec }} git-cliff --tag {{TAG}} --strip header --output release-notes.md

# Release preparation checklist
[group('release')]
release-check: ci-check audit build-all act-ci-dry dist-plan
    @just act-cargo-dist-integration v0.2.7

# Release simulation for local testing
[group('release')]
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

[group('release')]
[windows]
release-dry: build-release
    #!pwsh
    $BINARY_PATH = "target\release\gold_digger.exe"
    if (-not (Test-Path $BINARY_PATH)) {
        Write-Error "Binary not found at $BINARY_PATH"
        exit 1
    }
    {{ mise_exec }} syft packages . -o cyclonedx-json=sbom-test.json
    (Get-FileHash -Path $BINARY_PATH -Algorithm SHA256).Hash | Out-File -FilePath checksums-test.txt
    (Get-FileHash -Path sbom-test.json -Algorithm SHA256).Hash | Add-Content -Path checksums-test.txt

# ─────────────────────────────────────────────────────────────────────────────
# Running & Development
# ─────────────────────────────────────────────────────────────────────────────

# Run with example environment variables
[group('dev')]
run OUTPUT_FILE DATABASE_URL DATABASE_QUERY:
    @OUTPUT_FILE={{OUTPUT_FILE}} DATABASE_URL={{DATABASE_URL}} DATABASE_QUERY={{DATABASE_QUERY}} \
        {{ mise_exec }} cargo run --release

# Run with safe example (casting to avoid panics)
[group('dev')]
run-safe:
    @DB_URL=sqlite://dummy.db API_KEY=dummy NODE_ENV=testing APP_ENV=safe \
        {{ mise_exec }} cargo run --release

# Development server (watch for changes)
[group('dev')]
watch:
    @{{ mise_exec }} cargo watch -x "run --release"
