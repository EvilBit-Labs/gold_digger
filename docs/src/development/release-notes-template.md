# Release Notes Template

This template provides a structure for creating release notes for Gold Digger releases.

## Template Structure

````markdown
# Gold Digger v1.0.0

## 🎉 Release Highlights

Brief overview of the most important changes in this release.

## ✨ New Features

- **Feature Name**: Description of the new feature
- **Another Feature**: Description of another new feature

## 🔧 Improvements

- **Improved Area**: Description of the improvement
- **Another Improvement**: Description of another improvement

## 🐛 Bug Fixes

- **Fixed Issue**: Description of the bug fix
- **Another Fix**: Description of another bug fix

## 🔒 Security Updates

- **Security Enhancement**: Description of security improvements
- **Vulnerability Fix**: Description of security fixes

## 📦 Installation

### Quick Install

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/EvilBit-Labs/gold_digger/releases/download/{{VERSION}}/gold_digger-installer.sh | sh

# Windows
powershell -c "irm https://github.com/EvilBit-Labs/gold_digger/releases/download/{{VERSION}}/gold_digger-installer.ps1 | iex"
````

### Package Managers

```bash
# Homebrew
brew install EvilBit-Labs/tap/gold-digger

# Manual download
# Visit: https://github.com/EvilBit-Labs/gold_digger/releases/tag/v1.0.0
```

## 🔍 What's Changed

### Breaking Changes

- **Breaking Change**: Description of breaking changes and migration steps

### Deprecations

- **Deprecated Feature**: Description of deprecated features and alternatives

### Configuration Changes

- **Config Change**: Description of configuration changes

## 🧪 Testing

This release has been tested on:

- **Linux**: Ubuntu 22.04 (x86_64, ARM64)
- **macOS**: 13+ (Intel, Apple Silicon)
- **Windows**: 10/11 (x86_64, ARM64)

## 🔐 Security

- All binaries are signed with GitHub attestation
- SBOM files included for security auditing
- SHA256 checksums provided for integrity verification

### Verification

```bash
gh attestation verify gold_digger-v1.0.0-x86_64-unknown-linux-gnu.tar.gz --attestation gold_digger-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.intoto.jsonl
```

After verification, check the included SBOM and SHA256 files for complete integrity validation.

## 📋 Changelog

For a complete list of changes, see the [CHANGELOG.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/CHANGELOG.md).

## 🐛 Known Issues

- **Issue Description**: Workaround or status

## 🚀 Upgrade Guide

### From v0.2.x

1. **Step 1**: Description of upgrade step
2. **Step 2**: Description of upgrade step

### From v0.1.x

1. **Breaking Change**: Description of breaking changes
2. **Migration**: Steps to migrate

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EvilBit-Labs/gold_digger/discussions)
- **Security**: [SECURITY.md](https://github.com/EvilBit-Labs/gold_digger/blob/main/SECURITY.md)

## 🙏 Contributors

Thanks to all contributors who made this release possible:

- @contributor1 - Description of contribution
- @contributor2 - Description of contribution

## 📄 License

Gold Digger is released under the MIT License. See [LICENSE](https://github.com/EvilBit-Labs/gold_digger/blob/main/LICENSE) for details.

````markdown

## Usage Instructions

### For Major Releases (v1.0.0, v2.0.0, etc.)

1. **Include breaking changes section** with detailed migration steps
2. **Highlight major new features** prominently
3. **Provide comprehensive upgrade guide**
4. **Include security section** with detailed information

### For Minor Releases (v1.1.0, v1.2.0, etc.)

1. **Focus on new features and improvements**
2. **Include any configuration changes**
3. **Note any deprecations**
4. **Provide brief upgrade notes if needed**

### For Patch Releases (v1.0.1, v1.0.2, etc.)

1. **Focus on bug fixes and security updates**
2. **Keep it concise**
3. **Highlight any critical fixes**
4. **Minimal upgrade guidance**

## Customization Tips

### Version-Specific Content

- **Update version numbers** throughout the template
- **Adjust installation URLs** to match the specific release
- **Update testing matrix** if platforms have changed
- **Modify upgrade guides** based on previous versions

### Content Guidelines

- **Use clear, concise language**
- **Include code examples** where helpful
- **Link to relevant documentation**
- **Highlight security improvements**
- **Provide migration steps** for breaking changes

### Automation

The release notes can be partially automated using:

```bash
# Generate changelog from git commits using git-cliff
git-cliff --tag v1.0.0 --output CHANGELOG.md

# Extract commit messages for specific version
git log v0.2.0..v1.0.0 --oneline --grep="feat\|fix\|docs\|style\|refactor\|test\|chore"
````

## Integration with cargo-dist

When using cargo-dist, the release notes can be:

1. **Included in the GitHub release** created by cargo-dist
2. **Generated automatically** from conventional commits using git-cliff
3. **Customized** for specific release highlights
4. **Linked** from the main documentation

## Example Completed Release Notes

See the [GitHub Releases](https://github.com/EvilBit-Labs/gold_digger/releases) page for examples of completed release notes for previous versions.
