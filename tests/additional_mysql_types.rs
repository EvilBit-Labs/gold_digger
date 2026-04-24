//! Feature-gated coverage for the `additional_mysql_types` feature (todo #076).
//!
//! `Cargo.toml` enables `mysql_common`'s `bigdecimal`, `rust_decimal`,
//! `time`, and `frunk` features through this flag, but no test file
//! references any extended-type surface directly. The integration
//! suite in `tests/integration/data_types.rs` covers them only through
//! a Docker container.
//!
//! This file pins the conversion contract for the value shapes that
//! MySQL/MariaDB produce when `DECIMAL`, `BIGINT`, `TIME`, and
//! `DATETIME` columns are returned: every shape must round-trip
//! through [`gold_digger::TypeTransformer`] without panicking, with
//! NULL preserved, boundary values stable, and JSON output
//! deterministic.
//!
//! Gated on `#[cfg(feature = "additional_mysql_types")]` so a build
//! with the feature explicitly disabled still passes the rest of the
//! suite — the feature is on by default, but a downstream
//! distribution may opt out.

#![cfg(feature = "additional_mysql_types")]

use gold_digger::TypeTransformer;
use mysql::Value;
use serde_json::json;

/// `DECIMAL(p,s)` columns are returned by the mysql crate as
/// `Value::Bytes` containing the decimal's stringified form. The
/// conversion path must preserve every byte (no precision loss), pass
/// negatives through, handle the zero case, and never panic on a
/// payload that has no UTF-8 issues.
#[test]
fn decimal_strings_roundtrip_lossless() {
    let cases: &[&str] = &[
        "0",
        "0.0",
        "-0.0",
        "123456789012345.6789",
        "-123456789012345.6789",
        "0.0000000001",
        "99999999999999999999.99999999",
        // Boundary near MySQL DECIMAL(65,30) maximum precision.
        "9999999999999999999999999999999999.999999999999999999999999999999",
    ];

    for raw in cases {
        let v = Value::Bytes(raw.as_bytes().to_vec());

        // CSV/TSV: the bytes must reach the writer verbatim.
        let s = TypeTransformer::value_to_string(&v).expect("decimal -> string");
        assert_eq!(
            &s, raw,
            "CSV/TSV path corrupted decimal {:?} -> {:?}",
            raw, s
        );

        // JSON: stringly-typed (preserves precision; serde_json::Number
        // would round-trip f64 and lose digits past 53 bits).
        let j = TypeTransformer::value_to_json(&v).expect("value_to_json");
        assert_eq!(
            j,
            json!(raw),
            "JSON path lost precision for decimal {:?} -> {}",
            raw,
            j
        );
    }
}

/// MySQL `DECIMAL(NULL)` arrives as [`Value::NULL`]. The transformer
/// must return the empty string for CSV/TSV and `serde_json::Value::Null`
/// for JSON — never panic, never default to "0".
#[test]
fn decimal_null_yields_empty_and_null() {
    let v = Value::NULL;
    assert_eq!(
        TypeTransformer::value_to_string(&v).expect("null -> string"),
        ""
    );
    assert_eq!(
        TypeTransformer::value_to_json(&v).expect("value_to_json"),
        serde_json::Value::Null
    );
}

/// `BIGINT UNSIGNED` columns reach the transformer as
/// `Value::UInt(u64)`. The boundary value `u64::MAX` must serialize as
/// JSON without overflowing into a float. CSV/TSV paths must also
/// produce the canonical decimal string.
#[test]
fn bigint_unsigned_max_boundary() {
    let v = Value::UInt(u64::MAX);

    let s = TypeTransformer::value_to_string(&v).expect("uint max -> string");
    assert_eq!(s, u64::MAX.to_string());

    let j = TypeTransformer::value_to_json(&v).expect("value_to_json");
    assert_eq!(j, json!(u64::MAX));
}

