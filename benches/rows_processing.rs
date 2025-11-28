use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use gold_digger::rows_to_strings;
use mysql::{OptsBuilder, Pool, prelude::*};

/// Helper function to create a test database connection for generating rows
/// Returns None if database is not available (benchmarks will be skipped)
fn create_test_pool() -> Option<Pool> {
    let opts = OptsBuilder::default()
        .ip_or_hostname(Some("127.0.0.1"))
        .tcp_port(3306)
        .user(Some("root"))
        .pass(Some(""))
        .db_name(Some("test"));

    Pool::new(opts).ok()
}

/// Generate test rows with specified dimensions
fn generate_test_rows(pool: &Pool, num_rows: usize, num_cols: usize) -> Option<Vec<mysql::Row>> {
    let mut conn = pool.get_conn().ok()?;

    // Create a temporary table
    conn.query_drop(format!(
        "CREATE TEMPORARY TABLE bench_data (
                {}
            )",
        (0..num_cols)
            .map(|i| format!("col_{} VARCHAR(255)", i))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .ok()?;

    // Insert test data
    for row_idx in 0..num_rows {
        let values: Vec<String> = (0..num_cols)
            .map(|col_idx| format!("value_{}_{}", row_idx, col_idx))
            .collect();
        let placeholders = (0..num_cols).map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("INSERT INTO bench_data VALUES ({})", placeholders);
        conn.exec_drop(&query, values).ok()?;
    }

    // Fetch all rows
    conn.query("SELECT * FROM bench_data").ok()
}

/// Generate null-heavy test rows
fn generate_null_heavy_rows(
    pool: &Pool,
    num_rows: usize,
    num_cols: usize,
) -> Option<Vec<mysql::Row>> {
    let mut conn = pool.get_conn().ok()?;

    conn.query_drop(format!(
        "CREATE TEMPORARY TABLE bench_data_null (
                {}
            )",
        (0..num_cols)
            .map(|i| format!("col_{} VARCHAR(255)", i))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .ok()?;

    for row_idx in 0..num_rows {
        let values: Vec<Option<String>> = (0..num_cols)
            .map(|col_idx| {
                // Every other column is NULL
                if col_idx % 2 == 0 {
                    Some(format!("value_{}_{}", row_idx, col_idx))
                } else {
                    None
                }
            })
            .collect();
        let placeholders = (0..num_cols).map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("INSERT INTO bench_data_null VALUES ({})", placeholders);
        conn.exec_drop(&query, values).ok()?;
    }

    conn.query("SELECT * FROM bench_data_null").ok()
}

/// Generate rows with mixed MySQL value types
fn generate_mixed_type_rows(pool: &Pool, num_rows: usize) -> Option<Vec<mysql::Row>> {
    let mut conn = pool.get_conn().ok()?;

    conn.query_drop(
        "CREATE TEMPORARY TABLE bench_data_mixed (
            col_int INT,
            col_uint BIGINT UNSIGNED,
            col_float FLOAT,
            col_double DOUBLE,
            col_varchar VARCHAR(255),
            col_text TEXT,
            col_date DATE,
            col_datetime DATETIME,
            col_time TIME,
            col_null VARCHAR(255)
        )",
    )
    .ok()?;

    for row_idx in 0..num_rows {
        conn.exec_drop(
            "INSERT INTO bench_data_mixed VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                row_idx as i32,
                row_idx as u64,
                row_idx as f32 * 1.5,
                row_idx as f64 * 2.5,
                format!("text_{}", row_idx),
                format!("long_text_{}", row_idx),
                mysql::Value::Date(2023, 12, 25, 0, 0, 0, 0),
                mysql::Value::Date(2023, 12, 25, 14, 30, 45, 123456),
                mysql::Value::Time(false, 0, 14, 30, 45, 0),
                mysql::Value::NULL,
            ),
        )
        .ok()?;
    }

    conn.query("SELECT * FROM bench_data_mixed").ok()
}

fn benchmark_rows_processing(c: &mut Criterion) {
    let pool = match create_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("Warning: Database not available, skipping rows_processing benchmarks");
            return;
        }
    };

    let mut group = c.benchmark_group("rows_to_strings");

    // Small dataset: 10 rows, 5 columns
    if let Some(small_rows) = generate_test_rows(&pool, 10, 5) {
        group.throughput(Throughput::Elements(small_rows.len() as u64));
        group.bench_with_input(BenchmarkId::new("small", "10x5"), &small_rows, |b, rows| {
            b.iter(|| black_box(rows_to_strings(black_box(rows.clone())).unwrap()))
        });
    }

    // Medium dataset: 100 rows, 10 columns
    if let Some(medium_rows) = generate_test_rows(&pool, 100, 10) {
        group.throughput(Throughput::Elements(medium_rows.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("medium", "100x10"),
            &medium_rows,
            |b, rows| b.iter(|| black_box(rows_to_strings(black_box(rows.clone())).unwrap())),
        );
    }

    // Large dataset: 1000 rows, 20 columns
    if let Some(large_rows) = generate_test_rows(&pool, 1000, 20) {
        group.throughput(Throughput::Elements(large_rows.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("large", "1000x20"),
            &large_rows,
            |b, rows| b.iter(|| black_box(rows_to_strings(black_box(rows.clone())).unwrap())),
        );
    }

    // Wide dataset: 10 rows, 50 columns
    if let Some(wide_rows) = generate_test_rows(&pool, 10, 50) {
        group.throughput(Throughput::Elements(wide_rows.len() as u64));
        group.bench_with_input(BenchmarkId::new("wide", "10x50"), &wide_rows, |b, rows| {
            b.iter(|| black_box(rows_to_strings(black_box(rows.clone())).unwrap()))
        });
    }

    // Null-heavy dataset: 100 rows, 10 columns (50% NULLs)
    if let Some(null_rows) = generate_null_heavy_rows(&pool, 100, 10) {
        group.throughput(Throughput::Elements(null_rows.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("null_heavy", "100x10_50pct_null"),
            &null_rows,
            |b, rows| b.iter(|| black_box(rows_to_strings(black_box(rows.clone())).unwrap())),
        );
    }

    // Mixed types dataset: 100 rows with various MySQL types
    if let Some(mixed_rows) = generate_mixed_type_rows(&pool, 100) {
        group.throughput(Throughput::Elements(mixed_rows.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("mixed_types", "100_rows"),
            &mixed_rows,
            |b, rows| b.iter(|| black_box(rows_to_strings(black_box(rows.clone())).unwrap())),
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_rows_processing);
criterion_main!(benches);
