//! Streaming row-sink trait and format-specific implementations (F007).
//!
//! The query-execution pipeline (`src/run.rs`) no longer materialises the
//! full result set into a `Vec<mysql::Row>` before writing. It opens a
//! streaming `conn.query_iter` cursor and feeds rows one at a time into a
//! [`RowSink`] implementation chosen by the requested output format.
//!
//! # Atomic output invariant
//!
//! Streaming + ahead-of-time output truncation would violate the project's
//! atomic-output guarantee: a type-conversion error on row N would leave
//! rows 0..N-1 visible on disk at the target path. The sinks therefore
//! always write to a sibling `<output>.tmp` file and rename on success.
//! On error, the sink deletes the `.tmp` file so the filesystem either
//! has the complete output at `<output>` or no file at all.
//!
//! # Error-class routing
//!
//! Type-conversion failures from [`crate::TypeTransformer`] carry the
//! "Type conversion error" prefix, which maps to
//! [`GoldDiggerError::Query`] (exit 4) via the existing substring /
//! typed classifier in [`crate::exit`]. Filesystem failures inside the
//! sinks surface as [`GoldDiggerError::Io`] (exit 5). The sinks wrap
//! conversion errors in a typed `Query` variant so the exit code stays
//! stable even if the upstream message text is refactored.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use csv::{QuoteStyle, WriterBuilder};

use crate::exit::GoldDiggerError;
use crate::output::create_output_file;

/// 64 KiB write buffer — same size the non-streaming writers used so per-row
/// write overhead stays comparable under the new pipeline.
const WRITE_BUFFER_BYTES: usize = 64 * 1024;

/// Outcome returned by a completed [`RowSink::finalize`] call.
///
/// `rows_written` excludes the header row so callers can compare it
/// against a "no rows" threshold without special-casing CSV/TSV.
pub struct WriteOutcome {
    /// Number of data rows written (excludes the CSV/TSV header row).
    pub rows_written: u64,
}

/// Streaming consumer of MySQL rows.
///
/// Each sink owns its destination file handle (buffered, writing to a
/// sibling `.tmp` path) and commits the result via an atomic rename in
/// [`Self::finalize`]. Conversion errors propagate out of [`Self::on_row`]
/// without touching the target path — the caller's `Drop` impl on the
/// concrete sink cleans the `.tmp` up.
pub trait RowSink {
    /// Called exactly once before any rows. `columns` holds the column
    /// names in query order. CSV/TSV sinks write this as the header line;
    /// JSON emits the opening `{"data":[` marker and remembers the names
    /// for the per-row object keys.
    fn on_headers(&mut self, columns: &[String]) -> Result<()>;

    /// Called once per row in query order. The sink is responsible for
    /// converting the row via `TypeTransformer` and writing it to its
    /// buffered `.tmp` file.
    fn on_row(&mut self, row: &mysql::Row) -> Result<()>;

