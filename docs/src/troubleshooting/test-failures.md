# Test Failure Troubleshooting

This guide provides detailed solutions for test execution failures in the Gold Digger project.

## Common Test Failures

### 1. Unit Test Assertion Failures

**Error Pattern:**

```
thread 'test_name' panicked at 'assertion failed: `(left == right)`
  left: `expected_value`,
 right: `actual_value`'
```

**Diagnosis:**

```bash
# Run specific test with detailed output
cargo test test_name -- --exact --nocapture

# Run with backtrace for more context
RUST_BACKTRACE=1 cargo test test_name

# Use nextest for better reporting
just test-nextest
```

**Common Solutions:**

1. **Data Type Conversion Issues:**

   ```rust
   // Problem: MySQL NULL values causing panics
   let value = from_value::<String>(row[column]); // PANICS on NULL

   // Solution: Handle NULL values safely
   let value = match row.get_opt::<String, _>(column) {
       Some(Ok(val)) => val,
       _ => String::new(), // or appropriate default
   };
   ```

2. **Platform-Specific Behavior:**

   ```rust
   // Problem: Different behavior on Windows vs Unix
   #[test]
   fn test_file_paths() {
       let path = "src/main.rs";
       // This might fail on Windows due to path separators
   }

   // Solution: Use Path for cross-platform compatibility
   #[test]
   fn test_file_paths() {
       use std::path::Path;
       let path = Path::new("src").join("main.rs");
   }
   ```

### 2. Database Connection Test Failures

**Error Pattern:**

```
Error: Database connection failed: Access denied for user 'test'@'localhost'
```

**Solutions:**

1. **Use In-Memory Database for Tests:**

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       fn setup_test_db() -> String {
           // Use testcontainers for real MySQL/MariaDB testing
           // For unit tests, prefer mocks/stubs for fast execution
           "mysql://test_user:test_pass@localhost:3306/test_db".to_string()
       }

       #[test]
       fn test_database_query() {
           let db_url = setup_test_db();
           // Test with real MySQL/MariaDB container
       }
   }
   ```

2. **Mock Database Connections:**

   ```rust
   // Use mockall or similar for database mocking
   #[cfg(test)]
   use mockall::predicate::*;

   #[test]
   fn test_with_mock_db() {
       let mut mock_db = MockDatabase::new();
       mock_db
           .expect_query()
           .with(eq("SELECT * FROM test"))
           .returning(|_| Ok(vec![]));
   }
   ```

3. **Test Containers (for integration tests):**

   ```toml
   # In Cargo.toml
   [dev-dependencies]
   testcontainers = "0.15"
   ```

   ```rust
   #[cfg(test)]
   mod integration_tests {
       use testcontainers::*;

       #[test]
       fn test_with_real_mariadb() {
           let docker = clients::Cli::default();
           let mariadb = docker.run(images::mariadb::MariaDb::default());
           let connection_string = format!("mysql://root@127.0.0.1:{}/test", mariadb.get_host_port_ipv4(3306));
           // Test with real MariaDB container
       }
   }
   ```

### 3. Environment Variable Test Issues

**Error Pattern:**

```text
Error: Environment variable DATABASE_URL not found
```

**Solutions:**

1. **Set Test Environment Variables:**

   ```rust
   #[cfg(test)]
   mod tests {
       use std::env;

       #[test]
       fn test_with_env_vars() {
           env::set_var("DATABASE_URL", "mysql://root@localhost:3306/mysql");
           env::set_var("DATABASE_QUERY", "SELECT 1");

           // Run test

           // Clean up
           env::remove_var("DATABASE_URL");
           env::remove_var("DATABASE_QUERY");
       }
   }
   ```

2. **Use Test Configuration:**

   ```rust
   #[cfg(test)]
   fn get_test_config() -> Config {
       Config {
           database_url: "mysql://root@localhost:3306/mysql".to_string(),
           query: "SELECT 1".to_string(),
           output_file: "/tmp/test_output.json".to_string(),
       }
   }
   ```

### 4. File System Test Failures

**Error Pattern:**

```text
Error: Permission denied (os error 13)
Error: No such file or directory (os error 2)
```

**Solutions:**

1. **Use Temporary Directories:**

   ```rust
   #[cfg(test)]
   mod tests {
       use tempfile::TempDir;

       #[test]
       fn test_file_operations() {
           let temp_dir = TempDir::new().unwrap();
           let file_path = temp_dir.path().join("test_output.json");

           // Test file operations

           // temp_dir is automatically cleaned up
       }
   }
   ```

2. **Check File Permissions:**

   ```rust
   #[test]
   fn test_file_creation() {
       use std::fs::OpenOptions;

       let file = OpenOptions::new()
           .create(true)
           .write(true)
           .truncate(true)
           .open("test_file.txt");

       match file {
           Ok(_) => {
               // Test successful file creation
               std::fs::remove_file("test_file.txt").ok();
           },
           Err(e) => panic!("Failed to create test file: {}", e),
       }
   }
   ```

### 5. Concurrent Test Failures

**Error Pattern:**

```text
Error: Resource busy or locked
Error: Test failed due to race condition
```

**Solutions:**

1. **Use Serial Test Execution:**

   ```toml
   # In Cargo.toml
   [dev-dependencies]
   serial_test = "3.0"
   ```

   ```rust
   use serial_test::serial;

   #[test]
   #[serial]
   fn test_shared_resource() {
       // This test runs serially with other #[serial] tests
   }
   ```

2. **Isolate Test Resources:**

   ```rust
   #[test]
   fn test_isolated_resource() {
       let unique_id = std::thread::current().id();
       let resource_name = format!("test_resource_{:?}", unique_id);
       // Use unique resource names per test
   }
   ```

## Integration Test Issues

### 1. CLI Integration Tests

**Error Pattern:**

```text
Error: Command not found or execution failed
```

**Setup:**

```toml
# In Cargo.toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

