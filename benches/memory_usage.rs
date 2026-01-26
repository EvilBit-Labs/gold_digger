use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use gold_digger::{csv, json, rows_to_strings, tab};
use mysql::{OptsBuilder, Pool, prelude::*};
use std::io::Cursor;

/// Helper function to create a test database connection for generating rows
fn create_test_pool() -> Pool {
    let opts = OptsBuilder::default()
        .ip_or_hostname(Some("127.0.0.1"))
        .tcp_port(3306)
        .user(Some("root"))
        .pass(Some(""))
        .db_name(Some("test"));

    Pool::new(opts).unwrap_or_else(|_| {
        panic!("Cannot create test pool for benchmarks. Ensure MySQL is available.");
    })
}

/// Generate large dataset for memory benchmarking
fn generate_large_dataset(pool: &Pool, num_rows: usize, num_cols: usize) -> Vec<mysql::Row> {
    let mut conn = pool.get_conn().unwrap();

    conn.query_drop(format!(
        "CREATE TEMPORARY TABLE bench_memory (
                {}
            )",
        (0..num_cols)
            .map(|i| format!("col_{} VARCHAR(255)", i))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .unwrap();

    // Insert in batches for better performance
    let batch_size = 1000;
    for batch_start in (0..num_rows).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(num_rows);
        let mut values_batch = Vec::new();

        for row_idx in batch_start..batch_end {
            let values: Vec<String> = (0..num_cols)
                .map(|col_idx| format!("value_{}_{}", row_idx, col_idx))
                .collect();
            values_batch.push(values);
        }

        // Insert batch
        for values in values_batch {
            let placeholders = (0..num_cols).map(|_| "?").collect::<Vec<_>>().join(", ");
            let query = format!("INSERT INTO bench_memory VALUES ({})", placeholders);
            conn.exec_drop(&query, values).unwrap();
        }
    }

    conn.query("SELECT * FROM bench_memory").unwrap()
}

/// Benchmark end-to-end flow: rows_to_strings + CSV write
fn benchmark_csv_memory(c: &mut Criterion) {
    let pool = create_test_pool();
    let large_rows = generate_large_dataset(&pool, 10000, 10);

    let mut group = c.benchmark_group("csv_memory");
    group.throughput(Throughput::Elements(large_rows.len() as u64));

    group.bench_function("end_to_end_10000_rows", |b| {
        b.iter(|| {
            // Convert rows to strings
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());

            // Write to CSV
            let mut output = Cursor::new(Vec::new());
            csv::write(black_box(string_rows), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    group.finish();
}

/// Benchmark end-to-end flow: rows_to_strings + JSON write
fn benchmark_json_memory(c: &mut Criterion) {
    let pool = create_test_pool();
    let large_rows = generate_large_dataset(&pool, 10000, 10);

    let mut group = c.benchmark_group("json_memory");
    group.throughput(Throughput::Elements(large_rows.len() as u64));

    group.bench_function("end_to_end_10000_rows_compact", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            json::write(black_box(string_rows.clone()), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    group.bench_function("end_to_end_10000_rows_pretty", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            json::write_with_pretty(black_box(string_rows.clone()), &mut output, true).unwrap();
            black_box(output.into_inner())
        })
    });

    group.finish();
}

/// Benchmark end-to-end flow: rows_to_strings + TSV write
fn benchmark_tsv_memory(c: &mut Criterion) {
    let pool = create_test_pool();
    let large_rows = generate_large_dataset(&pool, 10000, 10);

    let mut group = c.benchmark_group("tsv_memory");
    group.throughput(Throughput::Elements(large_rows.len() as u64));

    group.bench_function("end_to_end_10000_rows", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            tab::write(black_box(string_rows), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    group.finish();
}

/// Compare memory and throughput across formats
fn benchmark_format_comparison(c: &mut Criterion) {
    let pool = create_test_pool();
    let large_rows = generate_large_dataset(&pool, 10000, 10);

    let mut group = c.benchmark_group("format_comparison");
    group.throughput(Throughput::Elements(large_rows.len() as u64));

    // CSV
    group.bench_function("csv_10000_rows", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            csv::write(black_box(string_rows), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    // JSON compact
    group.bench_function("json_compact_10000_rows", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            json::write(black_box(string_rows.clone()), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    // JSON pretty
    group.bench_function("json_pretty_10000_rows", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            json::write_with_pretty(black_box(string_rows.clone()), &mut output, true).unwrap();
            black_box(output.into_inner())
        })
    });

    // TSV
    group.bench_function("tsv_10000_rows", |b| {
        b.iter(|| {
            let string_rows = black_box(rows_to_strings(black_box(large_rows.clone())).unwrap());
            let mut output = Cursor::new(Vec::new());
            tab::write(black_box(string_rows), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_csv_memory,
    benchmark_json_memory,
    benchmark_tsv_memory,
    benchmark_format_comparison
);
criterion_main!(benches);
