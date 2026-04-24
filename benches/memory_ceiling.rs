//! Historical memory-ceiling baseline for the pre-streaming `rows_to_strings` path.
//!
//! **Status:** Todo #005 landed streaming (`conn.query_iter` + `RowSink`),
//! so `rows_to_strings` is no longer on the live query-execution path.
//! This benchmark is retained as a historical reference for the old
//! fully-buffered conversion so future regression work has something to
//! diff against — it does NOT reflect the production memory profile as
//! of F007.
//!
//! The benchmark measures both:
//!
//! 1. The wall-clock cost of the legacy fully-buffered conversion at
//!    progressively larger row counts (1k / 10k / 100k synthetic rows).
//! 2. An estimate of the resulting `Vec<Vec<String>>` footprint and the
//!    process resident-set size (RSS) via `sysinfo`.
//!
//! Under the streaming pipeline peak RSS is linear in the column count
//! (one `mysql::Row` live at a time), not the row count, so the curves
//! below should be read as "what memory pressure looked like before F007",
//! not "what to expect today".
//!
//! The benchmark is synthetic — it builds `mysql::Row` values in-process
//! via `mysql_common::row::new_row` (accessed through `mysql::mysql_common`)
//! so no live MySQL/MariaDB server is required.
//!
//! Run with: `cargo bench --bench memory_ceiling`

use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gold_digger::rows_to_strings;
use mysql::consts::ColumnType;
use mysql::{Column, Row, Value};
use mysql_common::row::new_row;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, get_current_pid};

/// Number of columns in each synthetic row. Ten columns matches the shape
/// used by `benches/memory_usage.rs` so numbers are comparable.
const NUM_COLS: usize = 10;

/// Approximate byte width of a single string cell in the synthetic data.
/// Used in the report output to derive a raw-payload estimate.
const CELL_WIDTH_BYTES: usize = 32;

/// Build a column descriptor set shared across every synthetic row.
fn build_columns(num_cols: usize) -> Arc<[Column]> {
    (0..num_cols)
        .map(|i| {
            Column::new(ColumnType::MYSQL_TYPE_VAR_STRING).with_name(format!("col_{i}").as_bytes())
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
        .into()
}

/// Build `num_rows` synthetic MySQL rows without touching a database.
///
/// Each row carries `NUM_COLS` `Value::Bytes` cells of roughly
/// `CELL_WIDTH_BYTES` width, simulating a realistic VARCHAR result set.
fn build_synthetic_rows(num_rows: usize) -> Vec<Row> {
    let columns = build_columns(NUM_COLS);
    (0..num_rows)
        .map(|row_idx| {
            let values: Vec<Value> = (0..NUM_COLS)
                .map(|col_idx| {
                    // 32-byte padded payload keeps per-cell size roughly
                    // constant so scaling observations remain linear.
                    let payload = format!("r{row_idx:010}_c{col_idx:02}_xxxxxxxxxxx");
                    Value::Bytes(payload.into_bytes())
                })
                .collect();
            new_row(values, Arc::clone(&columns))
        })
        .collect()
}

/// Best-effort RSS snapshot for the current process.
///
/// Returns 0 when the platform/sysinfo build cannot report the value — the
/// caller should treat 0 as "measurement unavailable" rather than a real
/// zero-byte resident set.
fn current_rss_bytes() -> u64 {
    let Ok(pid) = get_current_pid() else {
        return 0;
    };
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing().with_memory(),
    );
    system.process(pid).map_or(0, |p| p.memory())
}

/// Report the shallow byte size of a `Vec<Vec<String>>` result.
///
/// Walks every inner vector and sums `String::capacity()` plus per-cell
/// `String` overhead. This is an approximation of the heap footprint —
/// it ignores allocator bookkeeping but is sufficient for trend tracking.
fn estimate_result_bytes(result: &[Vec<String>]) -> usize {
    let mut total: usize = std::mem::size_of_val(result);
    for row in result {
        total = total.saturating_add(std::mem::size_of_val(row.as_slice()));
        for cell in row {
            total = total
                .saturating_add(std::mem::size_of::<String>())
                .saturating_add(cell.capacity());
        }
    }
    total
}

/// Print a single size-class summary line to stderr.
///
/// Format: `[memory_ceiling] rows=N cols=M raw_bytes=... result_bytes=... rss_delta_bytes=...`.
fn report(num_rows: usize, result_bytes: usize, rss_before: u64, rss_after: u64) {
    let raw_bytes = num_rows
        .saturating_mul(NUM_COLS)
        .saturating_mul(CELL_WIDTH_BYTES);
    let rss_delta = rss_after.saturating_sub(rss_before);
    eprintln!(
        "[memory_ceiling] rows={num_rows} cols={NUM_COLS} raw_bytes={raw_bytes} \
         result_bytes={result_bytes} rss_before_bytes={rss_before} \
         rss_after_bytes={rss_after} rss_delta_bytes={rss_delta}"
    );
}

/// Criterion benchmark entry point.
fn benchmark_memory_ceiling(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_ceiling");
    // Cap sample sizes to keep the 100k case reasonable on CI runners.
    group.sample_size(10);

    for &num_rows in &[1_000_usize, 10_000, 100_000] {
        let rows = build_synthetic_rows(num_rows);
        group.throughput(Throughput::Elements(num_rows as u64));

        // One-shot baseline measurement before Criterion's warmup loop so
        // we can print a stable size/RSS snapshot for the size class.
        let rss_before = current_rss_bytes();
        let baseline = rows_to_strings(rows.clone()).expect("baseline conversion failed");
        let result_bytes = estimate_result_bytes(&baseline);
        let rss_after = current_rss_bytes();
        drop(baseline);
        report(num_rows, result_bytes, rss_before, rss_after);

        group.bench_with_input(
            BenchmarkId::new("rows_to_strings", num_rows),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let out = rows_to_strings(black_box(rows.clone()))
                        .expect("rows_to_strings failed during bench iteration");
                    black_box(out)
                });
            },
        );
    }

    group.finish();
}

