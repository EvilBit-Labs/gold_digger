# GitHub Actions Runner Analysis

## Runner Labels Found in Workflows

### Static Runner Labels

- `ubuntu-latest` - Used in ci.yml (quality, test-tls, test-features, test-tls-validation,
  coverage), docs.yml (build, deploy), security.yml (audit), tls-integration.yml (tls-cli-tests)
- `ubuntu-22.04` - Used in codeql.yml (analyze), release.yml (plan, host, publish-homebrew-formula,
  announce)

### Matrix Runner Labels

- `${{ matrix.os }}` in ci.yml with matrix: `[ubuntu-latest, macos-latest, windows-latest]`
- `${{ matrix.os }}` in tls-integration.yml with matrix:
  `[ubuntu-latest, macos-latest, windows-latest]`
- `${{ matrix.runner }}` in release.yml (dynamically determined by cargo-dist)

### All Unique Runner Labels Used

1. `ubuntu-latest`
2. `ubuntu-22.04`
3. `macos-latest`
4. `windows-latest`

## Act Mapping Coverage

All runner labels are covered by the updated `.actrc` configuration:

```
# Using Ubuntu 22.04 base for all runners to match most workflows and ensure reproducible builds
-P ubuntu-22.04=catthehacker/ubuntu:act-22.04
-P ubuntu-latest=catthehacker/ubuntu:act-22.04
-P macos-13=catthehacker/ubuntu:act-22.04
-P macos-latest=catthehacker/ubuntu:act-22.04
-P windows-2022=catthehacker/ubuntu:act-22.04
-P windows-latest=catthehacker/ubuntu:act-22.04
```

## Decision Rationale

- **Chose Ubuntu 22.04 base**: Most workflows use `ubuntu-latest` or `ubuntu-22.04`, and the release
  workflow specifically uses `ubuntu-22.04`
- **Consistent mapping**: All runners map to the same base image for reproducible local testing
- **GitHub parity**: Ubuntu 22.04 matches GitHub's current ubuntu-latest (as of the analysis date)
- **Simplicity**: Single base image reduces complexity and potential inconsistencies

## Verification Status

✅ All workflow runner labels are covered by act mappings ✅ Consistent Ubuntu 22.04 base across all
mappings ✅ Comment added explaining the choice