/// `BIGINT SIGNED` boundaries (i64::MIN/i64::MAX) must round-trip
/// through both output paths without truncation.
#[test]
fn bigint_signed_boundaries() {
    for &n in &[i64::MIN, -1, 0, 1, i64::MAX] {
        let v = Value::Int(n);

        let s = TypeTransformer::value_to_string(&v).expect("int -> string");
        assert_eq!(s, n.to_string(), "string mismatch for {}", n);

        let j = TypeTransformer::value_to_json(&v).expect("value_to_json");
        assert_eq!(j, json!(n), "json mismatch for {}", n);
    }
}

/// `TIME` boundary: MySQL `TIME` ranges from `-838:59:59` to
/// `838:59:59` and supports fractional seconds. The transformer must
/// emit a stable string for every boundary case, sign-handling
/// included. Fractional seconds round-trip via the microseconds field.
#[test]
fn time_boundaries_stable() {
    // Tuple shape: (negative, days, hours, minutes, seconds, microseconds, expected_string).
    // MySQL's Time encoding splits hours into (days, hours_in_day); the
    // transformer aggregates to total hours for the `HH:MM:SS.mmmmmm`
    // format (microseconds are always emitted, padded to six digits).
    let cases: &[(bool, u32, u8, u8, u8, u32, &str)] = &[
        (false, 0, 0, 0, 0, 0, "00:00:00.000000"),
        (false, 0, 1, 2, 3, 0, "01:02:03.000000"),
        (false, 0, 23, 59, 59, 0, "23:59:59.000000"),
        // Negative duration.
        (true, 0, 1, 30, 0, 0, "-01:30:00.000000"),
        // Fractional seconds (microseconds).
        (false, 0, 1, 0, 0, 500_000, "01:00:00.500000"),
        // Days + hours -> aggregated hours (1*24 + 14 = 38).
        (false, 1, 14, 0, 0, 0, "38:00:00.000000"),
    ];

    for &(neg, days, h, m, s, us, expected) in cases {
        let v = Value::Time(neg, days, h, m, s, us);
        let got = TypeTransformer::value_to_string(&v).expect("time -> string");
        assert_eq!(
            got, expected,
            "time conversion mismatch (neg={neg}, days={days}, h={h}, m={m}, s={s}, us={us})"
        );

        // JSON path emits the same canonical string.
        let j = TypeTransformer::value_to_json(&v).expect("value_to_json");
        assert_eq!(
            j,
            json!(expected),
            "JSON time mismatch (neg={neg}, days={days})"
        );
    }
}

/// `DATETIME` with microseconds (MySQL `DATETIME(6)`) must serialise
/// with the documented ISO-8601 `T` separator on the JSON path so
/// downstream parsers (jq, Python, JS) can round-trip without a
/// custom formatter. The CSV/TSV path uses a space separator for
/// human readability.
#[test]
fn datetime_microseconds_roundtrip() {
    // 2026-04-22 14:30:45.123456 UTC.
    let v = Value::Date(2026, 4, 22, 14, 30, 45, 123_456);

    let s = TypeTransformer::value_to_string(&v).expect("datetime -> string");
    assert_eq!(s, "2026-04-22 14:30:45.123456");

    let j = TypeTransformer::value_to_json(&v).expect("value_to_json");
    assert_eq!(j, json!("2026-04-22T14:30:45.123456"));
}

/// NULL handling for every extended-type-bearing variant must
/// converge on the same outputs (empty string / `null`). Pinning this
/// avoids a regression where one variant's NULL accidentally returns
/// `"NULL"` or `"0"` after a refactor.
#[test]
fn extended_types_null_paths_converge() {
    // Use Value::NULL — the unifying shape MySQL returns for any
    // unset extended-type column.
    let v = Value::NULL;
    assert_eq!(TypeTransformer::value_to_string(&v).unwrap(), "");
    assert_eq!(
        TypeTransformer::value_to_json(&v).expect("value_to_json"),
        serde_json::Value::Null
    );
}
