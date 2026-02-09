use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use gold_digger::mysql_value_to_string_bench;
use mysql::Value;

/// Create representative MySQL Value variants for benchmarking
fn create_null_values(count: usize) -> Vec<Value> {
    vec![Value::NULL; count]
}

fn create_integer_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            if i % 2 == 0 {
                Value::Int(i as i64)
            } else {
                Value::UInt(i as u64)
            }
        })
        .collect()
}

fn create_float_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            if i % 2 == 0 {
                Value::Float(i as f32 * 1.5)
            } else {
                Value::Double(i as f64 * 2.5)
            }
        })
        .collect()
}

fn create_string_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| Value::Bytes(format!("string_value_{}", i).into_bytes()))
        .collect()
}

fn create_invalid_utf8_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            // Create invalid UTF-8 sequences
            let mut bytes = vec![0xFF, 0xFE, 0xFD];
            bytes.extend_from_slice(&i.to_le_bytes());
            Value::Bytes(bytes)
        })
        .collect()
}

fn create_date_time_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            let day = ((i % 28) + 1) as u8;
            let month = ((i % 12) + 1) as u8;
            let year = (2023 + (i / 365)) as u16;
            let hour = (i % 24) as u8;
            let minute = ((i * 2) % 60) as u8;
            let second = ((i * 3) % 60) as u8;
            let microsecond = ((i * 1000) % 1000000) as u32;

            if i % 3 == 0 {
                // Date only
                Value::Date(year, month, day, 0, 0, 0, 0)
            } else if i % 3 == 1 {
                // DateTime
                Value::Date(year, month, day, hour, minute, second, microsecond)
            } else {
                // Time
                Value::Time(false, 0, hour, minute, second, microsecond)
            }
        })
        .collect()
}

fn create_special_float_values() -> Vec<Value> {
    vec![
        Value::Float(f32::NAN),
        Value::Float(f32::INFINITY),
        Value::Float(f32::NEG_INFINITY),
        Value::Double(f64::NAN),
        Value::Double(f64::INFINITY),
        Value::Double(f64::NEG_INFINITY),
    ]
}

fn create_large_binary_blobs(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            // Create binary data of varying sizes
            let size = 1024 + (i * 100);
            let bytes: Vec<u8> = (0..size).map(|j| ((i + j) % 256) as u8).collect();
            Value::Bytes(bytes)
        })
        .collect()
}

fn benchmark_value_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("mysql_value_to_string");

    // NULL values
    let null_values = create_null_values(1000);
    group.bench_with_input(
        BenchmarkId::new("null", "1000_values"),
        &null_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // Integer values
    let int_values = create_integer_values(1000);
    group.bench_with_input(
        BenchmarkId::new("integers", "1000_values"),
        &int_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // Float values
    let float_values = create_float_values(1000);
    group.bench_with_input(
        BenchmarkId::new("floats", "1000_values"),
        &float_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // String values
    let string_values = create_string_values(1000);
    group.bench_with_input(
        BenchmarkId::new("strings", "1000_values"),
        &string_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // Invalid UTF-8 bytes
    let invalid_utf8_values = create_invalid_utf8_values(100);
    group.bench_with_input(
        BenchmarkId::new("invalid_utf8", "100_values"),
        &invalid_utf8_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // Date and time values
    let date_time_values = create_date_time_values(1000);
    group.bench_with_input(
        BenchmarkId::new("date_time", "1000_values"),
        &date_time_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // Special floating-point values
    let special_float_values = create_special_float_values();
    group.bench_with_input(
        BenchmarkId::new("special_floats", "6_values"),
        &special_float_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    // Large binary blobs
    let binary_blobs = create_large_binary_blobs(10);
    group.bench_with_input(
        BenchmarkId::new("large_binary", "10_blobs"),
        &binary_blobs,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(mysql_value_to_string_bench(black_box(value)).unwrap());
                }
            })
        },
    );

    group.finish();
}

criterion_group!(benches, benchmark_value_conversion);
criterion_main!(benches);