**Solutions:**

1. **Test CLI Commands:**

   ```rust
   #[cfg(test)]
   mod cli_tests {
       use assert_cmd::Command;
       use predicates::prelude::*;

       #[test]
       fn test_cli_help() {
           let mut cmd = Command::cargo_bin("gold_digger").unwrap();
           cmd.arg("--help")
               .assert()
               .success()
               .stdout(predicate::str::contains("Usage:"));
       }

       #[test]
       fn test_cli_with_invalid_args() {
           let mut cmd = Command::cargo_bin("gold_digger").unwrap();
           cmd.arg("--invalid-flag")
               .assert()
               .failure()
               .stderr(predicate::str::contains("error:"));
       }
   }
   ```

2. **Test with Environment Variables:**

   ```rust
   #[test]
   fn test_cli_with_env_vars() {
       let mut cmd = Command::cargo_bin("gold_digger").unwrap();
       cmd.env("DATABASE_URL", "mysql://test:password@127.0.0.1:3306/test_db")
           .env("DATABASE_QUERY", "SELECT 1")
           .env("OUTPUT_FILE", "/tmp/test.json")
           .assert()
           .success();
   }
   ```

### 2. Output Format Tests

**Error Pattern:**

```text
Error: JSON parsing failed
Error: CSV format validation failed
```

**Solutions:**

1. **Snapshot Testing:**

   ```toml
   # In Cargo.toml
   [dev-dependencies]
   insta = "1.34"
   ```

   ```rust
   #[cfg(test)]
   mod output_tests {
       use insta::assert_snapshot;

       #[test]
       fn test_json_output_format() {
           let output = generate_json_output(test_data());
           assert_snapshot!(output);
       }

       #[test]
       fn test_csv_output_format() {
           let output = generate_csv_output(test_data());
           assert_snapshot!(output);
       }
   }
   ```

2. **Format Validation:**

   ```rust
   #[test]
   fn test_json_validity() {
       let output = generate_json_output(test_data());
       let parsed: serde_json::Value = serde_json::from_str(&output).expect("Generated JSON should be valid");

       assert!(parsed.is_object());
       assert!(parsed["data"].is_array());
   }

   #[test]
   fn test_csv_validity() {
       let output = generate_csv_output(test_data());
       let mut reader = csv::Reader::from_reader(output.as_bytes());

       // Should be able to parse without errors
       for result in reader.records() {
           result.expect("CSV record should be valid");
       }
   }
   ```

## Performance and Timeout Issues

### 1. Test Timeouts

