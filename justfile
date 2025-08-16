# Gold Digger Justfile
# Task runner for the MySQL/MariaDB query tool

# Default recipe
default: lint

# Set shell for recipe execution with proper PATH
set shell := ["zsh", "-c"]

# Variables
export RUST_BACKTRACE := "1"
export CARGO_TERM_COLOR := "always"

# Development setup
setup:
    #!/usr/bin/env zsh
    echo "🔧 Setting up development environment..."
    rustup component add rustfmt clippy
    cargo install cargo-nextest --locked || echo "cargo-nextest already installed"
    echo "✅ Setup complete!"

# Install development tools (extended setup)
install-tools:
    #!/usr/bin/env zsh
    echo "🛠️ Installing additional development tools..."
    cargo install cargo-tarpaulin --locked || echo "cargo-tarpaulin already installed"
    cargo install cargo-audit --locked || echo "cargo-audit already installed"
    cargo install cargo-deny --locked || echo "cargo-deny already installed"
    echo "✅ Tools installed!"

# Format code
fmt:
    @echo "📝 Formatting code..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo fmt

# Check formatting
fmt-check:
    @echo "🔍 Checking code formatting..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo fmt --check

# Run clippy linting
lint:
    @echo "🔍 Running clippy linting..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy --all-targets --all-features -- -D warnings

# Run clippy with fixes
fix:
    @echo "🔧 Running clippy with automatic fixes..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy --fix --allow-dirty --allow-staged

# Build debug version
build:
    @echo "🔨 Building debug version..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo build

# Build release version
build-release:
    @echo "🔨 Building release version..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo build --release

# Build with vendored OpenSSL (static linking)
build-vendored:
    @echo "🔨 Building with vendored OpenSSL..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo build --release --features vendored

# Build minimal version (no default features)
build-minimal:
    @echo "🔨 Building minimal version..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo build --release --no-default-features --features "csv json"

# Build all feature combinations
build-all: build build-release build-vendored build-minimal
    @echo "✅ All builds completed!"

# Install locally from workspace
install:
    @echo "📦 Installing locally from workspace..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo install --path .

# Run tests
test:
    @echo "🧪 Running tests..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo test

# Run tests with nextest (if available)
test-nextest:
    @echo "🧪 Running tests with nextest..."
    export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run || cargo test

# Run tests with coverage
coverage:
    @echo "📊 Running tests with coverage..."
    cargo tarpaulin --out Html --output-dir target/tarpaulin

# Security audit
audit:
    @echo "🔒 Running security audit..."
    cargo audit

# Check for license/security issues
deny:
    @echo "🚫 Checking licenses and security..."
    cargo deny check || echo "cargo-deny not installed - run 'just install-tools'"

# Quality gates (CI equivalent)
ci-check: fmt-check lint test-nextest
    @echo "✅ All CI checks passed!"

# Quick development check
check: fmt lint test
    @echo "✅ Quick development checks passed!"

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean

# Run with example environment variables
run OUTPUT_FILE DATABASE_URL DATABASE_QUERY:
    #!/usr/bin/env zsh
    echo "🚀 Running Gold Digger..."
    echo "Output: {{OUTPUT_FILE}}"
    echo "Database: $(echo {{DATABASE_URL}} | sed 's/:[^@]*@/:***@/')"
    echo "Query: {{DATABASE_QUERY}}"
    OUTPUT_FILE={{OUTPUT_FILE}} \
    DATABASE_URL={{DATABASE_URL}} \
    DATABASE_QUERY={{DATABASE_QUERY}} \
    cargo run --release

# Run with safe example (casting to avoid panics)
run-safe:
    #!/usr/bin/env zsh
    echo "🚀 Running Gold Digger with safe example..."
    OUTPUT_FILE=/tmp/gold_digger_example.json \
    DATABASE_URL="mysql://user:pass@localhost:3306/test" \
    DATABASE_QUERY="SELECT CAST(1 AS CHAR) as id, CAST('test' AS CHAR) as name" \
    cargo run --release

# Development server (watch for changes) - requires cargo-watch
watch:
    @echo "👀 Watching for changes..."
    cargo watch -x "run --release" || echo "Install cargo-watch: cargo install cargo-watch"

# Generate documentation
docs:
    @echo "📚 Generating documentation..."
    cargo doc --open --no-deps

# Build documentation without opening
docs-build:
    @echo "📚 Building documentation..."
    cargo doc --no-deps

# Check for outdated dependencies
outdated:
    @echo "📅 Checking for outdated dependencies..."
    cargo outdated || echo "Install cargo-outdated: cargo install cargo-outdated"