    /// Called exactly once after all rows (or immediately after
    /// [`Self::on_headers`] if the stream was empty). Flushes the
    /// buffered writer, closes the file, and renames the `.tmp` onto the
    /// target path. Returns the number of data rows written.
    fn finalize(self: Box<Self>) -> Result<WriteOutcome>;
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Builds the sibling `.tmp` path for an output target.
///
/// We append a literal `.tmp` suffix to the full target path (preserving
/// any existing extension) so the sibling file is obviously a transient
/// artefact. Using a fixed suffix (rather than a random one) keeps the
/// failure-cleanup path unambiguous: the rename target is exactly
/// `<output>.tmp`, so a stale `.tmp` from a previous crash reliably
/// collides with `create_new(true)` under the non-`--force` path.
fn temp_path_for(output: &Path) -> PathBuf {
    let mut buf = output.to_path_buf().into_os_string();
    buf.push(".tmp");
    PathBuf::from(buf)
}

/// Verifies the final target path is usable, then opens the sibling
/// `.tmp` file with the same path-safety guards as [`create_output_file`].
///
/// The pre-check for the target is deliberate: without `--force`,
/// `fs::rename` on a path that already exists would *overwrite* the file
/// silently. We refuse early so the contract matches the non-streaming
/// behaviour documented on [`create_output_file`].
fn open_tmp_for(output: &Path, force: bool) -> Result<(PathBuf, BufWriter<File>)> {
    // Pre-flight: if the target exists and `--force` is not set, refuse
    // now — otherwise a successful `.tmp` write followed by `fs::rename`
    // would clobber the existing file.
    if !force && output.exists() {
        return Err(GoldDiggerError::Config(format!(
            "Output file already exists: {}. Pass --force to overwrite.",
            output.display()
        ))
        .into());
    }

    let tmp_path = temp_path_for(output);
    // Always truncate the `.tmp` file: a leftover from a previous crash
    // is meaningless to us, and `--force` semantics apply to the *target*,
    // not the transient sibling. Using `force=true` here is safe because
    // `create_output_file` still enforces `O_NOFOLLOW` on the `.tmp` path.
    let file = create_output_file(&tmp_path, true)?;
    Ok((tmp_path, BufWriter::with_capacity(WRITE_BUFFER_BYTES, file)))
}

/// Best-effort cleanup of a `.tmp` file after a write failure.
///
/// Ignores the result: the underlying write error is already the root
/// cause and a secondary `unlink` failure (e.g. the file was removed
/// underneath us) should not mask it.
fn remove_tmp(tmp_path: &Path) {
    let _ = std::fs::remove_file(tmp_path);
}

/// Renames `tmp_path` onto `output` on successful flush, removing the
/// `.tmp` if the rename itself fails.
fn commit_tmp(tmp_path: &Path, output: &Path) -> Result<()> {
    match std::fs::rename(tmp_path, output) {
        Ok(()) => Ok(()),
        Err(e) => {
            remove_tmp(tmp_path);
            Err(anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
                "Failed to rename temporary output {} to {}",
                tmp_path.display(),
                output.display()
            )))
        }
    }
}

/// Routes a `TypeTransformer` conversion failure through the typed
/// [`GoldDiggerError::Query`] variant so the exit code stays at 4 even
/// when the message text is refactored. Preserves the original error
/// chain so operators still see the underlying "Invalid month …" detail.
fn wrap_conversion_error(row_index: u64, err: anyhow::Error) -> anyhow::Error {
    anyhow::Error::from(GoldDiggerError::Query(format!(
        "row {}: {}",
        row_index + 1,
        err
    )))
}

// ---------------------------------------------------------------------------
// CSV / TSV sinks (share writer plumbing, differ only in delimiter)
// ---------------------------------------------------------------------------

/// CSV/TSV sink implementation.
///
/// Uses the `csv` crate with [`QuoteStyle::Necessary`]. The writer is
/// wrapped in `Option` so [`RowSink::finalize`] can `take()` it out and
/// drop it before the atomic rename — `csv::Writer`'s `Drop` flushes
/// buffered bytes, and we need that to happen before `fs::rename`
/// observes the `.tmp` path.
struct DelimitedSink {
    writer: Option<csv::Writer<BufWriter<File>>>,
    tmp_path: PathBuf,
    output: PathBuf,
    rows_written: u64,
    committed: bool,
}

impl DelimitedSink {
    fn new(output: &Path, force: bool, delimiter: u8) -> Result<Self> {
        let (tmp_path, buf) = open_tmp_for(output, force)?;
        let writer = WriterBuilder::new()
            .delimiter(delimiter)
            .quote_style(QuoteStyle::Necessary)
            .from_writer(buf);
        Ok(Self {
            writer: Some(writer),
            tmp_path,
            output: output.to_path_buf(),
            rows_written: 0,
            committed: false,
        })
    }