**Error Pattern:**

```text
Error: Test timed out after 60 seconds
```

**Solutions:**

1. **Configure Test Timeout with Nextest:**

   ```toml
   # In nextest.toml
   [profile.default]
   global-defaults.timeout = "120s"
   ```

   ```bash
   # Run tests with nextest
   cargo nextest run
   ```

   **Alternative: Use tokio::time::timeout for async tests:**

   ```rust
   #[tokio::test]
   async fn long_running_async_test() {
       let result = tokio::time::timeout(
           std::time::Duration::from_secs(120),
           async_test_function()
       ).await;

       assert!(result.is_ok());
   }
   ```

2. **Optimize Test Performance:**

   ```rust
   #[test]
   fn optimized_test() {
       // Use smaller test datasets
       let test_data = generate_small_test_dataset();

       // Mock expensive operations
       let mock_service = MockService::new();

       // Test with optimized setup
   }
   ```

### 2. Memory Issues in Tests

**Error Pattern:**

```text
Error: Out of memory
Error: Stack overflow
```

**Solutions:**

1. **Limit Test Data Size:**

   ```rust
   #[test]
   fn test_with_limited_data() {
       const MAX_TEST_ROWS: usize = 1000;
       let test_data: Vec<_> = (0..MAX_TEST_ROWS).map(|i| format!("test_row_{}", i)).collect();

       // Test with limited dataset
   }
   ```

2. **Use Streaming for Large Data:**

   ```rust
   #[test]
   fn test_streaming_processing() {
       // Instead of loading all data into memory
       let data_stream = create_test_data_stream();

       // Process data in chunks
       for chunk in data_stream.chunks(100) {
           process_chunk(chunk);
       }
   }
   ```

## Platform-Specific Test Issues

### 1. Windows-Specific Issues

**Common Problems:**

