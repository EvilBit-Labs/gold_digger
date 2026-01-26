---
mode: agent
model: GPT-5 (copilot)
tools: [githubRepo, edit, search, new, runCommands, runTasks, usages, vscodeAPI, think, problems, changes, testFailure, openSimpleBrowser, fetch, extensions, todos, memory]
description: Analyze diff for security posture, apply safe internal hardening edits, produce report
---

Analyze ONLY changed files (diff scope) for security posture and apply clearly safe hardening improvements while preserving all public APIs.

## FOCUS CATEGORIES

01. Credential Protection (never log DATABASE_URL, redact credentials in errors, secure connection string handling)
02. Input Validation & Parsing (CLI arguments, environment variables, file paths, SQL query validation) – reject invalid early, no silent defaults
03. Data Handling & Storage (no secrets logged, path validation, safe file operations, respect system umask)
04. Cryptography & TLS (rustls-only implementation, proper certificate validation, secure TLS configuration, no insecure algorithms)
05. SQL Injection Prevention (parameterized queries, input sanitization, safe query construction, avoid string concatenation)
06. Error Handling & Logging Hygiene (no sensitive leakage, structured context, credential redaction, proper error messages)
07. Dependency & Surface Minimization (avoid unnecessary crates/features, dead code removal, cargo audit, cargo deny)
08. Defense-in-Depth Opportunities (input sanitization, bounds checking, resource ceilings, path validation, connection limits)
09. Security Regression Risks (stubs flagged, TODOs categorized, unimplemented sections clearly documented, security warnings)
10. Supply Chain & Build Hygiene (cargo audit, cargo deny, dependency updates, no unsafe code without justification, rustls-only TLS)

## ACTION WORKFLOW (MANDATORY)

1 Diff list → 2 Security analysis per category → 3 Classify findings (`safe-edit` / `deferred` / `requires-approval`) → 4 Apply only mechanical non-breaking hardening edits (credential redaction in logs/errors, path validation + bound checks, adding missing input validation, adding missing error context, ensuring rustls-only TLS, safe MySQL value conversion) → 5 Run `just lint` & `just test-no-docker` → 6 Revert any failing hunk → 7 Report (summary, applied, deferred, approval-needed, risk notes, roadmap) → 8 Output unified diff (no commit).

If zero safe edits: state "No safe security edits applied" and still emit full report.

## SAFE HARDENING EDIT EXAMPLES

- Redact credentials in error messages and logs (use `redact_database_url()` helper)
- Add input validation for CLI arguments and environment variables
- Inline guard clauses for obvious errors or unchecked access (if internal)
- Validate and sanitize file paths, verify they're within allowed boundaries
- Remove dead code exposing potential attack surface
- Strengthen error messages (no raw system paths, secrets, or sensitive data in errors)
- Add length / size / iteration bounds for unbounded growth structures
- Replace stringly-typed mode flags with enums or const objects
- Ensure all public API doc comments mention security considerations where relevant
- Add input sanitization for user-provided SQL queries (parameterized queries, validation)
- Ensure DATABASE_URL is never logged (always redact)
- Verify rustls-only TLS implementation (no native-tls dependencies)
- Use safe MySQL value conversion (never `from_value::<String>()`, use helpers)

## AUTO-EDIT CONSTRAINTS (STRICT)

Scope: diff-only | Gates: `just lint` + tests must pass | No commits | No public signature/visibility changes | Validate after edits

## CRITICAL REQUIREMENTS

- Preserve functional behavior while reducing risk
- No new dependencies unless strictly necessary for safety
- Avoid speculative rewrites—minimal surface change
- Avoid perf regressions; if added checks are non-trivial mark as deferred
- Do not mask existing errors—surface with context instead

## REPO RULES (REINFORCED)

Zero warnings (cargo clippy -D warnings) | Safe Rust (avoid `unwrap()` in production, proper `Result` handling) | Precise typing | Memory efficiency | CLI-first architecture | `anyhow::Result` for errors | Safe MySQL value conversion | Credential redaction (never log DATABASE_URL) | rustls-only TLS | Path validation | No secrets in logs | Doc comments (`///`) for all public APIs | No panics in production code

## EXECUTION CHECKLIST

1 Diff scan 2 Analyze security 3 Classify 4 Apply safe hardening edits 5 Gates pass 6 Report 7 Output diff | On blocker: report with remediation.

## QUICK SECURITY MATRIX

Category → Sample Safe Edit:

- Credential Protection → Redact DATABASE_URL in error messages using `redact_database_url()`
- Input Validation → Add validation for CLI arguments and environment variables
- Data Handling → Validate + ensure file paths within allowed boundaries, respect system umask
- Cryptography/TLS → Verify rustls-only implementation, ensure proper certificate validation
- SQL Injection → Ensure queries use parameterized inputs, validate SQL input, avoid string concatenation
- Logging → Replace raw error chain with redacted display (no secrets, no DATABASE_URL)
- Resource Bounds → Add comment + bound to array growth pattern, limit connection pool size
- Stub Sections → Mark with `SECURITY_TODO:` prefix for tracking
- MySQL Value Conversion → Use safe helpers (`mysql_value_to_string()`, `mysql_value_to_json()`), never `from_value::<String>()`

Ambiguous? Defer and document.