    /// Returns a mutable reference to the writer, or an error if it has
    /// already been taken (should only happen post-finalize, which
    /// callers are not allowed to do — the sink API consumes the box in
    /// `finalize`).
    fn writer_mut(&mut self) -> Result<&mut csv::Writer<BufWriter<File>>> {
        self.writer
            .as_mut()
            .context("Internal error: CSV/TSV sink writer already finalized")
    }
}

impl Drop for DelimitedSink {
    fn drop(&mut self) {
        if !self.committed {
            // Drop the writer first so any buffered bytes flush before
            // we unlink the file (flush errors here are irrelevant — we
            // are removing the file anyway).
            self.writer.take();
            remove_tmp(&self.tmp_path);
        }
    }
}

impl RowSink for DelimitedSink {
    fn on_headers(&mut self, columns: &[String]) -> Result<()> {
        self.writer_mut()?
            .write_record(columns)
            .context("Failed to write header row")?;
        Ok(())
    }

    fn on_row(&mut self, row: &mysql::Row) -> Result<()> {
        let mut record: Vec<String> = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            match row.as_ref(i) {
                Some(value) => {
                    let s = crate::TypeTransformer::value_to_string(value)
                        .map_err(|e| wrap_conversion_error(self.rows_written, e))?;
                    record.push(s);
                }
                // Within 0..row.len(), None indicates a SQL NULL.
                None => record.push(String::new()),
            }
        }
        self.writer_mut()?
            .write_record(&record)
            .context("Failed to write data row")?;
        self.rows_written = self.rows_written.saturating_add(1);
        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> Result<WriteOutcome> {
        let mut writer = self
            .writer
            .take()
            .context("Internal error: CSV/TSV sink writer already finalized")?;
        writer.flush().context("Failed to flush CSV/TSV writer")?;
        drop(writer);
        commit_tmp(&self.tmp_path, &self.output)?;
        self.committed = true;
        Ok(WriteOutcome {
            rows_written: self.rows_written,
        })
    }
}

/// Builds a CSV sink writing to `<output>.tmp`, renaming on
/// [`RowSink::finalize`].
pub fn csv_sink(output: &Path, force: bool) -> Result<Box<dyn RowSink>> {
    Ok(Box::new(DelimitedSink::new(output, force, b',')?))
}

/// Builds a TSV sink writing to `<output>.tmp`, renaming on
/// [`RowSink::finalize`].
pub fn tsv_sink(output: &Path, force: bool) -> Result<Box<dyn RowSink>> {
    Ok(Box::new(DelimitedSink::new(output, force, b'\t')?))
}

// ---------------------------------------------------------------------------
// JSON sink
// ---------------------------------------------------------------------------

/// JSON streaming sink producing `{"data":[...]}`.
///
/// Writes the `{"data":[` preamble in [`RowSink::on_headers`] so the
/// empty-result case still emits a valid envelope (`{"data":[]}`). A
/// comma is emitted between rows. `--pretty` inserts a newline between
/// rows and serialises each row object with `to_writer_pretty`. The
/// closing `]}` is written in [`RowSink::finalize`].
struct JsonSink {
    writer: BufWriter<File>,
    tmp_path: PathBuf,
    output: PathBuf,
    pretty: bool,
    rows_written: u64,
    committed: bool,
}

impl JsonSink {
    fn new(output: &Path, force: bool, pretty: bool) -> Result<Self> {
        let (tmp_path, writer) = open_tmp_for(output, force)?;
        Ok(Self {
            writer,
            tmp_path,
            output: output.to_path_buf(),
            pretty,
            rows_written: 0,
            committed: false,
        })
    }
}

impl Drop for JsonSink {
    fn drop(&mut self) {
        if !self.committed {
            remove_tmp(&self.tmp_path);
        }
    }
}

