# Gold Digger Performance Benchmarks

This directory contains comprehensive performance benchmarks for Gold Digger's core functionality using Criterion, a statistical benchmarking framework for Rust.

## Purpose

The benchmark suite measures and tracks performance characteristics of:

- **`rows_to_strings`**: MySQL row to string conversion performance
- **Output format writers**: CSV, JSON (compact and pretty), and TSV writing performance
- **Value conversion**: MySQL value type to string conversion performance
- **Memory usage**: End-to-end memory and throughput characteristics across formats

These benchmarks help:

- Detect performance regressions
- Compare performance across different implementations
- Understand scaling characteristics with dataset size
- Guide optimization efforts

## When to Run Benchmarks

Contributors should run benchmarks when:

- Making changes that could affect performance (data processing, output formatting)
- Before submitting pull requests with performance-sensitive changes
- Investigating performance issues reported by users
- Comparing different implementation approaches

## Benchmark Files

### `rows_processing.rs`

Benchmarks the `rows_to_strings` function with various dataset characteristics:

- **Small**: 10 rows × 5 columns
- **Medium**: 100 rows × 10 columns
- **Large**: 1000 rows × 20 columns
- **Wide**: 10 rows × 50 columns
- **Null-heavy**: 100 rows × 10 columns (50% NULL values)
- **Mixed types**: 100 rows with various MySQL data types (INT, BIGINT, FLOAT, DOUBLE, VARCHAR, TEXT, DATE, DATETIME, TIME, NULL)

**Note**: Requires MySQL database connection. Benchmarks will be skipped if database is not available.

### `output_formats.rs`

Benchmarks the output format writers (`csv::write`, `json::write` compact and pretty, `tab::write`) with:

- **Small datasets**: 10 rows × 5 columns
- **Medium datasets**: 100 rows × 10 columns
- **Large datasets**: 1000 rows × 20 columns
- **Wide datasets**: 10 rows × 50 columns
- **Special characters**: Data containing quotes, commas, newlines, tabs
- **Null-heavy data**: Datasets with many empty strings (NULL representation)

Measures throughput in rows/second and output size.

### `value_conversion.rs`

Benchmarks MySQL value to string conversion (`mysql_value_to_string`) for:

- **NULL values**: Empty string conversion
- **Integers**: Signed and unsigned integer conversion
- **Floats**: Float and double conversion
- **Strings**: UTF-8 string conversion
- **Invalid UTF-8**: Binary data requiring hex encoding
- **Date/Time**: Date, datetime, and time value conversion
- **Special floats**: NaN, Infinity, -Infinity handling
- **Large binary blobs**: Binary data of varying sizes (1KB+)

Groups benchmarks by value category for easy comparison.

### `memory_usage.rs`

End-to-end benchmarks combining `rows_to_strings` with format writers on large datasets (10,000 rows):

- **CSV end-to-end**: Full pipeline from rows to CSV output
- **JSON end-to-end (compact)**: Full pipeline to compact JSON
- **JSON end-to-end (pretty)**: Full pipeline to pretty-printed JSON
- **TSV end-to-end**: Full pipeline to TSV output
- **Format comparison**: Side-by-side comparison of all formats

Measures both throughput and memory characteristics.

## Running Benchmarks

### Basic Commands

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench rows_processing
cargo bench --bench output_formats
cargo bench --bench value_conversion
cargo bench --bench memory_usage

# Run specific benchmark group
cargo bench --bench output_formats csv_write
```

### Using Just Recipes

The project provides enhanced benchmark recipes via `just`:

```bash
# Run full benchmark suite (mirrors CI)
just bench

# Run with reduced sample size for faster feedback during development
just bench-quick

# Save current performance as a named baseline
just bench-baseline main-branch

# Compare against a saved baseline to detect regressions
just bench-compare main-branch

# Run a specific benchmark by name
just bench-specific rows_processing

