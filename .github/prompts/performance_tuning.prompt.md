---
mode: agent
model: GPT-5 (copilot)
tools: [githubRepo, edit, search, new, runCommands, runTasks, usages, vscodeAPI, think, problems, changes, testFailure, openSimpleBrowser, fetch, extensions, todos, memory]
description: Analyze diff for performance, apply safe micro-optimizations, produce report
---

Analyze ONLY changed files (diff scope) for runtime performance characteristics while preserving correctness, public APIs, and security constraints. Apply only clearly safe micro-optimizations.

## FOCUS CATEGORIES

01. Memory Efficiency (unnecessary allocations, large object retention, iterator patterns vs loops, avoiding clones)
02. Iterator Optimization (use iterators instead of manual loops, chain operations efficiently, lazy evaluation)
03. MySQL Query Efficiency (query optimization, connection pooling, result set streaming opportunities, batch operations)
04. String Handling (avoid unnecessary string allocations, use string slices where possible, efficient formatting)
05. Error Handling Performance (avoid expensive error construction, use `?` operator efficiently, minimize error context overhead)
06. Output Format Performance (CSV/JSON/TSV writer efficiency, buffering strategies, streaming opportunities)
07. Memory & Resource Management (avoid memory leaks, proper cleanup, efficient buffer reuse, respect memory limits)
08. Concurrency & Parallelism (parallel processing opportunities, async patterns, connection pool sizing)
09. Caching Strategies (memoization opportunities, result caching, connection reuse)
10. I/O & Network Efficiency (buffered writes, connection pooling, streaming responses, reduce round trips)

## ACTION WORKFLOW (MANDATORY)

1 Diff list → 2 Perf analysis per category → 3 Classify (`safe-edit` / `deferred` / `requires-approval`) → 4 Apply only mechanical, behavior-preserving micro-optimizations (e.g., replace manual loop with iterator, eliminate unnecessary clone, use string slices, add buffering, optimize error handling) → 5 Run `just lint` & `just test-no-docker` → 6 Revert failing hunk if gates fail → 7 Report (summary, applied, deferred, approval-needed, perf notes, next steps) → 8 Output unified diff (no commit).

If zero safe edits: state "No safe performance edits applied" and still produce full report.

## SAFE PERFORMANCE EDIT EXAMPLES

### Memory & Allocations

- Replace manual loops with iterator chains (e.g., `collect()` only when needed)
- Eliminate unnecessary `clone()` calls, use references where possible
- Use string slices (`&str`) instead of owned `String` where possible
- Pre-allocate vectors with known capacity using `Vec::with_capacity()`
- Use `Cow<str>` for conditional ownership when appropriate
- Avoid unnecessary intermediate collections

### Iterator Patterns

- Replace `for` loops with iterator methods (`map`, `filter`, `fold`, etc.)
- Chain iterator operations efficiently (lazy evaluation)
- Use `collect::<Vec<_>>()` only when necessary, prefer iterators
- Use `try_fold` for error handling in iterator chains
- Prefer `find()` and `find_map()` over manual iteration

### MySQL & Database

- Optimize query construction (avoid string concatenation, use parameterized queries)
- Consider connection pool sizing for concurrent operations
- Use streaming for large result sets (when streaming support is available)
- Batch operations where possible
- Avoid loading entire result sets into memory when streaming is possible

### String & Formatting

- Use `format!()` efficiently, avoid repeated allocations
- Prefer `write!()` macro for buffered writers
- Use `Display` trait instead of `Debug` for user-facing output
- Cache formatted strings when reused
- Use `Cow<str>` for conditional formatting

### Error Handling

- Avoid expensive error construction in hot paths
- Use `?` operator efficiently (minimize error conversions)
- Cache error messages when possible
- Use `anyhow::Context` efficiently (avoid deep context chains)

### Output Writers

- Use `BufWriter` for file output (already recommended in project)
- Batch writes when possible
- Reuse buffers where safe
- Consider streaming for large outputs

## AUTO-EDIT CONSTRAINTS (STRICT)

Scope: diff-only | Gates: `just lint` + tests must pass | No commits | No public signature/visibility changes | Validate after edits | No semantic changes

## CRITICAL REQUIREMENTS

- Do not trade readability or security for micro perf
- Never introduce unsafe
- Provide benchmarks only as recommendations (do not add heavy harness automatically)
- Defer structural refactors (module splits) unless trivial & internal
- Avoid premature caching introducing invalidation complexity

## REPO RULES (REINFORCED)

Zero warnings (cargo clippy -D warnings) | Safe Rust (avoid `unwrap()` in production, proper `Result` handling) | Precise typing | Memory efficiency | CLI-first architecture | `anyhow::Result` for errors | Safe MySQL value conversion | Credential redaction | rustls-only TLS | Path validation | No secrets in logs | Doc comments (`///`) for all public APIs | No panics in production code

## EXECUTION CHECKLIST

1 Diff scan 2 Analyze perf 3 Classify 4 Apply safe micro-optimizations 5 Gates pass 6 Report 7 Output diff | On blocker: report & remediate guidance.

## QUICK PERFORMANCE MATRIX

Category → Sample Safe Edit:

- Memory Efficiency → Replace manual loop with iterator, eliminate unnecessary `clone()`, use `Vec::with_capacity()`
- Iterator Patterns → Replace `for` loop with iterator chain, use `find()` instead of manual search, chain operations efficiently
- MySQL Queries → Optimize query construction, consider connection pool sizing, batch operations
- String Handling → Use `&str` instead of `String` where possible, use `write!()` for buffered output, cache formatted strings
- Error Handling → Use `?` operator efficiently, avoid expensive error construction in hot paths
- Output Writers → Use `BufWriter` for file output, batch writes, reuse buffers
- Memory Management → Pre-allocate vectors, avoid unnecessary allocations, use `Cow<str>` for conditional ownership
- Concurrency → Consider parallel processing opportunities, optimize connection pool sizing
- Caching → Memoize expensive computations, cache formatted output, reuse connections
- I/O Efficiency → Use buffered writers, optimize database queries, reduce round trips

Ambiguous? Defer and document.
