---
title: cargo audit deny-warnings divergence between local and CI
date: 2026-06-23
category: tooling-decisions
module: .cargo/audit.toml, .github/workflows/cargo-audit-pr.yml, deny.toml
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - cargo audit exits 0 locally but the CI cargo-audit job fails on the same advisory
  - 'actions-rust-lang/audit runs with denyWarnings: true'
  - A transitive dependency has an unmaintained or unsound advisory with no patched version
  - cargo deny check advisories passes but the audit CI gate still fails
related_components:
  - tooling
tags:
  - cargo-audit
  - cargo-deny
  - ci
  - rustsec
  - deny-warnings
  - unmaintained-advisory
  - transitive-dependency
---

# cargo audit deny-warnings divergence between local and CI

## Context

A `cargo audit` CI gate went red while a bare local `cargo audit` on the same
`Cargo.lock` exited 0. The green terminal invites you to push anyway, then the
PR gate fails and you burn a round-trip guessing why.

The root cause is a default-severity mismatch, not a stale lockfile:

- **Local** — bare `cargo audit` treats *unmaintained* advisories as warnings
  and exits 0. Warnings do not fail by default.
- **CI** — `.github/workflows/cargo-audit-pr.yml` (and the scheduled
  `audit.yml`) run `actions-rust-lang/audit` with `denyWarnings: true`, which
  promotes every warning — including unmaintained — to a failing exit code.

Both runs agree on the facts; they disagree on what the facts *mean*. The
trigger here was `RUSTSEC-2026-0173` (`proc-macro-error2` unmaintained, **no
patched version**), pulled in transitively as a build-time proc-macro.

There is a second, quieter divergence: `cargo deny check advisories` passed
throughout (`deny.toml` has `ignore = []`) because cargo-deny treats
unmaintained as non-error by default. A green `deny` check does **not** imply a
green `audit` check.

## Guidance

### Reproduce the CI gate locally before pushing

Bare `cargo audit` is not the gate. Run the deny-warnings variant, which matches
what `cargo-audit-pr.yml` actually does:

```bash
cargo audit --deny warnings
# or, through the project's tool manager:
mise exec -- cargo audit --deny warnings
```

Exit 1 before a fix is in place, exit 0 after. Use this as the local acceptance
criterion. Run the cargo-deny gate independently — the two tools are not
proxies for each other:

```bash
cargo audit --deny warnings     # CI cargo-audit gate equivalent
cargo deny check advisories     # deny.toml gate
```

### Decide: ignore or upgrade?

1. **Is there a patched version?** If a fix exists, upgrade the crate (or the
   upstream that pulls it in). Do not ignore a fixable advisory.
2. **Is it a real vulnerability with no patch?** Evaluate blast radius. If your
   code reaches the vulnerable path, treat it as HIGH — open a tracking issue,
   pin to a known-safe version, or find an alternative crate.
3. **Is it unmaintained / unsound, with no patched version, in a dependency you
   do not control?** This is the ignore case, justified only when **all** hold:
   - No patched version exists.
   - The dependency is transitive (not a direct entry in `Cargo.toml`).
   - It is build-time (proc-macro) or test-only — not shipped in the binary.
   - The unsound/unmaintained code path is not reachable by our code.

### The `.cargo/audit.toml` ignore block shape

New entries join the existing `[advisories] ignore` array. The comment must
answer three questions so future triage is instant: (a) why no fix, (b) who owns
the dependency, (c) what the exposure is.

```toml
[advisories]
ignore = [
  # <crate> is <unmaintained|unsound> -- no patched version exists.
  # Transitive via <upstream> -> <immediate-parent> (<build-time proc-macro|test-only>);
  # a dependency we cannot directly control and which is not shipped in the
  # binary. Waiting for <upstream> to drop it. Re-evaluate if a patch lands.
  "RUSTSEC-20XX-YYYY",
]
```

## Why This Matters

A local-green / CI-red audit gate is a trust killer: engineers learn to wave the
gate off ("probably flapping again") and stop reading advisory details. The
inverse is just as bad — relying on bare `cargo audit` gives false confidence
and blocks the PR gate after the fact, burning review cycles on a dependency you
cannot fix. **Unmaintained does not mean vulnerable**: the right response to a
no-patch unmaintained advisory is a documented ignore entry, not ignoring the
gate and not leaving it red indefinitely.

## When to Apply

- Before pushing any commit that adds, removes, or upgrades a dependency.
- When `cargo audit` / CI is red and you cannot reproduce locally — run
  `cargo audit --deny warnings` first.
- When a new RUSTSEC advisory drops for a crate already in the tree, even if you
  did not change `Cargo.lock` (the scheduled `audit.yml` will catch these).
- When onboarding a contributor: tell them the gate is the `--deny warnings`
  variant, not the bare command.

## Examples

### Reproducing the CI failure locally

```text
$ cargo audit
...
Crate:     proc-macro-error2
Warning:   unmaintained
ID:        RUSTSEC-2026-0173
$ echo $?
0          # bare cargo audit -- false confidence; CI will still fail

$ cargo audit --deny warnings
...
error: 1 denied warning found!
$ echo $?
1          # matches the CI gate -- this is the real signal
```

### Tracing the transitive path

```bash
cargo tree -i proc-macro-error2
# proc-macro-error2 v2.0.1
# └── mysql-common-derive v0.32.1 (proc-macro)
#     └── mysql_common v0.37.3
#         ├── gold_digger
#         └── mysql v28.0.0
#             └── gold_digger
```

### Before (CI fails) vs after (CI passes)

```toml
# before -- RUSTSEC-2026-0173 not listed, cargo-audit-pr.yml is red
[advisories]
ignore = [
  "RUSTSEC-2025-0134",
  "RUSTSEC-2026-0002",
  "RUSTSEC-2026-0097",
]
```

```toml
# after -- entry added with rationale, gate green
[advisories]
ignore = [
  "RUSTSEC-2025-0134",
  "RUSTSEC-2026-0002",
  "RUSTSEC-2026-0097",

  # proc-macro-error2 is unmaintained (no patched version exists).
  # Transitive via mysql_common -> mysql-common-derive (proc-macro); a
  # build-time dependency we cannot directly control and which is not shipped
  # in the binary. Waiting for mysql to drop it.
  "RUSTSEC-2026-0173",
]
```

## Related

- `.cargo/audit.toml` — the live ignore list this learning maintains.
- `.github/workflows/cargo-audit-pr.yml` — the PR gate using `denyWarnings: true`.
- `.github/workflows/audit.yml` — the daily scheduled audit.
- `deny.toml` / `.github/workflows/security.yml` — the cargo-deny side; note the
  unmaintained-severity disagreement described above.
- GitHub issue #223 — `RUSTSEC-2026-0173: proc-macro-error2 is unmaintained`
  (the triggering advisory).
- `docs/solutions/best-practices/replace-substring-error-classifiers-with-typed-enums-2026-04-25.md`
  — a different root cause but the same "CI fails / local passes" shape of surprise.