impl RowSink for JsonSink {
    fn on_headers(&mut self, _columns: &[String]) -> Result<()> {
        // JSON object keys come from the row itself (via row.columns_ref())
        // so we only need to emit the envelope preamble here. Storing
        // `columns` is unnecessary — row conversion uses the per-row
        // metadata.
        write!(self.writer, "{{\"data\":[").context("Failed to write JSON preamble")?;
        Ok(())
    }

    fn on_row(&mut self, row: &mysql::Row) -> Result<()> {
        if self.rows_written > 0 {
            write!(self.writer, ",").context("Failed to write JSON row separator")?;
        }
        // Clone the row: TypeTransformer::row_to_json takes it by value
        // to reuse the existing API. The clone is cheap relative to the
        // conversion itself (the row already holds owned Values).
        let map = crate::TypeTransformer::row_to_json(row.clone())
            .map_err(|e| wrap_conversion_error(self.rows_written, e))?;
        if self.pretty {
            serde_json::to_writer_pretty(&mut self.writer, &map)
                .context("Failed to serialise pretty JSON row")?;
        } else {
            serde_json::to_writer(&mut self.writer, &map)
                .context("Failed to serialise JSON row")?;
        }
        self.rows_written = self.rows_written.saturating_add(1);
        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> Result<WriteOutcome> {
        write!(self.writer, "]}}").context("Failed to write JSON terminator")?;
        self.writer.flush().context("Failed to flush JSON writer")?;
        commit_tmp(&self.tmp_path, &self.output)?;
        self.committed = true;
        Ok(WriteOutcome {
            rows_written: self.rows_written,
        })
    }
}

/// Builds a JSON sink writing to `<output>.tmp`, renaming on
/// [`RowSink::finalize`].
pub fn json_sink(output: &Path, force: bool, pretty: bool) -> Result<Box<dyn RowSink>> {
    Ok(Box::new(JsonSink::new(output, force, pretty)?))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// Tests build synthetic `mysql::Row` values through `mysql_common::row::new_row`,
// which is only linked when the `additional_mysql_types` feature is active.
// That feature is on by default, so this gating only matters for minimal
// `--no-default-features` builds.
#[cfg(all(test, feature = "additional_mysql_types"))]
mod tests {
    use super::*;
    use mysql::Row;
    use mysql::consts::ColumnType;
    use mysql::{Column, Value};
    use mysql_common::row::new_row;
    use std::sync::Arc;
    use tempfile::tempdir;

    /// Helper: builds a single synthetic row with the given column names
    /// and byte-string values. The lengths of `columns` and `values` must
    /// match.
    fn build_row(columns: &[&str], values: Vec<Value>) -> Row {
        let cols: Vec<Column> = columns
            .iter()
            .map(|name| Column::new(ColumnType::MYSQL_TYPE_VAR_STRING).with_name(name.as_bytes()))
            .collect();
        let arc: Arc<[Column]> = cols.into_boxed_slice().into();
        new_row(values, arc)
    }

    #[test]
    fn csv_sink_writes_header_and_rows() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");

        let mut sink = csv_sink(&out, false).expect("sink");
        sink.on_headers(&["a".into(), "b".into()]).expect("headers");
        let row = build_row(
            &["a", "b"],
            vec![Value::Bytes(b"1".to_vec()), Value::Bytes(b"hello".to_vec())],
        );
        sink.on_row(&row).expect("row");
        let outcome = sink.finalize().expect("finalize");
        assert_eq!(outcome.rows_written, 1);

        let body = std::fs::read_to_string(&out).expect("read");
        assert!(body.contains("a,b"));
        assert!(body.contains("1,hello"));

        // No leftover .tmp file.
        let tmp = temp_path_for(&out);
        assert!(!tmp.exists(), "tmp file should be gone after rename");
    }

    #[test]
    fn tsv_sink_uses_tab_delimiter() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.tsv");

        let mut sink = tsv_sink(&out, false).expect("sink");
        sink.on_headers(&["a".into(), "b".into()]).expect("headers");
        let row = build_row(
            &["a", "b"],
            vec![Value::Bytes(b"1".to_vec()), Value::Bytes(b"x".to_vec())],
        );
        sink.on_row(&row).expect("row");
        let outcome = sink.finalize().expect("finalize");
        assert_eq!(outcome.rows_written, 1);

        let body = std::fs::read_to_string(&out).expect("read");
        assert!(body.contains("a\tb"));
        assert!(body.contains("1\tx"));
    }

