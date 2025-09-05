<a name="unreleased"></a>
## [Unreleased]

### BREAKING CHANGES
- **TLS Migration**: Completed migration to rustls-only TLS implementation
  - **Removed**: `ssl` and `ssl-rustls` feature flags - TLS is now always available
  - **Removed**: `native-tls` dependency and OpenSSL support completely
  - **Added**: Single rustls-based implementation with enhanced security controls
  - **Enhanced**: Platform certificate store integration on all platforms (Windows/macOS/Linux)
  - **Migration Impact**:
    - TLS support is now built into all Gold Digger binaries without requiring feature flags
    - Build commands no longer need `--features ssl` or `--features ssl-rustls`
    - OpenSSL development packages are no longer required for building
    - Certificate validation behavior may be more strict than native-tls (use CLI flags for compatibility)
    - All existing DATABASE_URL formats continue to work unchanged
  - **New TLS CLI Flags**: `--tls-ca-file`, `--insecure-skip-hostname-verify`, `--allow-invalid-certificate`

### Features
- **TLS Implementation**: Migrated to always-available rustls implementation with enhanced security controls
  - TLS support is now built into all Gold Digger binaries without requiring feature flags
  - Default behavior uses platform certificate store with full validation (no flags required)
  - **CLI Flags**: `--tls-ca-file <path>` (custom CA), `--insecure-skip-hostname-verify` (skip hostname), `--allow-invalid-certificate` (disable validation)
  - **Mutually Exclusive**: Only one TLS flag allowed; conflicting flags exit with code 2 (config error)
  - **Security Warnings**: Insecure modes print warnings but continue; `--allow-invalid-certificate` shows danger warning
  - **Error Handling**: TLS errors exit with code 3 (auth error); specific flag suggestions provided (e.g., "use --tls-ca-file <path> or --allow-invalid-certificate")
  - **CA File Validation**: `--tls-ca-file` validates file exists and contains valid PEM certificates; invalid format exits with code 5 (IO error)

### Documentation
- Updated comprehensive TLS configuration guide with new rustls-only model
- Updated README.md with simplified TLS implementation details
- Updated WARP.md, AGENTS.md, and GEMINI.md with new TLS architecture


<a name="v0.2.6"></a>
## [v0.2.6] - 2025-09-04

### Dependencies
- Fixed invalid dependency versions in Cargo.toml to use published crates.io versions
- Updated testcontainers-modules, tempfile, assert_cmd, insta, temp-env, rustls, and rustls-pemfile to latest stable versions

### TLS Migration
- **Completed migration from native-tls to rustls-only implementation**
- **Removed**: All native-tls dependencies and OpenSSL support
- **Simplified**: TLS is now always available without feature flags
- **Enhanced**: New CLI flags for granular TLS security control
- **Improved**: Better error messages with specific CLI flag suggestions for certificate issues


<a name="v0.2.5"></a>
## [v0.2.5] - 2024-05-15
### Code Refactoring
- Bumped version due to weird mismatch


<a name="v0.2.4"></a>
## [v0.2.4] - 2024-05-15
### Bug Fixes
- Bumped mysql crate version and tested

### Maintenance
- Add dependabot configuration
- Add git-chglog support


<a name="v0.2.3"></a>
## [v0.2.3] - 2023-09-14

<a name="v0.2.2"></a>
## [v0.2.2] - 2023-07-11

<a name="v0.2.1"></a>
## [v0.2.1] - 2023-07-11
### Pull Requests
- Merge pull request [#7](https://github.com/EvilBit-Labs/gold_digger/issues/7) from EvilBit-Labs/develop


<a name="v0.2.0"></a>
## [v0.2.0] - 2023-02-19
### Pull Requests
- Merge pull request [#6](https://github.com/EvilBit-Labs/gold_digger/issues/6) from EvilBit-Labs/develop
- Merge pull request [#5](https://github.com/EvilBit-Labs/gold_digger/issues/5) from EvilBit-Labs/develop
- Merge pull request [#3](https://github.com/EvilBit-Labs/gold_digger/issues/3) from EvilBit-Labs/hotfix/updating_crates
- Merge pull request [#2](https://github.com/EvilBit-Labs/gold_digger/issues/2) from EvilBit-Labs/develop


<a name="v0.1.2"></a>
## [v0.1.2] - 2022-05-05

<a name="v0.1.1"></a>
## [v0.1.1] - 2022-05-05
### Pull Requests
- Merge pull request [#1](https://github.com/EvilBit-Labs/gold_digger/issues/1) from EvilBit-Labs/hotfix/v0.1.1


<a name="v0.1.0"></a>
## v0.1.0 - 2022-05-05

[Unreleased]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.6...HEAD
[v0.2.6]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.5...v0.2.6
[v0.2.5]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.4...v0.2.5
[v0.2.4]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.3...v0.2.4
[v0.2.3]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.2...v0.2.3
[v0.2.2]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.1...v0.2.2
[v0.2.1]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.2.0...v0.2.1
[v0.2.0]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.1.2...v0.2.0
[v0.1.2]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.1.1...v0.1.2
[v0.1.1]: https://github.com/EvilBit-Labs/gold_digger/compare/v0.1.0...v0.1.1
