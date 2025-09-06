# Release Runbook

This document provides a step-by-step guide for creating releases using the cargo-dist automated
release workflow.

## Overview

Gold Digger uses cargo-dist for automated cross-platform releases. The release process is triggered
by pushing a Git tag and automatically handles:

- Cross-platform builds (6 target platforms)
- Multiple installer generation
- Security signing and SBOM generation
- GitHub release creation
- Package manager updates

## Pre-Release Checklist

Before creating a release, ensure the following:

### Code Quality

- [ ] All tests pass: `just test`
- [ ] Code formatting is correct: `just fmt-check`
- [ ] Linting passes: `just lint`
- [ ] Security audit passes: `just security`
- [ ] No critical vulnerabilities in dependencies

### Documentation

- [ ] README.md is up to date
- [ ] CHANGELOG.md has been generated using git-cliff
- [ ] API documentation is current
- [ ] Installation instructions are accurate

### Configuration

- [ ] Version number is updated in `Cargo.toml`
- [ ] `dist-workspace.toml` configuration is correct
- [ ] All target platforms are properly configured
- [ ] Installer configurations are valid

## Release Process

### 1. Prepare Release Branch

```bash
# Ensure you're on main branch
git checkout main
git pull origin main

# Create release branch
git checkout -b release/v1.0.0
```

### 2. Update Version and Changelog

```bash
# Update version in Cargo.toml
# Edit Cargo.toml and update version = "1.0.0"

# Generate changelog using git-cliff
git-cliff --tag v1.0.0 --output CHANGELOG.md

# Commit changes
git add Cargo.toml CHANGELOG.md
git commit -m "chore: prepare v1.0.0 release"
```

### 3. Test Release Workflow

```bash
# Test cargo-dist configuration
cargo dist plan

# Test release workflow locally (requires act)
just act-release-dry v1.0.0-test

# Verify all artifacts would be generated correctly
```

### 4. Create Release Tag

```bash
# Push release branch
git push origin release/v1.0.0

# Create pull request for review (if applicable)
# After review and approval, merge to main

# Ensure you're on main branch
git checkout main
git pull origin main

# Create and push version tag
git tag v1.0.0
git push origin v1.0.0
```

### 5. Monitor Release Process

The release workflow will automatically:

1. **Plan Phase**: Determine what artifacts to build
2. **Build Phase**: Create binaries for all 6 target platforms
3. **Global Phase**: Generate installers and SBOMs
4. **Host Phase**: Upload artifacts and create GitHub release
5. **Publish Phase**: Update Homebrew tap and other package managers

### 6. Verify Release

After the workflow completes, verify:

- [ ] GitHub release is created with correct version
- [ ] All 6 platform binaries are present
- [ ] Installers are generated (shell, PowerShell, MSI, Homebrew)
- [ ] SBOM files are included
- [ ] Checksums are provided
- [ ] Homebrew tap is updated (if applicable)

## Release Artifacts

Each release includes the following artifacts:

### Binaries

- `gold_digger-aarch64-apple-darwin.tar.gz` (macOS ARM64)
- `gold_digger-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `gold_digger-aarch64-unknown-linux-gnu.tar.gz` (Linux ARM64)
- `gold_digger-x86_64-unknown-linux-gnu.tar.gz` (Linux x86_64)
- `gold_digger-aarch64-pc-windows-msvc.zip` (Windows ARM64)
- `gold_digger-x86_64-pc-windows-msvc.zip` (Windows x86_64)

### Installers

- `gold_digger-installer.sh` (Shell installer for Linux/macOS)
- `gold_digger-installer.ps1` (PowerShell installer for Windows)
- `gold_digger-x86_64-pc-windows-msvc.msi` (MSI installer for Windows)
- Homebrew formula (automatically published to tap)

### Security Artifacts

- SBOM files in CycloneDX format (`.cdx.json`)
- GitHub attestation signatures
- SHA256 checksums

## Troubleshooting

### Common Issues

#### Workflow Fails During Build

- Check that all dependencies are available
- Verify target platform configurations
- Review build logs for specific error messages

#### Missing Artifacts

- Ensure all target platforms are configured in `dist-workspace.toml`
- Check that build matrix includes all required platforms
- Verify artifact upload permissions

#### Homebrew Tap Update Fails

- Check `HOMEBREW_TAP_TOKEN` secret is configured
- Verify tap repository permissions
- Review Homebrew formula generation

#### SBOM Generation Issues

- Ensure `cargo-cyclonedx` is properly configured
- Check that all dependencies are available for SBOM generation
- Verify CycloneDX format compliance

### Recovery Procedures

#### Failed Release

If a release fails partway through:

1. **Delete the failed release** from GitHub
2. **Delete the tag** locally and remotely
3. **Fix the issue** that caused the failure
4. **Re-run the release process** with the same version

#### Partial Artifacts

If some artifacts are missing:

1. **Check the workflow logs** for specific failure reasons
2. **Re-run the specific failed job** if possible
3. **Create a patch release** if necessary

## Post-Release Tasks

### Documentation Updates

- [ ] Update any version-specific documentation
- [ ] Verify installation instructions work with new release
- [ ] Update any example configurations

### Monitoring

- [ ] Monitor for any issues reported by users
- [ ] Check that package manager installations work correctly
- [ ] Verify security scanning results

### Communication

- [ ] Announce the release on appropriate channels
- [ ] Update any external documentation or references
- [ ] Notify stakeholders of the release

## Configuration Reference

### dist-workspace.toml

Key configuration options:

```toml
[dist]
# Target platforms
targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "aarch64-unknown-linux-gnu",
  "x86_64-unknown-linux-gnu",
  "aarch64-pc-windows-msvc",
  "x86_64-pc-windows-msvc",
]

# Installers to generate
installers = ["shell", "powershell", "npm", "homebrew", "msi"]

# Security features
github-attestation = true
cargo-auditable = true
cargo-cyclonedx = true

# Homebrew tap
tap = "EvilBit-Labs/homebrew-tap"
```

### git-cliff Configuration

The project uses git-cliff for automated changelog generation from conventional commits. The
configuration is typically in `cliff.toml` (if present) or uses git-cliff defaults.

**Key git-cliff commands:**

```bash
# Generate changelog for a specific version
git-cliff --tag v1.0.0 --output CHANGELOG.md

# Generate changelog without header (for release notes)
git-cliff --tag v1.0.0 --strip header --output release-notes.md

# Preview changelog without writing to file
git-cliff --tag v1.0.0 --prepend CHANGELOG.md
```

**Conventional Commit Format:**

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes
- `refactor:` - Code refactoring
- `test:` - Test additions or changes
- `chore:` - Maintenance tasks

### Environment Variables

Required secrets for the release workflow:

- `GITHUB_TOKEN`: Standard GitHub token for repository access
- `HOMEBREW_TAP_TOKEN`: Token for updating Homebrew tap repository

## Support

For issues with the release process:

1. Check the [cargo-dist documentation](https://opensource.axo.dev/cargo-dist/)
2. Review the [DISTRIBUTION.md](../DISTRIBUTION.md) guide
3. Create an issue in the repository
4. Check the [troubleshooting guide](../troubleshooting/build-failures.md)
