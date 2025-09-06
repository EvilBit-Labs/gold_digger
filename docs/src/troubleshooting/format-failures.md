# Format and Linting Failure Troubleshooting

This guide provides comprehensive solutions for code formatting violations, linting warnings, and
style issues in the Gold Digger project.

## Quick Reference

| Failure Type | Common Cause                      | Quick Fix                    |
| ------------ | --------------------------------- | ---------------------------- |
| Format       | Code not formatted with rustfmt   | `just format`                |
| Clippy       | Linting warnings (zero-tolerance) | `just fix`                   |
| Pre-commit   | Hook violations                   | `pre-commit run --all-files` |
| Style        | Import/organization issues        | Manual fixes + `just format` |
| Line Length  | Lines exceed 100 characters       | Refactor or break lines      |

## Code Formatting Issues

### rustfmt Formatting Violations

**Error Pattern:**

```console
error: rustfmt check failed
Diff in /path/to/file.rs at line 42:
 fn example() {
-    let x=1;
+    let x = 1;
 }
```

**Solutions:**

1. **Auto-format Code:**

   ```bash
   # Format all code
   just format

   # Or use cargo fmt directly
   cargo fmt

   # Format specific file
   cargo fmt -- src/main.rs
   ```

2. **Check Formatting:**

   ```bash
   # Check if formatting is needed
   just fmt-check

   # Or use cargo fmt directly
   cargo fmt --check
   ```

3. **Configure rustfmt:**

   ```toml
   # In rustfmt.toml
   max_width = 100
   hard_tabs = false
   tab_spaces = 4
   newline_style = "Unix"
   use_small_heuristics = "Default"
   reorder_imports = true
   reorder_modules = true
   remove_nested_parens = true
   edition = "2021"
   ```

### Line Length Violations

**Error Pattern:**

```text,ignore
error: line longer than 100 characters
   --> src/main.rs:42:1
    |
42  | let very_long_variable_name = some_function_with_many_parameters(param1, param2, param3, param4);
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Solutions:**

1. **Break Long Lines:**

   ```rust,ignore
   // Problem: Long line
   let result = some_function_with_many_parameters(param1, param2, param3, param4);

   // Solution: Break into multiple lines
   let result = some_function_with_many_parameters(
       param1,
       param2,
       param3,
       param4,
   );
   ```

2. **Extract Variables:**

   ```rust,ignore
   // Problem: Long expression
   let result = calculate_something(data.field1, data.field2, data.field3, other_data.field);

   // Solution: Extract intermediate variables
   let field1 = data.field1;
   let field2 = data.field2;
   let field3 = data.field3;
   let other_field = other_data.field;
   let result = calculate_something(field1, field2, field3, other_field);
   ```

3. **Use Builder Pattern:**

   ```rust,ignore
   // Problem: Long constructor call
   let config = Configuration::new(param1, param2, param3, param4, param5, param6);

   // Solution: Builder pattern
   let config = Configuration::builder()
       .param1(param1)
       .param2(param2)
       .param3(param3)
       .param4(param4)
       .param5(param5)
       .param6(param6)
       .build();
   ```

### Import Organization Issues

**Error Pattern:**

```
error: imports not properly organized
error: unused import
error: import order violation
```

**Solutions:**

1. **Organize Imports:**

   ```rust
   // Correct import organization

   // Standard library imports
   use std::collections::HashMap;
   use std::env;
   use std::fs::File;

   // External crate imports
   use anyhow::Result;
   use clap::Parser;
   use serde_json::Value;

   // Local module imports
   use crate::config::Config;
   use crate::database::Connection;
   ```

2. **Remove Unused Imports:**

   ```bash
   # Detect unused imports with clippy
   cargo clippy -- -W unused-imports

   # Auto-fix unused imports (optional)
   cargo fix --allow-dirty --allow-staged
   ```

3. **Configure Import Settings:**

   ```toml
   # In rustfmt.toml
   reorder_imports = true
   reorder_modules = true
   imports_layout = "Vertical"
   group_imports = "StdExternalCrate"
   ```

## Clippy Linting Issues

### Zero-Tolerance Policy Violations

**Error Pattern:**

```
error: this expression can be simplified
  --> src/main.rs:42:5
   |
