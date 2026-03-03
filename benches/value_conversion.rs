use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use gold_digger::TypeTransformer;
use mysql::Value;

/// Create NULL values for benchmarking
fn create_null_values(count: usize) -> Vec<Value> {
    vec![Value::NULL; count]
}

/// Create signed integer values only (Value::Int)
fn create_int_values(count: usize) -> Vec<Value> {
    (0..count).map(|i| Value::Int(i as i64)).collect()
}

/// Create unsigned integer values only (Value::UInt)
fn create_uint_values(count: usize) -> Vec<Value> {
    (0..count).map(|i| Value::UInt(i as u64)).collect()
}

/// Create 32-bit float values only (Value::Float)
fn create_float_values(count: usize) -> Vec<Value> {
    (0..count).map(|i| Value::Float(i as f32 * 1.5)).collect()
}

/// Create 64-bit double values only (Value::Double)
fn create_double_values(count: usize) -> Vec<Value> {
    (0..count).map(|i| Value::Double(i as f64 * 2.5)).collect()
}

/// Create valid UTF-8 string values (Value::Bytes)
fn create_string_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| Value::Bytes(format!("string_value_{}", i).into_bytes()))
        .collect()
}

/// Create invalid UTF-8 byte sequences (Value::Bytes)
fn create_invalid_utf8_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            let mut bytes = vec![0xFF, 0xFE, 0xFD];
            bytes.extend_from_slice(&i.to_le_bytes());
            Value::Bytes(bytes)
        })
        .collect()
}

/// Create date-only values (Value::Date with zeroed time components)
fn create_date_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            let day = ((i % 28) + 1) as u8;
            let month = ((i % 12) + 1) as u8;
            let year = (2023 + (i / 365)) as u16;
            Value::Date(year, month, day, 0, 0, 0, 0)
        })
        .collect()
}

/// Create time-only values (Value::Time)
fn create_time_values(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            let hour = (i % 24) as u8;
            let minute = ((i * 2) % 60) as u8;
            let second = ((i * 3) % 60) as u8;
            let microsecond = ((i * 1000) % 1_000_000) as u32;
            Value::Time(false, 0, hour, minute, second, microsecond)
        })
        .collect()
}

/// Create special 32-bit float edge cases (NaN, Inf, -Inf)
fn create_special_float_values() -> Vec<Value> {
    vec![
        Value::Float(f32::NAN),
        Value::Float(f32::INFINITY),
        Value::Float(f32::NEG_INFINITY),
    ]
}

/// Create special 64-bit double edge cases (NaN, Inf, -Inf)
fn create_special_double_values() -> Vec<Value> {
    vec![
        Value::Double(f64::NAN),
        Value::Double(f64::INFINITY),
        Value::Double(f64::NEG_INFINITY),
    ]
}

/// Create large binary blobs (Value::Bytes)
fn create_large_binary_blobs(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            let size = 1024 + (i * 100);
            let bytes: Vec<u8> = (0..size).map(|j| ((i + j) % 256) as u8).collect();
            Value::Bytes(bytes)
        })
        .collect()
}

fn benchmark_value_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("mysql_value_to_string");

    let null_values = create_null_values(1000);
    group.bench_with_input(
        BenchmarkId::new("null", "1000_values"),
        &null_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let int_values = create_int_values(1000);
    group.bench_with_input(
        BenchmarkId::new("int", "1000_values"),
        &int_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let uint_values = create_uint_values(1000);
    group.bench_with_input(
        BenchmarkId::new("uint", "1000_values"),
        &uint_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let float_values = create_float_values(1000);
    group.bench_with_input(
        BenchmarkId::new("float", "1000_values"),
        &float_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let double_values = create_double_values(1000);
    group.bench_with_input(
        BenchmarkId::new("double", "1000_values"),
        &double_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let string_values = create_string_values(1000);
    group.bench_with_input(
        BenchmarkId::new("strings", "1000_values"),
        &string_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let invalid_utf8_values = create_invalid_utf8_values(100);
    group.bench_with_input(
        BenchmarkId::new("invalid_utf8", "100_values"),
        &invalid_utf8_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let date_values = create_date_values(1000);
    group.bench_with_input(
        BenchmarkId::new("date", "1000_values"),
        &date_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let time_values = create_time_values(1000);
    group.bench_with_input(
        BenchmarkId::new("time", "1000_values"),
        &time_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let special_float_values = create_special_float_values();
    group.bench_with_input(
        BenchmarkId::new("special_floats", "3_values"),
        &special_float_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let special_double_values = create_special_double_values();
    group.bench_with_input(
        BenchmarkId::new("special_doubles", "3_values"),
        &special_double_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    let binary_blobs = create_large_binary_blobs(10);
    group.bench_with_input(
        BenchmarkId::new("large_binary", "10_blobs"),
        &binary_blobs,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_string(black_box(value)).unwrap());
                }
            })
        },
    );

    group.finish();
}

fn benchmark_value_to_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("mysql_value_to_json");

    let null_values = create_null_values(1000);
    group.bench_with_input(
        BenchmarkId::new("null", "1000_values"),
        &null_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let int_values = create_int_values(1000);
    group.bench_with_input(
        BenchmarkId::new("int", "1000_values"),
        &int_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let uint_values = create_uint_values(1000);
    group.bench_with_input(
        BenchmarkId::new("uint", "1000_values"),
        &uint_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let float_values = create_float_values(1000);
    group.bench_with_input(
        BenchmarkId::new("float", "1000_values"),
        &float_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let double_values = create_double_values(1000);
    group.bench_with_input(
        BenchmarkId::new("double", "1000_values"),
        &double_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let string_values = create_string_values(1000);
    group.bench_with_input(
        BenchmarkId::new("strings", "1000_values"),
        &string_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let invalid_utf8_values = create_invalid_utf8_values(100);
    group.bench_with_input(
        BenchmarkId::new("invalid_utf8", "100_values"),
        &invalid_utf8_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let date_values = create_date_values(1000);
    group.bench_with_input(
        BenchmarkId::new("date", "1000_values"),
        &date_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let time_values = create_time_values(1000);
    group.bench_with_input(
        BenchmarkId::new("time", "1000_values"),
        &time_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let special_float_values = create_special_float_values();
    group.bench_with_input(
        BenchmarkId::new("special_floats", "3_values"),
        &special_float_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let special_double_values = create_special_double_values();
    group.bench_with_input(
        BenchmarkId::new("special_doubles", "3_values"),
        &special_double_values,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    let binary_blobs = create_large_binary_blobs(10);
    group.bench_with_input(
        BenchmarkId::new("large_binary", "10_blobs"),
        &binary_blobs,
        |b, values| {
            b.iter(|| {
                for value in values {
                    black_box(TypeTransformer::value_to_json(black_box(value)));
                }
            })
        },
    );

    group.finish();
}

criterion_group!(benches, benchmark_value_conversion, benchmark_value_to_json);
criterion_main!(benches);
