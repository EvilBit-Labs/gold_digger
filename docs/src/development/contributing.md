# Contributing

Guidelines for contributing to Gold Digger. For project governance, see [GOVERNANCE.md](GOVERNANCE.md). For getting help, see [SUPPORT.md](SUPPORT.md).

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

7. Sign off all commits (see [Developer Certificate of Origin](#developer-certificate-of-origin))

8. Submit a pull request

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
- **Markdown**: Formatting with mdformat (GitHub Flavored Markdown)
- **Shell Scripts**: Validation with ShellCheck
- **GitHub Actions**: Workflow validation with actionlint
- **Commit Messages**: Conventional commit format validation
- **DCO**: Developer Certificate of Origin sign-off validation
- **Documentation**: Link checking and build validation

Install hooks: `pre-commit install` Run manually: `pre-commit run --all-files`

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new output format
fix: handle NULL values correctly
docs: update installation guide
```

All commits must include a `Signed-off-by` trailer (see [Developer Certificate of Origin](#developer-certificate-of-origin)).

## Development Guidelines

### Error Handling

- Use `anyhow::Result<T>` for fallible functions
- Provide meaningful error messages
- Never panic in production code paths

### Security

- Never log credentials or sensitive data
- Use secure defaults for TLS/SSL
- Validate all external input
- Report security issues privately per [SECURITY.md](SECURITY.md)

### Testing

- Write unit tests for new functions
- Add integration tests for CLI features using the comprehensive testing framework
- Test against both MySQL and MariaDB databases when applicable
- Validate output format compliance (CSV, JSON, TSV)
- Include error scenario testing with proper exit codes
- Maintain test coverage above 80%

## Pull Request Process

1. **Description**: Clearly describe changes and motivation using the PR template
2. **DCO Sign-off**: Ensure all commits are signed off with `git commit -s`
3. **Quality Checks**: Ensure all pre-commit hooks and CI checks pass
4. **Testing**: Include test results and coverage information
5. **Documentation**: Update docs for user-facing changes
6. **Review**: Address feedback promptly and professionally

The CODEOWNERS file automatically assigns the maintainer to review PRs based on changed files.

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

## Developer Certificate of Origin

Gold Digger requires all contributors to sign off their commits using the [Developer Certificate of Origin](https://developercertificate.org/) (DCO). The DCO certifies that you have the right to submit your contribution under the project's license.

### Signing Off Commits

Add a `Signed-off-by` trailer to every commit using the `-s` flag:

```bash
git commit -s -m "feat: add new output format"
```

This produces a commit message like:

```
feat: add new output format

Signed-off-by: Your Name <your.email@example.com>
```

The name and email must match your Git configuration (`git config user.name` and `git config user.email`).

### Fixing Missing Sign-Offs

If you forgot to sign off on the most recent commit:

```bash
git commit --amend -s --no-edit
```

For older commits, use an interactive rebase. The project allows remediation commits via the DCO bot configuration (`.github/dco.yml`), so you can also add a separate sign-off commit if needed.

### Automated Validation

The DCO check runs automatically on all pull requests. Pull requests cannot be merged until all commits are signed off.