# Update dependencies
update:
    @echo "⬆️ Updating dependencies..."
    cargo update

# Benchmark (when criterion tests exist)
bench:
    @echo "⚡ Running benchmarks..."
    cargo bench || echo "No benchmarks found"

# Profile release build
profile:
    @echo "📊 Profiling release build..."
    cargo build --release
    @echo "Use 'perf record target/release/gold_digger' or similar profiling tools"

# Show feature matrix
features:
    @echo "🎛️ Available feature combinations:"
    @echo ""
    @echo "Default features:"
    @echo "  cargo build --release"
    @echo ""
    @echo "Static build with vendored OpenSSL:"
    @echo "  cargo build --release --features vendored"
    @echo ""
    @echo "Minimal build (no TLS, no extra types):"
    @echo "  cargo build --no-default-features --features \"csv json\""
    @echo ""
    @echo "All MySQL types:"
    @echo "  cargo build --release --features \"default additional_mysql_types\""

# Check current version
version:
    @echo "📋 Current version information:"
    @echo "Cargo.toml version: $(grep '^version' Cargo.toml | cut -d'"' -f2)"
    @echo "CHANGELOG.md version: $(grep -m1 '## \[v' CHANGELOG.md | sed 's/.*\[v/v/' | sed 's/\].*//')"
    @echo ""
    @echo "⚠️  Note: Versions may be out of sync - check WARP.md for details"

# Show project status
status:
    @echo "📊 Gold Digger Project Status:"
    @echo ""
    @echo "🏗️  Architecture: Environment variable driven, structured output"
    @echo "🎯 Current: v0.2.5 (check version discrepancy)"
    @echo "🚀 Target: v1.0 with CLI interface"
    @echo "🧑‍💻 Maintainer: UncleSp1d3r"
    @echo ""
    @echo "🚨 Critical Issues:"
    @echo "  • Type conversion panics on NULL/non-string values"
    @echo "  • No dotenv support (use exported env vars)"
    @echo "  • Non-deterministic JSON output"
    @echo "  • Pattern matching bug in src/main.rs:59"
    @echo ""
    @echo "📖 See WARP.md for detailed information"

# Release preparation checklist
release-check:
    #!/usr/bin/env zsh
    echo "🚀 Pre-release checklist:"
    echo ""
    echo "1. Version sync check:"
    CARGO_VERSION=$(grep '^version' Cargo.toml | cut -d'"' -f2)
    CHANGELOG_VERSION=$(grep -m1 '## \[v' CHANGELOG.md | sed 's/.*\[v/v/' | sed 's/\].*//')
    if [[ "$CARGO_VERSION" != "${CHANGELOG_VERSION#v}" ]]; then
        echo "   ❌ Version mismatch: Cargo.toml=$CARGO_VERSION, CHANGELOG=$CHANGELOG_VERSION"
    else
        echo "   ✅ Versions synchronized"
    fi
    echo ""
    echo "2. Running quality checks..."
    just ci-check
    echo ""
    echo "3. Security checks..."
    just audit
    echo ""
    echo "4. Build matrix test..."
    just build-all
    echo ""
    echo "📋 Manual checklist:"
    echo "   □ Update CHANGELOG.md if needed"
    echo "   □ Review project_spec/requirements.md for completeness"
    echo "   □ Test with real database connections"
    echo "   □ Verify all feature flag combinations work"
    echo "   □ Check that credentials are never logged"

# Show help
help:
    @echo "🛠️  Gold Digger Justfile Commands:"
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
    @echo "  fmt           Format code"
    @echo "  fmt-check     Check formatting"
    @echo "  lint          Run clippy linting"
    @echo "  fix           Run clippy with automatic fixes"
    @echo "  check         Quick development checks"
    @echo "  ci-check      Full CI equivalent checks"
    @echo ""
    @echo "Testing:"
    @echo "  test          Run tests"
    @echo "  test-nextest  Run tests with nextest"
    @echo "  coverage      Run tests with coverage report"
    @echo "  bench         Run benchmarks"
    @echo ""
    @echo "Security:"
    @echo "  audit         Security audit"
    @echo "  deny          License and security checks"
    @echo ""
    @echo "Running:"
    @echo "  run OUTPUT_FILE DATABASE_URL DATABASE_QUERY  Run with custom env vars"
    @echo "  run-safe      Run with safe example query"
    @echo "  watch         Watch for changes (requires cargo-watch)"
    @echo ""
    @echo "Documentation:"
    @echo "  docs          Generate and open documentation"
    @echo "  docs-build    Build documentation only"
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
    @echo ""
    @echo "📖 For detailed project information, see WARP.md, AGENTS.md, or .cursor/rules/"