42 |     if x == true {
   |        ^^^^^^^^^ help: try: `if x`
   |
   = note: `-D clippy::bool-comparison` implied by `-D warnings`
```

**Solutions:**

1. **Auto-fix Clippy Issues:**

   ```bash
   # Auto-fix clippy warnings
   just fix

   # Or use cargo clippy directly
   cargo clippy --fix --allow-dirty --allow-staged
   ```

2. **Manual Fixes for Common Issues:**

   **Boolean Comparisons:**

   ```rust,ignore
   // Problem
   if x == true {
       // do something
   }
   if y == false {
       // do something
   }

   // Solution
   if x {
       // do something
   }
   if !y {
       // do something
   }
   ```

   **Unnecessary Returns:**

   ```rust
   // Problem
   fn get_value() -> i32 {
       return 42;
   }

   // Solution
   fn get_value() -> i32 {
       42
   }
   ```

   **String Comparisons:**

   ```rust,ignore
   // Problem
   if name == "test".to_string() {
       // do something
   }

   // Solution
   if name == "test" {
       // do something
   }
   ```

   **Option/Result Handling:**

   ```rust,ignore
   // Problem
   match result {
       Ok(value) => Some(value),
       Err(_) => None,
   }

   // Solution
   result.ok()
   ```

### Performance Lints

**Error Pattern:**

```
error: you are cloning a `Copy` type
error: this `.into_iter()` call is redundant
error: consider using `retain` instead of this pattern
```

**Solutions:**

1. **Avoid Unnecessary Cloning:**

   ```rust,ignore
   // Problem
   let x = 42;
   let y = x.clone();  // i32 is Copy, not Clone

   // Solution
   let x = 42;
   let y = x;  // Just copy
   ```

2. **Optimize Iterations:**

   ```rust,ignore
   // Problem
   for item in vec.into_iter() {
       // process item
   }  // Redundant into_iter()

   // Solution
   for item in vec {
       // process item
   }  // Direct iteration
   ```

3. **Use Efficient Methods:**

   ```rust,ignore
   // Problem
   let mut vec = vec![1, 2, 3, 4, 5];
   vec = vec.into_iter().filter(|&x| x > 2).collect();

   // Solution
   let mut vec = vec![1, 2, 3, 4, 5];
   vec.retain(|&x| x > 2);
   ```

### Correctness Lints

**Error Pattern:**

```
error: this looks like you are trying to swap `a` and `b`
error: this comparison involving the minimum or maximum element for this type contains a case that is always true or always false
```

**Solutions:**

1. **Proper Swapping:**

   ```rust,ignore
   // Problem
   let temp = a;
   a = b;
   b = temp;

   // Solution
   std::mem::swap(&mut a, &mut b);
   ```

2. **Correct Comparisons:**

   ```rust,ignore
   // Problem
   if x >= std::i32::MIN {
       // Always true
   }

   // Solution: Remove unnecessary comparison or use proper bounds
   if x > some_meaningful_value {
       // do something
   }
   ```

### Style Lints

**Error Pattern:**

```
error: consider using `writeln!` instead
error: this `match` can be collapsed into the outer `match`
error: consider using an `if let` instead of a `match`
```

**Solutions:**

1. **Simplify Match Statements:**

   ```rust,ignore
   // Problem
   match option {
       Some(value) => {
           match value {
               42 => println!("Found 42"),
               _ => println!("Other value"),
           }
       }
       None => println!("No value"),
   }

   // Solution
   match option {
       Some(42) => println!("Found 42"),
       Some(_) => println!("Other value"),
       None => println!("No value"),
   }
   ```

2. **Use if let for Simple Matches:**

   ```rust,ignore
   // Problem
   match option {
       Some(value) => println!("Value: {}", value),
       None => {},
   }

   // Solution
   if let Some(value) = option {
       println!("Value: {}", value);
   }
   ```

## Pre-commit Hook Issues

### Hook Execution Failures

**Error Pattern:**

```
rust-fmt.....................................................................Failed
rust-clippy..................................................................Failed
prettier.....................................................................Failed
```

**Solutions:**

1. **Run Pre-commit Manually:**

   ```bash
   # Run all hooks on all files
   pre-commit run --all-files

   # Run specific hook
   pre-commit run rust-fmt
   pre-commit run rust-clippy
   ```

2. **Update Pre-commit Hooks:**

   ```bash
   # Update hook repositories
   pre-commit autoupdate

   # Reinstall hooks
   pre-commit uninstall
   pre-commit install
   ```

3. **Fix Pre-commit Configuration:**

   ```yaml
   # In .pre-commit-config.yaml
   repos:
     - repo: local
       hooks:
         - id: rust-fmt
           name: rust-fmt
           entry: cargo fmt --check
           language: system
           files: \.rs$

         - id: rust-clippy
           name: rust-clippy
           entry: cargo clippy -- -D warnings
           language: system
           files: \.rs$
           pass_filenames: false
   ```

### Environment Issues

**Error Pattern:**

```
error: command not found: cargo
error: rustfmt not installed
error: clippy not installed
```

**Solutions:**

1. **Install Missing Tools:**

   ```bash
   # Install Rust components
   rustup component add rustfmt clippy

   # Install development tools
   just setup
   ```

2. **Check Tool Versions:**

   ```bash
   # Verify installations
   cargo --version
   rustfmt --version
   cargo clippy --version
   ```

## Advanced Formatting Issues

### Custom Formatting Rules

**Complex formatting scenarios:**

1. **Long Function Signatures:**

   ```rust,ignore
   // Problem: Long function signature
   pub fn process_database_query(
       connection: &mut Connection,
       query: &str,
       parameters: &[Value],
   ) -> Result<Vec<Row>, DatabaseError> {
       // implementation
       Ok(vec![])
   }

   // Solution: Break into multiple lines (same as above)
   pub fn process_database_query(
       connection: &mut Connection,
       query: &str,
       parameters: &[Value],
   ) -> Result<Vec<Row>, DatabaseError> {
       // implementation
       Ok(vec![])
   }
   ```

2. **Complex Generic Constraints:**

   ```rust,ignore
   use std::fmt::{Debug, Display};
   use std::str::FromStr;

   trait MyTrait {}

   // Problem: Long generic constraints
   impl<T: Clone + Debug + Send + Sync + 'static> MyTrait for T where T: Display + FromStr {}

   // Solution: Break constraints
   impl<T> MyTrait for T where T: Clone + Debug + Send + Sync + 'static + Display + FromStr {}
   ```

3. **Long Chain Calls:**

   ```rust,ignore
   // Problem: Long method chain
   let result = data.iter().filter(|x| x.is_valid()).map(|x| x.process()).collect::<Vec<_>>();

   // Solution: Break chain
   let result = data
       .iter()
       .filter(|x| x.is_valid())
       .map(|x| x.process())
       .collect::<Vec<_>>();
   ```

### Macro Formatting

**Macro-specific formatting issues:**

1. **Macro Definitions:**

   ```rust
   // Problem: Poorly formatted macro
   macro_rules! my_macro {
       ($x:expr) => {
           println!("{}", $x);
       };
   }

   // Solution: Proper formatting
   macro_rules! my_macro {
       ($x:expr) => {
           println!("{}", $x);
       };
   }
   ```

2. **Macro Invocations:**

   ```rust
   // Problem: Long macro call
   println!("Processing item {} with value {} and status {}", item.id, item.value, item.status);

   // Solution: Break into multiple lines
   println!("Processing item {} with value {} and status {}", item.id, item.value, item.status,);
   ```

## Configuration Management

### rustfmt Configuration

**Complete rustfmt.toml example:**

```toml
# Rust formatting configuration for Gold Digger

# Basic settings
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"

# Import settings
reorder_imports = true
reorder_modules = true
imports_layout = "Vertical"
group_imports = "StdExternalCrate"

# Code style
use_small_heuristics = "Default"
binop_separator = "Front"
remove_nested_parens = true
normalize_comments = true
normalize_doc_attributes = true

# Function formatting
fn_args_layout = "Tall"
brace_style = "SameLineWhere"
control_brace_style = "AlwaysSameLine"

# Struct and enum formatting
struct_field_align_threshold = 0
enum_discrim_align_threshold = 0

# String formatting
format_strings = false
format_macro_matchers = true

# Comment formatting
comment_width = 80
wrap_comments = true

# Misc
trailing_comma = "Vertical"
trailing_semicolon = true
match_block_trailing_comma = false
blank_lines_upper_bound = 1
blank_lines_lower_bound = 0
```

### Clippy Configuration

**Custom clippy configuration:**

```toml
# In Cargo.toml
[lints.clippy]
# Deny all warnings (zero-tolerance policy)
all = "deny"

# Allow specific lints if needed
too_many_arguments = "allow" # Sometimes necessary for APIs

# Specific lint levels
pedantic = "warn"
nursery = "warn"
cargo = "warn"
```

**Or use clippy.toml:**

```toml
# clippy.toml
avoid-breaking-exported-api = false
msrv = "1.70.0"                     # Minimum supported Rust version
```

## IDE Integration

### VS Code Configuration

**Settings for consistent formatting:**

```json
{
  "rust-analyzer.rustfmt.extraArgs": [
    "--config-path",
    "./rustfmt.toml"
  ],
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": [
    "--",
    "-D",
    "warnings"
  ],
  "editor.formatOnSave": true,
  "editor.rulers": [
    100
  ],
  "files.trimTrailingWhitespace": true,
  "files.insertFinalNewline": true
}
```

### Other IDEs

**IntelliJ IDEA/CLion:**

- Enable "Reformat code" on save
- Set line length to 100 characters
- Enable Rust plugin with clippy integration

**Vim/Neovim:**

```vim
" Auto-format on save
autocmd BufWritePre *.rs :RustFmt

