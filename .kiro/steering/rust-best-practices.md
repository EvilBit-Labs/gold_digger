---
inclusion: always
---

# Rust Best Practices for gold_digger

## Project Structure

- The main entry point is [`main.rs`](mdc:src/main.rs), which delegates core logic to library code in [`lib.rs`](mdc:src/lib.rs).
- Modules for output formats are organized as separate files: [`csv.rs`](mdc:src/csv.rs), [`json.rs`](mdc:src/json.rs), and [`tab.rs`](mdc:src/tab.rs).
- Shared logic and public APIs are exposed via [`lib.rs`](mdc:src/lib.rs).

## Module Organization

- Each output format (CSV, JSON, Tab) should have its own module with a `write` function that takes rows and an output writer.
- Public functions in modules should be documented with doc comments (`///`).
- Use `pub mod` in [`lib.rs`](mdc:src/lib.rs) to expose modules.

## Error Handling

- Use the [`anyhow`](https://docs.rs/anyhow) crate for error propagation and context, as seen in function signatures like `Result<()>`.
- Avoid panics in production code; prefer returning errors. Only use `panic!` for unrecoverable, truly exceptional cases (e.g., missing header row in [`json.rs`](mdc:src/json.rs)).
- Use `?` for error propagation.

### Database Type Safety (CRITICAL)

```rust
// DANGEROUS - causes runtime panics on NULL/non-string values
from_value::<String>(row[column.name_str().as_ref()])

// SAFE - use Row's safe getters with explicit NULL handling
row.get_opt::<String, _>(column.name_str().as_ref())
    .map(|opt_val| match opt_val {
        Some(val) => val,
        None => "".to_string(),
    })
    .unwrap_or_else(|_| {
        // Fallback for non-string types: convert to string representation
        match row.get::<mysql::Value, _>(column.name_str().as_ref()) {
            Ok(mysql::Value::NULL) => "".to_string(),
            Ok(val) => format!("{:?}", val),
            Err(_) => "".to_string(), // Handle column access errors gracefully
        }
    })
```

## Code Style

- Follow [Rustfmt](https://github.com/rust-lang/rustfmt) conventions for formatting. Run `cargo fmt` before committing.
- Use `snake_case` for function and variable names, `CamelCase` for types and structs.
- Prefer iterators and combinators over manual loops where possible.
- Use explicit types for function signatures, especially for public APIs.
- Group imports by standard library, external crates, and local modules, separated by newlines.

## Features and Conditional Compilation

- Use Cargo features (see `[features]` in [`Cargo.toml`](mdc:Cargo.toml)) to enable/disable output formats and verbose logging.
- Use `#[cfg(feature = "...")]` to conditionally compile code based on enabled features, as in [`main.rs`](mdc:src/main.rs).

## Dependency Management

- Pin dependency versions in [`Cargo.toml`](mdc:Cargo.toml) and use minimal required features for each crate.
- Use optional dependencies and Cargo features with `default = false` for expensive or heavy-weight features (e.g., additional MySQL types, output formats) to keep minimal builds lean.
- Document which features are default-off and why in feature descriptions and project documentation.

## Testing and Safety

- Add tests in a `tests/` directory or as `#[cfg(test)]` modules within each file.
- Validate all external input (e.g., environment variables) and handle missing/invalid values gracefully.
- Prefer returning early on error conditions.

## Documentation

- Keep [`README.md`](mdc:README.md) up to date with usage, features, and examples.
- Document all public functions and modules with doc comments.

## Miscellaneous

- Use `.gitignore` to exclude build artifacts and sensitive files.
- Use `.editorconfig` for consistent editor settings.
- Follow the guidelines in [`CONTRIBUTING.md`](mdc:CONTRIBUTING.md) for code contributions.
