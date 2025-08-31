<a name="unreleased"></a>
## [Unreleased]

### BREAKING CHANGES
- **TLS Migration**: Removed native-tls/OpenSSL support in favor of always-available rustls implementation
  - **Removed**: `ssl` and `ssl-rustls` feature flags (native-tls and rustls-tls implementations)
  - **Removed**: OpenSSL dependency and native-tls crate
  - **Added**: Single rustls-based implementation always available without feature flags
  - **Migration Steps**:
    - Remove `ssl` and `ssl-rustls` feature flags from `Cargo.toml` dependencies
    - Remove OpenSSL development packages from CI/build environments
    - Update build matrices to remove OpenSSL toolchain setup steps
    - Update distro packaging to remove OpenSSL dependencies
    - TLS support is now built-in and always available

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
## [v0.2.6] - 2024-05-15
### Documentation Update
- Updated maintain tag

### Style
- Remove unused category tag and added git types

### Pull Requests
- Merge pull request [#9](https://github.com/EvilBit-Labs/gold_digger/issues/9) from EvilBit-Labs/dependabot/github_actions/github/codeql-action-3
- Merge pull request [#8](https://github.com/EvilBit-Labs/gold_digger/issues/8) from EvilBit-Labs/dependabot/github_actions/actions/checkout-4


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