" Set line length
set textwidth=100
set colorcolumn=100
```

## Automation and CI Integration

### GitHub Actions Integration

**Formatting check in CI:**

```yaml
  - name: Check formatting
    run: cargo fmt --check

  - name: Run clippy
    run: cargo clippy -- -D warnings -W clippy::pedantic -W clippy::nursery

  - name: Run pre-commit hooks
    run: pre-commit run --all-files
```

### Local Development Workflow

**Pre-commit setup:**

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run manually
pre-commit run --all-files
```

**Development script:**

```bash
#!/bin/bash
# dev-check.sh - Run before committing

echo "🔍 Running development checks..."

echo "📝 Formatting code..."
just format

echo "🔍 Running clippy..."
just lint

# For comprehensive linting matching CI (optional)
echo "🔍 Running comprehensive clippy..."
cargo clippy -- -D warnings -W clippy::pedantic -W clippy::nursery

echo "🧪 Running tests..."
just test

echo "✅ All checks passed!"
```

## Troubleshooting Common Issues

### Tool Version Conflicts

**Problem:** Different versions of rustfmt/clippy giving different results

**Solution:**

```bash
# Check tool versions
rustfmt --version
cargo clippy --version

# Update to latest stable
rustup update stable
rustup component add rustfmt clippy
```