    #[test]
    fn json_sink_empty_result_emits_envelope() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.json");

        let mut sink = json_sink(&out, false, false).expect("sink");
        sink.on_headers(&["a".into()]).expect("headers");
        let outcome = sink.finalize().expect("finalize");
        assert_eq!(outcome.rows_written, 0);

        let body = std::fs::read_to_string(&out).expect("read");
        assert_eq!(body, r#"{"data":[]}"#);
    }

    #[test]
    fn json_sink_streams_rows() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.json");

        let mut sink = json_sink(&out, false, false).expect("sink");
        sink.on_headers(&["a".into()]).expect("headers");
        let row1 = build_row(&["a"], vec![Value::Bytes(b"first".to_vec())]);
        let row2 = build_row(&["a"], vec![Value::Bytes(b"second".to_vec())]);
        sink.on_row(&row1).expect("row1");
        sink.on_row(&row2).expect("row2");
        let outcome = sink.finalize().expect("finalize");
        assert_eq!(outcome.rows_written, 2);

        let body = std::fs::read_to_string(&out).expect("read");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("valid json");
        let arr = parsed["data"].as_array().expect("data is array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["a"], "first");
        assert_eq!(arr[1]["a"], "second");
    }

    #[test]
    fn sink_failure_cleans_up_tmp_and_leaves_no_target() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");

        let mut sink = csv_sink(&out, false).expect("sink");
        sink.on_headers(&["a".into()]).expect("headers");

        // Invalid date triggers a Type conversion error at on_row time.
        let row = build_row(&["a"], vec![Value::Date(2023, 13, 1, 0, 0, 0, 0)]);
        let err = sink.on_row(&row).expect_err("invalid month must fail");
        assert!(err.to_string().contains("row 1"), "err={}", err);

        // Drop the sink without finalize(); the tmp path must be gone
        // and the target must NOT have been created.
        drop(sink);
        let tmp = temp_path_for(&out);
        assert!(!tmp.exists(), "tmp file should be removed after drop");
        assert!(!out.exists(), "target must not be created on failure");
    }

    #[test]
    fn sink_refuses_existing_target_without_force() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");
        std::fs::write(&out, "preexisting").expect("seed");

        let result = csv_sink(&out, false);
        let err = match result {
            Ok(_) => panic!("must refuse existing file"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("already exists"), "err={}", msg);
        assert!(msg.contains("--force"), "err={}", msg);
    }

    #[test]
    fn sink_force_overwrites_existing_target() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");
        std::fs::write(&out, "preexisting").expect("seed");

        let mut sink = csv_sink(&out, true).expect("sink");
        sink.on_headers(&["a".into()]).expect("headers");
        let row = build_row(&["a"], vec![Value::Bytes(b"new".to_vec())]);
        sink.on_row(&row).expect("row");
        sink.finalize().expect("finalize");

        let body = std::fs::read_to_string(&out).expect("read");
        assert!(body.contains("new"), "body={}", body);
        assert!(!body.contains("preexisting"), "old content leaked");
    }

    #[test]
    fn conversion_error_routes_to_query_exit_code() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");
        let mut sink = csv_sink(&out, false).expect("sink");
        sink.on_headers(&["a".into()]).expect("headers");
        let row = build_row(&["a"], vec![Value::Date(2023, 0, 1, 0, 0, 0, 0)]);
        let err = sink.on_row(&row).expect_err("invalid date");
        assert_eq!(
            crate::exit::map_error_to_exit_code(&err),
            crate::exit::EXIT_QUERY_ERROR
        );
    }
}
