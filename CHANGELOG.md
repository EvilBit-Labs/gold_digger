<a name="unreleased"></a>

## [Unreleased]

### BREAKING / BEHAVIOR

- **CLI-first configuration (F001-F003):** CLI flags now take precedence over
  the `DATABASE_URL` / `DATABASE_QUERY` / `OUTPUT_FILE` environment variables.
  Both mechanisms still work; CLI wins when both are present.
- **Typed exit codes (F005):** Exit codes are now 0 (success), 1 (no rows),
  2 (config error), 3 (DB auth / TLS), 4 (query execution), 5 (I/O).
  Previously unknown errors defaulted to exit 255.
- **Streaming output (F007):** Rows stream through a `RowSink` directly into
  `<output>.tmp` (renamed on success). The old "buffer full result set in
  memory" path is gone; peak memory is now O(1 row), not O(N rows).
- **`--output` safety guards:** output files are created with
  `O_NOFOLLOW` + mode `0o600` on Unix, and `create_new(true)` so a
  pre-existing file is an error unless `--force` is passed.
- **`--query-file` safety guards:** path is canonicalised, size is capped
  at 10 MiB, and binary extensions (`.exe`, `.dll`, `.so`, `.dylib`,
  `.bin`, `.bat`, `.cmd`, `.com`) are rejected.

### Features

- **Structured logging (F008):** stderr output now flows through
  `tracing-subscriber`; verbosity is controlled by `-v` / `-vv` / `-vvv`
  and `RUST_LOG`. Credentials are redacted by `utils::redact_sql_error`
  before reaching the subscriber.
- **Coloured stderr:** `[DANGER]` / `[WARNING]` banners are coloured when
  stderr is a TTY and `NO_COLOR` is unset.
- **Progress indicators:** spinner during connect / query, row-counter
  during write. Hidden when `--quiet` or stderr is piped.
- **New workflows:** CodeQL, coverage (80% threshold), cargo-audit PR
  gate, benchmarks in CI.

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