/// Heavy memory-ceiling sweep at 1M / 2M / 3M synthetic rows (todo #083).
///
/// Skipped by default — `cargo bench --bench memory_ceiling` will not run
/// these size classes because Criterion respects the `IGNORE_BENCHES`
/// gate below. Run explicitly with `IGNORE_BENCHES=0` to opt in:
///
/// ```text
/// IGNORE_BENCHES=0 cargo bench --bench memory_ceiling -- memory_ceiling_heavy
/// ```
///
/// Each iteration allocates roughly `num_rows * 10 cols * 32 B = ~960 MB`
/// of synthetic `Value::Bytes` payload PLUS the resulting `Vec<Vec<String>>`,
/// so the 3M-row case needs ~6 GB of headroom. The benchmark records both
/// the estimated result-set size and the process RSS delta so the legacy
/// `rows_to_strings` path is comparable to the new streaming sink (todo
/// #005, F007) — the streaming sink's RSS should stay flat while this
/// curve scales linearly with `num_rows`.
fn benchmark_memory_ceiling_heavy(c: &mut Criterion) {
    // Default skip: respect the `#[ignore]`-equivalent gate so CI doesn't
    // OOM on shared runners. Set `IGNORE_BENCHES=0` to opt in locally.
    let opt_in = std::env::var("IGNORE_BENCHES")
        .map(|v| v == "0")
        .unwrap_or(false);
    if !opt_in {
        eprintln!(
            "[memory_ceiling] skipping heavy 1M/2M/3M sweep \
             (set IGNORE_BENCHES=0 to enable; needs ~6 GB RAM)"
        );
        return;
    }

    let mut group = c.benchmark_group("memory_ceiling_heavy");
    // One sample per size class — the heavy sweep is for trend tracking,
    // not statistical comparison, and each iteration is expensive.
    group.sample_size(10);

    for &num_rows in &[1_000_000_usize, 2_000_000, 3_000_000] {
        let rows = build_synthetic_rows(num_rows);
        group.throughput(Throughput::Elements(num_rows as u64));

        let rss_before = current_rss_bytes();
        let baseline = rows_to_strings(rows.clone()).expect("baseline conversion failed");
        let result_bytes = estimate_result_bytes(&baseline);
        let rss_after = current_rss_bytes();
        drop(baseline);
        report(num_rows, result_bytes, rss_before, rss_after);

        group.bench_with_input(
            BenchmarkId::new("rows_to_strings", num_rows),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let out = rows_to_strings(black_box(rows.clone()))
                        .expect("rows_to_strings failed during bench iteration");
                    black_box(out)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_memory_ceiling,
    benchmark_memory_ceiling_heavy
);
criterion_main!(benches);
