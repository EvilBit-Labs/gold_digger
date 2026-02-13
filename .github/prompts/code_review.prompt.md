---
mode: agent
model: GPT-5 (copilot)
tools: [githubRepo, edit, search, new, runCommands, runTasks, usages, vscodeAPI, think, problems, changes, testFailure, openSimpleBrowser, fetch, extensions, todos, memory]
description: Analyze diff, apply safe internal fixes, report results
---

Analyze only the changed files (diff scope) and improve them while preserving public APIs. Focus categories: (1) Code Smells (large/duplicate/complex functions, excessive nesting) (2) Design Patterns (builder, factory, error handling patterns) (3) Best Practices (Rust idioms, project conventions, CLI tool patterns, zero-warnings policy) (4) Readability (naming, structure, module organization, doc comments) (5) Maintainability (modularization, clarity, separation of concerns) (6) Performance (memory efficiency, iterator patterns, avoiding unnecessary allocations, streaming opportunities) (7) Type Safety (avoid `unwrap()` in production paths, proper `Result` handling, safe MySQL value conversion, NULL handling) (8) Error Handling (`anyhow::Result`, structured errors, no panics, credential redaction, proper context). Context: Gold Digger = Rust CLI tool, MySQL/MariaDB queries, rustls-only TLS, CLI-first config, structured output (CSV/JSON/TSV), security-first, zero-warnings, memory conscious. Prefer clear + secure over clever.

## ACTION WORKFLOW (MANDATORY)

1. Collect diff file list. 2. Analyze per focus category. 3. Classify each finding: `safe-edit` (apply now), `deferred`, `requires-approval`. 4. Auto-apply only `safe-edit` (mechanical, internal, non-breaking, warning removal, correctness, error handling improvements, credential redaction, safe NULL handling, replacing `unwrap()` with proper error handling). 5. Run `just lint` then `just test-no-docker`. On failure: isolate failing hunk, revert it, re-run, document skip. 6. Generate report (summary table, applied edits + rationale, deferred backlog, approval-needed with risks, next-step roadmap). 7. Output unified diff (never commit). If zero safe edits: state "No safe automatic edits applied" and still output full report.

## AUTO-EDIT CONSTRAINTS (STRICT)

- Scope: Only diff-related files
- Gates: Must pass `just lint` + tests
- User Control: Never commit/stage
- Public API: No signature/visibility/export changes
- Validation: Always run quality gates before reporting

## CRITICAL REQUIREMENTS

- Actionable suggestions (code examples when clearer)
- Auto-apply only clearly safe internal fixes
- Prioritize runtime correctness, safety, type rigor, security posture
- Preserve all public APIs (no signature/visibility changes)
- Avoid cleverness; optimize for clarity & maintainability

## REPO RULES (REINFORCED)

Zero warnings (cargo clippy -D warnings) | Safe Rust (avoid `unwrap()` in production, proper `Result` handling) | Precise typing | Safe MySQL value conversion (never `from_value::<String>()`, use `mysql_value_to_string()` or `mysql_value_to_json()`) | CLI-first architecture | `anyhow::Result` for errors | Credential redaction (never log `DATABASE_URL`) | Memory efficient | rustls-only TLS | Path validation | No secrets in logs | Doc comments (`///`) for all public APIs | No panics in production code

---

## EXECUTION CHECKLIST

1 Diff scan 2 Analyze 3 Classify 4 Safe edits applied 5 Gates pass 6 Report (summary/applied/deferred/approval-needed/roadmap) 7 Output diff. On blocker: report + remediation guidance.

## QUICK REFERENCE MATRIX

Category -> Examples of Safe Edits:

- Smells: remove dead code, split oversized internal function (no visibility change), reduce nesting
- Patterns: introduce small private helper function internally, extract common error handling
- Best Practices: replace `unwrap()` with proper `Result` handling, use `?` operator for error propagation, add doc comments
- Readability: rename local vars (non-public), add doc comments (`///`), improve module organization
- Maintainability: extract internal module or helper function (keep public API stable), improve error context
- Performance: eliminate needless allocations, use iterators instead of loops, avoid unnecessary clones, consider streaming for large datasets
- Type Safety: replace `unwrap()` with `?` or proper error handling, ensure safe MySQL value conversion (use `mysql_value_to_string()` or `mysql_value_to_json()`), handle NULL values properly
- Error Handling: add context via `anyhow::Context`, convert panics to errors, ensure credential redaction in error messages, use structured exit codes

If ambiguity arises, default to: classify (deferred) instead of applying.