### Configuration Conflicts

**Problem:** Local configuration differs from CI

**Solution:**

```bash
# Use same configuration as CI
cp .github/rustfmt.toml rustfmt.toml

# Verify configuration
cargo fmt --check
cargo clippy -- -D warnings -W clippy::pedantic -W clippy::nursery
```

### Clippy Lint Group Configuration

**CI vs Local Differences:**

- **Local (just lint):** `cargo clippy -- -D warnings` (standard warnings only)
- **CI (recommended):** `cargo clippy -- -D warnings -W clippy::pedantic -W clippy::nursery`
  (comprehensive)

**Handling Noisy Pedantic/Nursery Lints:**

If pedantic or nursery lints are too noisy, selectively allowlist them in a `.clippy.toml` file:

```toml
# .clippy.toml
# Allow specific pedantic lints that are too noisy
allow = [
  "clippy::missing_docs_in_private_items", # Allow missing docs in private items
  "clippy::too_many_arguments",            # Allow functions with many args
  "clippy::cast_precision_loss",           # Allow precision loss in casts
]

# Or disable entire groups if needed
# allow = ["clippy::pedantic", "clippy::nursery"]
```

### Performance Issues

**Problem:** Formatting/linting takes too long

**Solution:**

```bash
# Format only changed files
git diff --name-only | grep '\.rs$' | xargs cargo fmt --

# Use parallel clippy
cargo clippy --all-targets --jobs $(nproc)
```

### Clippy Lint Group Issues

**Problem:** CI fails with pedantic/nursery lints that don't fail locally

**Solution:**

```bash
# Run the same clippy configuration locally as CI
cargo clippy -- -D warnings -W clippy::pedantic -W clippy::nursery

# If specific lints are too noisy, add them to .clippy.toml
echo 'allow = ["clippy::specific_lint_name"]' >> .clippy.toml

# Or run with specific lint groups disabled
cargo clippy -- -D warnings -W clippy::pedantic  # Only pedantic, no nursery
```

## Prevention Strategies

### Development Workflow

```bash
# Before committing
just format    # Auto-format
just lint      # Check linting
just test      # Run tests

# Or use combined check
just check     # Format + lint + test
```

### Editor Setup

- Configure auto-format on save
- Enable real-time linting
- Set up ruler at 100 characters
- Enable trailing whitespace removal

### Team Standards

- Document formatting standards
- Use consistent tool versions
- Regular tool updates
- Code review for style consistency

## Getting Help

### Useful Commands

```bash
# Formatting
just format           # Auto-format all code
just fmt-check        # Check formatting
cargo fmt --help      # Formatting options

# Linting
just lint             # Run clippy
just fix              # Auto-fix clippy issues
cargo clippy --help   # Clippy options

# Pre-commit
pre-commit run --all-files    # Run all hooks
pre-commit autoupdate         # Update hooks
```

### Resources

- [rustfmt Documentation](https://rust-lang.github.io/rustfmt/)
- [Clippy Lint List](https://rust-lang.github.io/rust-clippy/master/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- [Pre-commit Documentation](https://pre-commit.com/)