- Path separator differences (`\` vs `/`)
- Case sensitivity differences
- File locking behavior
- Line ending differences (`\r\n` vs `\n`)

**Solutions:**

1. **Cross-Platform Path Handling:**

   ```rust
   #[test]
   fn test_cross_platform_paths() {
       use std::path::Path;

       let path = Path::new("src").join("main.rs");
       assert!(path.exists());
   }
   ```

2. **Normalize Line Endings:**

   ```rust
   fn normalize_line_endings(s: &str) -> String {
       s.replace("\r\n", "\n").replace('\r', "\n")
   }

   #[test]
   fn test_cli_output_with_assert_cmd() {
       use assert_cmd::Command;
       use predicates::prelude::*;

       let mut cmd = Command::cargo_bin("gold_digger").unwrap();
       let output = cmd.arg("--query").arg("SELECT 1 as test").output().unwrap();

       let stdout = String::from_utf8_lossy(&output.stdout);
       let normalized_stdout = normalize_line_endings(&stdout);

       // Option 1: Compare against normalized expected string
       let expected = "test\n1\n";
       assert_eq!(normalized_stdout, expected);

       // Option 2: Use predicates with normalized output
       let predicate = predicates::str::contains("test");
       assert!(predicate.eval(&normalized_stdout));
   }

   #[test]
   fn test_cli_output_with_predicates() {
       use assert_cmd::Command;
       use predicates::prelude::*;

       let mut cmd = Command::cargo_bin("gold_digger").unwrap();
       cmd.arg("--query")
           .arg("SELECT 'hello' as message")
           .assert()
           .success()
           .stdout(predicates::str::contains("hello"));

       // For exact matching, normalize both expected and actual
       let expected = normalize_line_endings("message\nhello\n");
       let actual = normalize_line_endings(&String::from_utf8_lossy(&cmd.output().unwrap().stdout));
       assert_eq!(actual, expected);
   }
   ```

   **Note:** When testing CLI output, always normalize line endings on both the expected and actual output strings, or use predicates that handle line ending differences. This ensures tests pass consistently across Windows (CRLF), Unix (LF), and macOS (historically CR) platforms.

### 2. macOS-Specific Issues

**Common Problems:**

- Case-insensitive filesystem by default
- Different system library versions
- Apple Silicon vs Intel differences

**Solutions:**

1. **Handle Case Sensitivity:**

   ```rust
   #[test]
   fn test_case_sensitive_operations() {
       // Ensure test works on both case-sensitive and case-insensitive filesystems
       let file1 = "test_file.txt";
       let file2 = "TEST_FILE.TXT";

       // Don't assume these are different files
   }
   ```

### 3. Linux Distribution Differences

**Common Problems:**

- Different package versions
- Library path differences
- Permission model variations

**Solutions:**

1. **Use Standard Library Paths:**

   ```rust
   #[test]
   fn test_library_loading() {
       // Use standard system paths
       let lib_paths = ["/usr/lib", "/usr/local/lib", "/lib"];

       // Test library loading from standard paths
   }
   ```

## Test Organization and Best Practices

### 1. Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for individual functions
    mod unit_tests {
        use super::*;

        #[test]
        fn test_function_name() {
            // Test individual function
        }
    }

    // Integration tests for module interactions
    mod integration_tests {
        use super::*;

        #[test]
        fn test_module_integration() {
            // Test module interactions
        }
    }

    // Helper functions for tests
    fn setup_test_environment() -> TestEnvironment {
        // Common test setup
    }

    fn cleanup_test_environment(env: TestEnvironment) {
        // Common test cleanup
    }
}
```

### 2. Test Data Management

```rust
#[cfg(test)]
mod test_data {
    pub fn sample_mysql_rows() -> Vec<mysql::Row> {
        // Generate sample MySQL rows for testing
    }

    pub fn sample_json_data() -> serde_json::Value {
        // Generate sample JSON data
    }

    pub fn sample_csv_data() -> String {
        // Generate sample CSV data
    }
}
```

### 3. Test Configuration

```rust
#[cfg(test)]
mod test_config {
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn setup() {
        INIT.call_once(|| {
            // Initialize test environment once
            env_logger::init();
            setup_test_database();
        });
    }
}
```

## Debugging Test Failures

### 1. Verbose Test Output

```bash
# Run tests with verbose output
cargo test -- --nocapture

# Run specific test with backtrace
RUST_BACKTRACE=1 cargo test test_name

# Use nextest for better reporting
cargo nextest run --verbose
```

### 2. Test Debugging Tools

```rust
#[cfg(test)]
mod debug_helpers {
    pub fn debug_print_data<T: std::fmt::Debug>(data: &T) {
        if cfg!(test) {
            println!("Debug data: {:#?}", data);
        }
    }

    pub fn assert_with_debug<T: std::fmt::Debug + PartialEq>(left: &T, right: &T, context: &str) {
        if left != right {
            println!("Assertion failed in context: {}", context);
            println!("Left: {:#?}", left);
            println!("Right: {:#?}", right);
            panic!("Assertion failed");
        }
    }
}
```

### 3. Test Isolation

```rust
#[cfg(test)]
mod isolated_tests {
    use std::sync::Mutex;

    // Use mutex for tests that can't run concurrently
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_requiring_isolation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        // Test that requires exclusive access
    }
}
```

## Prevention Strategies

### 1. Test-Driven Development

```rust
// Write test first
#[test]
fn test_new_feature() {
    let input = "test input";
    let expected = "expected output";
    let actual = new_feature(input);
    assert_eq!(actual, expected);
}

// Then implement the feature
fn new_feature(input: &str) -> String {
    // Implementation
    todo!()
}
```

### 2. Continuous Testing

```bash
# Watch for changes and run tests
cargo watch -x test

# Run tests on every commit
git config core.hooksPath .githooks
```

### 3. Test Coverage

```bash
# Generate coverage report
cargo llvm-cov --html

# Check coverage with JSON output
cargo llvm-cov --json --output-path coverage.json
```

## Getting Help

### Useful Commands

```bash
# Test execution
just test              # Run all tests
just test-nextest      # Run with nextest
cargo test test_name   # Run specific test

# Debugging
RUST_BACKTRACE=1 cargo test
cargo test -- --nocapture
cargo test -- --show-output

# Coverage
just coverage          # HTML coverage report
just coverage-llvm     # CI-compatible coverage
```

### Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Nextest Documentation](https://nexte.st/)
- [Assert CMD Documentation](https://docs.rs/assert_cmd/)
- [Testcontainers Rust](https://docs.rs/testcontainers/)
