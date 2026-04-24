use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gold_digger::{csv, json, tab};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Cursor;

/// Convert a `Vec<Vec<String>>` benchmark fixture (first row is headers)
/// into the typed-JSON representation consumed by `json::write` after
/// todo #015. Plain string values stay strings so the bench measures
/// serialisation cost without the deleted inference path.
fn string_rows_to_json_maps(rows: &[Vec<String>]) -> Vec<BTreeMap<String, Value>> {
    if rows.is_empty() {
        return Vec::new();
    }
    let headers = &rows[0];
    rows.iter()
        .skip(1)
        .map(|row| {
            headers
                .iter()
                .zip(row.iter())
                .map(|(h, v)| (h.clone(), Value::String(v.clone())))
                .collect()
        })
        .collect()
}

/// Create test data with specified dimensions
fn create_test_data(num_rows: usize, num_cols: usize) -> Vec<Vec<String>> {
    let mut data = Vec::with_capacity(num_rows + 1);

    // Header row
    let header: Vec<String> = (0..num_cols).map(|i| format!("col_{}", i)).collect();
    data.push(header);

    // Data rows
    for row_idx in 0..num_rows {
        let row: Vec<String> = (0..num_cols)
            .map(|col_idx| format!("value_{}_{}", row_idx, col_idx))
            .collect();
        data.push(row);
    }

    data
}

/// Create test data with null values (empty strings)
fn create_test_data_with_nulls(num_rows: usize, num_cols: usize) -> Vec<Vec<String>> {
    let mut data = Vec::with_capacity(num_rows + 1);

    let header: Vec<String> = (0..num_cols).map(|i| format!("col_{}", i)).collect();
    data.push(header);

    for row_idx in 0..num_rows {
        let row: Vec<String> = (0..num_cols)
            .map(|col_idx| {
                if col_idx % 2 == 0 {
                    format!("value_{}_{}", row_idx, col_idx)
                } else {
                    String::new() // NULL represented as empty string
                }
            })
            .collect();
        data.push(row);
    }

    data
}

/// Create test data with special characters that need escaping
fn create_test_data_with_special_chars(num_rows: usize, num_cols: usize) -> Vec<Vec<String>> {
    let mut data = Vec::with_capacity(num_rows + 1);

    let header: Vec<String> = (0..num_cols).map(|i| format!("col_{}", i)).collect();
    data.push(header);

    let special_chars = [
        "normal_value",
        "value,with,commas",
        "value\"with\"quotes",
        "value\nwith\nnewlines",
        "value\twith\ttabs",
        "value with spaces",
        "value'with'apostrophes",
    ];

    for row_idx in 0..num_rows {
        let row: Vec<String> = (0..num_cols)
            .map(|col_idx| {
                let char_idx = (row_idx + col_idx) % special_chars.len();
                format!("{}_{}", special_chars[char_idx], row_idx)
            })
            .collect();
        data.push(row);
    }

    data
}

fn benchmark_csv_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("csv_write");

    // Small dataset
    let small_data = create_test_data(10, 5);
    group.throughput(Throughput::Elements(small_data.len() as u64));
    group.bench_with_input(BenchmarkId::new("small", "10x5"), &small_data, |b, data| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            csv::write(black_box(data.clone()), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    // Medium dataset
    let medium_data = create_test_data(100, 10);
    group.throughput(Throughput::Elements(medium_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", "100x10"),
        &medium_data,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                csv::write(black_box(data.clone()), &mut output).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Large dataset
    let large_data = create_test_data(1000, 20);
    group.throughput(Throughput::Elements(large_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large", "1000x20"),
        &large_data,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                csv::write(black_box(data.clone()), &mut output).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Wide dataset
    let wide_data = create_test_data(10, 50);
    group.throughput(Throughput::Elements(wide_data.len() as u64));
    group.bench_with_input(BenchmarkId::new("wide", "10x50"), &wide_data, |b, data| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            csv::write(black_box(data.clone()), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    // Special characters dataset
    let special_data = create_test_data_with_special_chars(100, 10);
    group.throughput(Throughput::Elements(special_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("special_chars", "100x10"),
        &special_data,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                csv::write(black_box(data.clone()), &mut output).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    group.finish();
}

fn benchmark_json_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_write");

    // Small dataset - compact
    let small_maps = string_rows_to_json_maps(&create_test_data(10, 5));
    group.throughput(Throughput::Elements(small_maps.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("small_compact", "10x5"),
        &small_maps,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                json::write(black_box(data.clone()), &mut output, false).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Small dataset - pretty
    group.bench_with_input(
        BenchmarkId::new("small_pretty", "10x5"),
        &small_maps,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                json::write(black_box(data.clone()), &mut output, true).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Medium dataset - compact
    let medium_maps = string_rows_to_json_maps(&create_test_data(100, 10));
    group.throughput(Throughput::Elements(medium_maps.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium_compact", "100x10"),
        &medium_maps,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                json::write(black_box(data.clone()), &mut output, false).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Medium dataset - pretty
    group.bench_with_input(
        BenchmarkId::new("medium_pretty", "100x10"),
        &medium_maps,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                json::write(black_box(data.clone()), &mut output, true).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Large dataset - compact
    let large_maps = string_rows_to_json_maps(&create_test_data(1000, 20));
    group.throughput(Throughput::Elements(large_maps.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large_compact", "1000x20"),
        &large_maps,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                json::write(black_box(data.clone()), &mut output, false).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Null-heavy dataset
    let null_maps = string_rows_to_json_maps(&create_test_data_with_nulls(100, 10));
    group.throughput(Throughput::Elements(null_maps.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("null_heavy", "100x10"),
        &null_maps,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                json::write(black_box(data.clone()), &mut output, false).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    group.finish();
}

fn benchmark_tab_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab_write");

    // Small dataset
    let small_data = create_test_data(10, 5);
    group.throughput(Throughput::Elements(small_data.len() as u64));
    group.bench_with_input(BenchmarkId::new("small", "10x5"), &small_data, |b, data| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            tab::write(black_box(data.clone()), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    // Medium dataset
    let medium_data = create_test_data(100, 10);
    group.throughput(Throughput::Elements(medium_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", "100x10"),
        &medium_data,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                tab::write(black_box(data.clone()), &mut output).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Large dataset
    let large_data = create_test_data(1000, 20);
    group.throughput(Throughput::Elements(large_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large", "1000x20"),
        &large_data,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                tab::write(black_box(data.clone()), &mut output).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    // Wide dataset
    let wide_data = create_test_data(10, 50);
    group.throughput(Throughput::Elements(wide_data.len() as u64));
    group.bench_with_input(BenchmarkId::new("wide", "10x50"), &wide_data, |b, data| {
        b.iter(|| {
            let mut output = Cursor::new(Vec::new());
            tab::write(black_box(data.clone()), &mut output).unwrap();
            black_box(output.into_inner())
        })
    });

    // Special characters dataset
    let special_data = create_test_data_with_special_chars(100, 10);
    group.throughput(Throughput::Elements(special_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("special_chars", "100x10"),
        &special_data,
        |b, data| {
            b.iter(|| {
                let mut output = Cursor::new(Vec::new());
                tab::write(black_box(data.clone()), &mut output).unwrap();
                black_box(output.into_inner())
            })
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    benchmark_csv_write,
    benchmark_json_write,
    benchmark_tab_write
);
criterion_main!(benches);
