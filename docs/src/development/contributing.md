# Contributing

Guidelines for contributing to Gold Digger.

## Getting Started

1. Fork the repository

2. Create a feature branch

3. Set up development environment:

   ```bash
   just setup
   pre-commit install  # Install pre-commit hooks
   ```

4. Make your changes

5. Add tests for new functionality

6. Ensure all quality checks pass:

   ```bash
   just ci-check
   pre-commit run --all-files
   ```

7. Submit a pull request

## Code Standards

### Formatting

- Use `cargo fmt` for consistent formatting
- 100-character line limit
- Follow Rust naming conventions

### Quality Gates

All code must pass:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### Pre-commit Hooks

Gold Digger uses comprehensive pre-commit hooks that automatically run on each commit:

- **Rust**: Code formatting, linting, and security auditing
- **YAML/JSON**: Formatting with Prettier
- **Markdown**: Formatting with mdformat (GitHub Flavored Markdown)
- **Shell Scripts**: Validation with ShellCheck
- **GitHub Actions**: Workflow validation with actionlint
- **Commit Messages**: Conventional commit format validation
- **Documentation**: Link checking and build validation

Install hooks: `pre-commit install` Run manually: `pre-commit run --all-files`

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new output format
fix: handle NULL values correctly
docs: update installation guide
```

## Development Guidelines

### Error Handling

- Use `anyhow::Result<T>` for fallible functions
- Provide meaningful error messages
- Never panic in production code paths

### Security

- Never log credentials or sensitive data
- Use secure defaults for TLS/SSL
- Validate all external input

### Testing

- Write unit tests for new functions
- Add integration tests for CLI features using the comprehensive testing framework
- Test against both MySQL and MariaDB databases when applicable
- Validate output format compliance (CSV, JSON, TSV)
- Include error scenario testing with proper exit codes
- Maintain test coverage above 80%

## Pull Request Process

1. **Description**: Clearly describe changes and motivation
2. **Quality Checks**: Ensure all pre-commit hooks and CI checks pass
3. **Testing**: Include test results and coverage information
4. **Documentation**: Update docs for user-facing changes
5. **Review**: Address feedback promptly and professionally

### Before Submitting

Run the complete quality check suite:

```bash
# Run all CI-equivalent checks
just ci-check

# Verify pre-commit hooks pass
pre-commit run --all-files

# Test multiple feature combinations
just build-all

# Run integration tests (requires Docker)
just test-integration

# Test release workflow (optional)
just release-dry
```

## Code Review

Reviews focus on:

- Correctness and safety
- Performance implications
- Security considerations
- Code clarity and maintainability
