# Releasing Gold Digger

Gold Digger uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for automated cross-platform releases triggered by Git tags. For the full runbook with troubleshooting and recovery procedures, see [docs/src/development/release-runbook.md](docs/src/development/release-runbook.md).

## Pre-Release Checklist

- [ ] All quality gates pass: `just ci-check`
- [ ] Version bumped in `Cargo.toml`
- [ ] Changelog regenerated: `just changelog vX.Y.Z`
- [ ] `dist-workspace.toml` configuration is correct

## Release Steps

1. **Create release branch**

   ```bash
   git checkout main && git pull origin main
   git checkout -b release/vX.Y.Z
   ```

2. **Update version and changelog**

   ```bash
   # Edit Cargo.toml version field
   just changelog vX.Y.Z
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore: prepare vX.Y.Z release"
   ```

3. **Open PR and merge to main**

   ```bash
   git push origin release/vX.Y.Z
   # Open PR, get review, merge
   ```

4. **Tag and push**

   ```bash
   git checkout main && git pull origin main
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. **Monitor the `release.yml` workflow** on GitHub Actions

## Artifact Output

cargo-dist produces binaries for 6 platforms (macOS ARM64/Intel, Linux ARM64/x86_64, Windows ARM64/x86_64) plus shell, PowerShell, MSI, and Homebrew installers. Each release also includes CycloneDX SBOMs and SHA256 checksums.

## See Also

- [Full release runbook](docs/src/development/release-runbook.md) -- troubleshooting, recovery procedures, and configuration reference