# Open generated HTML report in browser
just bench-report
```

## Understanding Criterion Output

### Console Output

Criterion provides statistical analysis in the console:

```
Benchmarking csv_write/small
time:   [123.45 µs 124.56 µs 125.67 µs]
thrpt:  [7.9568 Melem/s 8.0312 Melem/s 8.1001 Melem/s]
```

- **time**: Time per iteration (with confidence intervals)
- **thrpt**: Throughput (elements/second or rows/second)

### HTML Reports

Detailed HTML reports are generated in `target/criterion/`:

- **Performance trends**: Track performance over time
- **Statistical significance**: Identify meaningful changes vs noise
- **Baseline comparison**: Compare against saved baselines
- **Distribution plots**: Visualize performance distribution
- **Outlier detection**: Identify and investigate outliers

Open reports with: `just bench-report` or manually navigate to `target/criterion/<benchmark-name>/index.html`

## Baseline Management

Baselines allow tracking performance regressions across commits and releases.

### Creating Baselines

```bash
# Create a baseline for the current main branch performance
just bench-baseline main-branch

# Create a baseline before making changes
just bench-baseline before-optimization
```

Baselines are stored in `target/criterion/` as JSON files.

### Comparing Against Baselines

```bash
# Compare current performance against a saved baseline
just bench-compare main-branch
```

Criterion will:

- Highlight statistically significant changes
- Show performance improvements (green) and regressions (red)
- Provide confidence intervals for changes

### When to Update Baselines

Update baselines when:

- Performance improvements are intentionally made
- Algorithm changes affect performance characteristics
- After major refactoring that changes performance profile

**Note**: Baselines should be committed to version control for main branch tracking.

## CI Integration

The GitHub Actions CI workflow includes a `benchmark` job that:

- **Runs on**: Main branch pushes only (not on pull requests)
- **Non-blocking**: Failures don't block PR merges
- **Artifacts**: Uploads HTML reports and baseline data as artifacts
- **Caching**: Caches Criterion baselines for regression detection

### Accessing CI Results

1. Navigate to the workflow run on GitHub
2. Find the `benchmark` job
3. Download the `criterion-results` artifact for HTML reports
4. Download the `criterion-baselines` artifact for baseline data

### Correlating CI and Local Results

To compare local results with CI:

1. Download CI baseline artifacts
2. Extract to `target/criterion/`
3. Run `just bench-compare <baseline-name>` locally
4. Compare HTML reports side-by-side

## Best Practices

### Development Workflow

1. **Before changes**: Run `just bench-baseline before-changes`
2. **Make changes**: Implement your changes
3. **After changes**: Run `just bench-compare before-changes`
4. **Review**: Check for regressions or improvements
5. **Update baseline**: If improvements are confirmed, update main baseline

### Performance Investigation

1. **Identify slow benchmark**: Run full suite and identify slow benchmarks
2. **Isolate**: Run specific benchmark with `just bench-specific <name>`
3. **Profile**: Use `cargo bench --bench <name> -- --profile-time <seconds>` for profiling
4. **Compare**: Use baselines to compare before/after optimization

### CI Integration

- **Main branch**: Baselines are automatically saved and compared
- **Pull requests**: Benchmarks run but don't block merges
- **Artifacts**: Always download and review benchmark artifacts for main branch commits

## Troubleshooting

### Database Connection Required

Some benchmarks (`rows_processing.rs`, `memory_usage.rs`) require a MySQL database connection:

- **Error**: "Cannot create test pool for benchmarks"
- **Solution**: Ensure MySQL is running and accessible at `127.0.0.1:3306` with user `root` and no password, or modify benchmark connection settings

### Baseline Not Found

- **Error**: "Baseline 'name' not found"
- **Solution**: Ensure baseline was created with `just bench-baseline <name>` and exists in `target/criterion/`

### Slow Benchmark Execution

- **Issue**: Benchmarks take too long
- **Solution**: Use `just bench-quick` for faster feedback during development, or run specific benchmarks only

### HTML Reports Not Opening

- **Issue**: `just bench-report` doesn't open browser
- **Solution**: Manually navigate to `target/criterion/<benchmark-name>/index.html` in your browser

## Future Enhancements

Planned improvements to the benchmark suite:

- **Streaming benchmarks**: Measure performance of streaming output (when implemented)
- **Concurrent benchmarks**: Measure performance under concurrent load
- **Memory profiling**: Detailed memory allocation tracking
- **Cross-platform benchmarks**: Compare performance across operating systems
- **Regression detection**: Automated regression alerts in CI
