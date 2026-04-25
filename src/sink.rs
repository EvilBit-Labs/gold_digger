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
//! always write to a sibling `<output>.tmp` file and atomically commit on
//! success. The atomic commit uses per-platform fail-on-clobber primitives
//! (CRITICAL #2 — TOCTOU rename race): `renameat2(RENAME_NOREPLACE)` on
//! Linux, `hard_link` + unlink on macOS / non-Linux Unix, and `create_new`
//! + copy on Windows. See [`commit_tmp`] for the full rationale.
//!
//! On error from `on_row` (or any other pre-finalize step), the sink's
//! `Drop` deletes the `.tmp` so the filesystem has either the complete
//! output at `<output>` or no file at all. **Exception** (HIGH #13): if
//! the atomic commit itself fails (e.g. cross-device rename, target
//! directory read-only), the `.tmp` is preserved for user recovery and
//! the error message names the `.tmp` path.
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

use crate::OUTPUT_BUFFER_CAPACITY;
use crate::exit::{ConfigError, GoldDiggerError};
use crate::output::create_output_file;

/// 64 KiB write buffer — same size the non-streaming writers used so per-row
/// write overhead stays comparable under the new pipeline. Re-exported from
/// [`crate::OUTPUT_BUFFER_CAPACITY`] (todo #059) so the sink path tracks the
/// canonical knob.
const WRITE_BUFFER_BYTES: usize = OUTPUT_BUFFER_CAPACITY;

/// Outcome returned by a completed [`RowSink::finalize`] call.
///
/// `rows_written` excludes the header row so callers can compare it
/// against a "no rows" threshold without special-casing CSV/TSV.
pub struct WriteOutcome {
    /// Number of data rows written (excludes the CSV/TSV header row).
    pub rows_written: u64,
}

/// One-bit state guard ensuring `on_headers` runs before any `on_row`
/// (HIGH #11 / Type-design #4).
///
/// JSON sinks write a `{"data":[` preamble in `on_headers`; if a future
/// contributor accidentally invoked `on_row` first, the sink would emit
/// a row before the preamble and the resulting `.tmp` would not be valid
/// JSON. CSV/TSV sinks would emit an unlabelled record. Rather than
/// document the protocol and hope, we enforce it at runtime via this
/// state field on every concrete sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinkState {
    /// Initial state: `on_row` is rejected until `on_headers` has been
    /// invoked.
    NeedsHeaders,
    /// `on_headers` has run; rows may flow.
    InRows,
}

/// Streaming consumer of MySQL rows.
///
/// Each sink owns its destination file handle (buffered, writing to a
/// sibling `.tmp` path) and commits the result via an atomic rename in
/// [`Self::finalize`]. Conversion errors propagate out of [`Self::on_row`]
/// without touching the target path — the caller's `Drop` impl on the
/// concrete sink cleans the `.tmp` up.
///
/// Protocol: `on_headers` MUST be called exactly once before any
/// `on_row`. The state guard inside each concrete sink rejects out-of-order
/// calls with [`GoldDiggerError::Query`] so the failure is a typed error
/// rather than silently-malformed output.
pub trait RowSink {
    /// Called exactly once before any rows. `columns` holds the column
    /// names in query order. CSV/TSV sinks write this as the header line;
    /// JSON emits the opening `{"data":[` marker and remembers the names
    /// for the per-row object keys.
    #[must_use = "header-write errors must be propagated; ignoring them leaves a partial .tmp file"]
    fn on_headers(&mut self, columns: &[String]) -> Result<()>;

    /// Called once per row in query order. The sink is responsible for
    /// converting the row via `TypeTransformer` and writing it to its
    /// buffered `.tmp` file.
    #[must_use = "row-write errors must be propagated; ignoring them leaves a partial .tmp file"]
    fn on_row(&mut self, row: &mysql::Row) -> Result<()>;

    /// Called exactly once after all rows (or immediately after
    /// [`Self::on_headers`] if the stream was empty). Flushes the
    /// buffered writer, closes the file, and renames the `.tmp` onto the
    /// target path. Returns the number of data rows written.
    #[must_use = "finalize errors signal incomplete output; the .tmp will linger if ignored"]
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
/// behaviour documented on [`create_output_file`]. The pre-check is NOT
/// the security guarantee — that is enforced atomically inside
/// [`commit_tmp`] via per-platform fail-on-clobber rename primitives. The
/// pre-check is here so the user gets a clean error before any I/O work
/// happens, instead of after writing megabytes of data to `.tmp`.
///
/// HIGH #9: the `.tmp` file is *also* opened with no-clobber semantics
/// (`force = false` → `O_EXCL | O_CREAT`). A predictable sibling at
/// `<output>.tmp` planted by a co-tenant cannot be silently truncated;
/// instead the user gets [`ConfigError::StaleTempFile`] and is told to
/// remove it manually. This is symmetric with the no-clobber default for
/// the target path.
fn open_tmp_for(output: &Path, force: bool) -> Result<(PathBuf, BufWriter<File>)> {
    // Pre-flight: if the target exists and `--force` is not set, refuse
    // now — otherwise a successful `.tmp` write followed by `fs::rename`
    // would clobber the existing file. The atomic enforcement happens
    // inside `commit_tmp` (per-platform fail-on-clobber); this pre-flight
    // is a friendliness optimisation so we don't waste work writing
    // megabytes to `.tmp` before discovering the target is taken.
    if !force && output.exists() {
        return Err(GoldDiggerError::Config(ConfigError::OutputExists {
            path: output.to_path_buf(),
        })
        .into());
    }

    let tmp_path = temp_path_for(output);
    // Always refuse to truncate a pre-existing `.tmp` (HIGH #9): a co-tenant
    // can plant a file at the predictable `<output>.tmp` path and have its
    // contents silently destroyed every run. We translate the
    // `OutputExists` signal from `create_output_file` into the more
    // specific `StaleTempFile` variant so the operator sees actionable
    // guidance ("remove it manually before retrying") rather than the
    // less-applicable `--force` hint.
    let file = match create_output_file(&tmp_path, false) {
        Ok(f) => f,
        Err(e) => {
            // Inspect the error chain for the typed `OutputExists` so we
            // can re-route to `StaleTempFile`. Any other error (genuine
            // filesystem failure, symlink-at-tmp from `O_NOFOLLOW`, etc.)
            // passes through unchanged.
            let is_exists = e.chain().any(|cause| {
                matches!(
                    cause.downcast_ref::<GoldDiggerError>(),
                    Some(GoldDiggerError::Config(ConfigError::OutputExists { .. }))
                )
            });
            if is_exists {
                return Err(
                    GoldDiggerError::Config(ConfigError::StaleTempFile { path: tmp_path }).into(),
                );
            }
            return Err(e);
        }
    };
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

/// Atomically commits `tmp_path` onto `output`, with fail-on-clobber
/// semantics by default and overwrite semantics under `force = true`.
///
/// CRITICAL #2 (TOCTOU rename race): plain `std::fs::rename` unconditionally
/// clobbers any file at the destination; the pre-flight existence check in
/// [`open_tmp_for`] is racy by construction. An attacker who plants a file
/// at the target path between the pre-check and the rename wins the race
/// and gets their content silently overwritten by query results
/// (information disclosure). The atomic guarantee comes from the rename
/// primitive itself, not the pre-check.
///
/// Per-platform strategy when `force = false`:
///
/// * **Linux:** `renameat2(2)` with `RENAME_NOREPLACE` — single syscall,
///   atomic, returns `EEXIST` on collision. Available since Linux 3.15
///   (≈2014); gold_digger's MSRV implies a kernel newer than that on any
///   supported deployment.
/// * **macOS / non-Linux Unix:** no `RENAME_NOREPLACE` equivalent in the
///   stable libc surface, so we use `std::fs::hard_link` (returns
///   `EEXIST` atomically on collision) followed by `std::fs::remove_file`
///   on the original `.tmp`. The `hard_link` step is the atomic
///   commit point — if it fails, nothing has changed and the `.tmp` still
///   holds the data. If `hard_link` succeeds but `remove_file` fails the
///   user has the data at the target plus a leftover `.tmp`; we surface a
///   warning rather than rolling back, since the primary goal (data at
///   target) is met.
/// * **Windows:** no POSIX `link(2)` equivalent on the stable surface that
///   works across volumes consistently. We pre-acquire exclusive
///   ownership of the target via `OpenOptions::create_new(true)`, then
///   `fs::copy` from `.tmp` to the (already-owned) target, then
///   `fs::remove_file` on the `.tmp`. The `create_new` step is the atomic
///   commit point — collision returns `AlreadyExists`. (We accept the
///   double-write cost on Windows as a tradeoff for correctness.)
///
/// When `force = true`, all platforms fall back to plain `std::fs::rename`
/// since clobbering is the documented intent.
///
/// On rename failure (HIGH #13), we DO NOT delete `.tmp` — the user is told
/// to recover from `.tmp` manually. The caller's `committed_or_aborted`
/// flag prevents the sink's `Drop` impl from deleting the `.tmp` on this
/// path either.
fn commit_tmp(tmp_path: &Path, output: &Path, force: bool) -> Result<()> {
    if force {
        // Documented intent: overwrite. Plain rename is correct and
        // atomic in the presence of a collision.
        return atomic_force_rename(tmp_path, output);
    }

    #[cfg(target_os = "linux")]
    {
        commit_tmp_linux(tmp_path, output)
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    {
        commit_tmp_unix_via_hardlink(tmp_path, output)
    }

    #[cfg(windows)]
    {
        commit_tmp_windows(tmp_path, output)
    }
}

/// `force = true` rename. Plain `fs::rename` IS atomic with respect to a
/// concurrent reader observing one or the other version; it just doesn't
/// give us a fail-on-clobber signal. With `--force` the caller has opted
/// into clobbering, so that's the desired behaviour.
fn atomic_force_rename(tmp_path: &Path, output: &Path) -> Result<()> {
    std::fs::rename(tmp_path, output).map_err(|e| rename_failure(e, tmp_path, output))
}

/// Linux fast path: `renameat2(RENAME_NOREPLACE)` — single syscall, atomic,
/// returns `EEXIST` on collision.
#[cfg(target_os = "linux")]
fn commit_tmp_linux(tmp_path: &Path, output: &Path) -> Result<()> {
    use nix::fcntl::{AT_FDCWD, RenameFlags, renameat2};

    // `AT_FDCWD` for both directory fds means the paths are interpreted
    // relative to CWD (same as plain `rename(2)`); we always pass
    // absolute or CWD-relative paths from the call sites in this module.
    match renameat2(
        AT_FDCWD,
        tmp_path,
        AT_FDCWD,
        output,
        RenameFlags::RENAME_NOREPLACE,
    ) {
        Ok(()) => Ok(()),
        Err(errno) => {
            // `nix::errno::Errno` implements `From` for `std::io::Error`,
            // so we don't need an `as i32` cast (which would trip the
            // workspace's `as_conversions` clippy lint).
            let io_err: std::io::Error = errno.into();
            Err(rename_failure(io_err, tmp_path, output))
        }
    }
}

/// macOS / non-Linux Unix path: `hard_link` is atomic and returns `EEXIST`
/// on collision, then unlink the source `.tmp`. The `hard_link` is the
/// commit point.
#[cfg(all(unix, not(target_os = "linux")))]
fn commit_tmp_unix_via_hardlink(tmp_path: &Path, output: &Path) -> Result<()> {
    std::fs::hard_link(tmp_path, output).map_err(|e| rename_failure(e, tmp_path, output))?;

    // The link succeeded — the data is now visible at `output`. Unlink
    // the original `.tmp`. If unlink fails (highly unusual: same dir,
    // we just opened it for write), surface a non-fatal warning but do
    // NOT roll back the link — the user has the data at the target,
    // which is the outcome they asked for.
    if let Err(e) = std::fs::remove_file(tmp_path) {
        tracing::warn!(
            "Output committed to {} but failed to remove temporary file {}: {}",
            output.display(),
            tmp_path.display(),
            e
        );
    }
    Ok(())
}

/// Windows path: pre-acquire exclusive ownership of the target via
/// `create_new`, then copy the `.tmp` into it. The `create_new` is the
/// atomic commit point — `AlreadyExists` indicates the race lost.
#[cfg(windows)]
fn commit_tmp_windows(tmp_path: &Path, output: &Path) -> Result<()> {
    // Stake an exclusive claim on the target. If something else is at
    // the path we get `AlreadyExists` here, atomically.
    let target = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
    {
        Ok(f) => f,
        Err(e) => return Err(rename_failure(e, tmp_path, output)),
    };
    // Drop the handle before copying so `fs::copy` can re-open the file
    // for write without sharing-mode conflicts.
    drop(target);

    if let Err(e) = std::fs::copy(tmp_path, output) {
        // Best-effort: try to undo the empty target we just created so
        // the user isn't left with a 0-byte file.
        let _ = std::fs::remove_file(output);
        return Err(rename_failure(e, tmp_path, output));
    }
    if let Err(e) = std::fs::remove_file(tmp_path) {
        tracing::warn!(
            "Output committed to {} but failed to remove temporary file {}: {}",
            output.display(),
            tmp_path.display(),
            e
        );
    }
    Ok(())
}

/// HIGH #13: format a rename/commit failure that PRESERVES the `.tmp`
/// path so the user can recover their data manually. The caller MUST set
/// `committed_or_aborted = true` so the sink's `Drop` does not delete
/// the `.tmp` after this error returns.
fn rename_failure(e: std::io::Error, tmp_path: &Path, output: &Path) -> anyhow::Error {
    anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
        "Failed to commit output to {}. Your data is preserved at {}; \
         move it manually if the target path becomes writable.",
        output.display(),
        tmp_path.display()
    ))
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

/// HIGH #11 / Type-design #4: enforce the `on_headers → on_row` ordering
/// invariant at runtime. Returns a typed `Query` error (exit 4) on
/// violation so out-of-order callers fail fast with a stable exit code,
/// instead of producing silently-malformed output (e.g. a JSON row written
/// before the `{"data":[` preamble).
fn ensure_row_state_or_err(state: SinkState) -> Result<()> {
    match state {
        SinkState::InRows => Ok(()),
        SinkState::NeedsHeaders => Err(anyhow::Error::from(GoldDiggerError::Query(
            "on_row called before on_headers — sink protocol violation".to_string(),
        ))),
    }
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
    /// `--force` policy in effect for this sink. Threaded through to
    /// [`commit_tmp`] so the per-platform fail-on-clobber path is
    /// chosen when the user did NOT pass `--force`, and a plain
    /// overwrite-rename is chosen when they did.
    force: bool,
    /// True iff `commit_tmp` succeeded and the `.tmp` was consumed by
    /// the rename / hardlink / copy step. When this is set, `Drop` does
    /// no work (the `.tmp` is already gone).
    committed: bool,
    /// True iff `finalize` was entered (success OR failure). HIGH #13:
    /// when `commit_tmp` fails we leave the `.tmp` on disk for the user
    /// to recover; the `Drop` impl must not undo that by deleting it.
    /// Distinguishes "sink dropped without finalize" (Drop deletes
    /// `.tmp`) from "finalize failed" (Drop preserves `.tmp`).
    committed_or_aborted: bool,
    /// HIGH #11 / Type-design #4: enforces `on_headers → on_row` order.
    state: SinkState,
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
            force,
            committed: false,
            committed_or_aborted: false,
            state: SinkState::NeedsHeaders,
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
        // HIGH #13: only delete the `.tmp` when finalize was NOT entered.
        // - committed = true   → rename consumed the .tmp; nothing to do.
        // - committed_or_aborted = true (but not committed) → finalize
        //   failed and we explicitly preserved .tmp for user recovery;
        //   Drop must not undo that.
        // - both false → sink dropped before finalize (panic / error path
        //   before commit); Drop SHOULD delete the partial .tmp.
        if !self.committed_or_aborted {
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
        self.state = SinkState::InRows;
        Ok(())
    }

    fn on_row(&mut self, row: &mysql::Row) -> Result<()> {
        ensure_row_state_or_err(self.state)?;
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
        // HIGH #13: mark "finalize entered" before the commit attempt so
        // `Drop` preserves the `.tmp` if the commit fails. Without this,
        // a rename failure would return the typed error AND silently
        // delete the only complete copy of the data.
        self.committed_or_aborted = true;
        commit_tmp(&self.tmp_path, &self.output, self.force)?;
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
/// `column_names` is populated once from the `on_headers` hook and
/// shared across every row via `Arc<str>`, so per-row JSON conversion
/// no longer re-allocates the column-name strings (todo #069).
struct JsonSink {
    writer: BufWriter<File>,
    tmp_path: PathBuf,
    output: PathBuf,
    pretty: bool,
    rows_written: u64,
    /// `--force` policy in effect; threaded through to [`commit_tmp`].
    force: bool,
    /// True iff `commit_tmp` succeeded; see [`DelimitedSink::committed`].
    committed: bool,
    /// True iff `finalize` was entered (success OR failure); see
    /// [`DelimitedSink::committed_or_aborted`]. HIGH #13.
    committed_or_aborted: bool,
    /// HIGH #11 / Type-design #4: enforces `on_headers → on_row` order.
    state: SinkState,
    column_names: Vec<std::sync::Arc<str>>,
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
            force,
            committed: false,
            committed_or_aborted: false,
            state: SinkState::NeedsHeaders,
            column_names: Vec::new(),
        })
    }
}

impl Drop for JsonSink {
    fn drop(&mut self) {
        // HIGH #13: see DelimitedSink::drop for the rationale.
        if !self.committed_or_aborted {
            remove_tmp(&self.tmp_path);
        }
    }
}

impl RowSink for JsonSink {
    fn on_headers(&mut self, columns: &[String]) -> Result<()> {
        // Extract column names once into shared `Arc<str>`s so per-row
        // JSON conversion does not re-allocate them (todo #069). The
        // BTreeMap keys produced by `row_to_json_with_columns` are
        // still owned `String`s — the win is that we no longer call
        // `name_str().to_string()` on the row's metadata per row.
        self.column_names = columns
            .iter()
            .map(|name| std::sync::Arc::<str>::from(name.as_str()))
            .collect();
        write!(self.writer, "{{\"data\":[").context("Failed to write JSON preamble")?;
        self.state = SinkState::InRows;
        Ok(())
    }

    fn on_row(&mut self, row: &mysql::Row) -> Result<()> {
        ensure_row_state_or_err(self.state)?;
        if self.rows_written > 0 {
            write!(self.writer, ",").context("Failed to write JSON row separator")?;
        }
        // Use the shared column-name list captured in `on_headers` so
        // we avoid the per-row `name_str().to_string()` cost the legacy
        // `row_to_json` path incurs (todo #069). The new helper takes
        // the row by reference, so no `row.clone()` is needed either.
        let map = crate::TypeTransformer::row_to_json_with_columns(row, &self.column_names)
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
        // HIGH #13: see DelimitedSink::finalize.
        self.committed_or_aborted = true;
        commit_tmp(&self.tmp_path, &self.output, self.force)?;
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

    /// HIGH #11 / Type-design #4: invoking `on_row` before `on_headers`
    /// must fail with a typed `Query` error (exit 4) rather than emit a
    /// silently-malformed `.tmp` (e.g. a JSON row before the `{"data":[`
    /// preamble). This is the runtime side of the protocol the
    /// `SinkState` field encodes.
    #[test]
    fn json_sink_rejects_on_row_before_on_headers() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.json");
        let mut sink = json_sink(&out, false, false).expect("sink");
        let row = build_row(&["a"], vec![Value::Bytes(b"x".to_vec())]);
        let err = sink.on_row(&row).expect_err("must reject pre-header row");
        let msg = err.to_string();
        assert!(
            msg.contains("on_row called before on_headers"),
            "err msg should name the violation, got: {}",
            msg
        );
        assert_eq!(
            crate::exit::map_error_to_exit_code(&err),
            crate::exit::EXIT_QUERY_ERROR,
            "protocol-violation must route to query-class exit (4)"
        );
    }

    /// Same protocol guard for the CSV/TSV path.
    #[test]
    fn csv_sink_rejects_on_row_before_on_headers() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");
        let mut sink = csv_sink(&out, false).expect("sink");
        let row = build_row(&["a"], vec![Value::Bytes(b"x".to_vec())]);
        let err = sink.on_row(&row).expect_err("must reject pre-header row");
        assert!(
            err.to_string().contains("on_row called before on_headers"),
            "err={}",
            err
        );
    }

    /// HIGH #9: a co-tenant cannot silently destroy a planted
    /// `<output>.tmp` — the sink refuses to truncate it and surfaces a
    /// typed `StaleTempFile` error directing the user to remove it
    /// manually. Asserts (a) the error message names the path, (b) the
    /// pre-existing `.tmp` content is still on disk after the failure
    /// (no truncation happened).
    #[test]
    fn sink_refuses_to_truncate_pre_existing_tmp() {
        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");
        let tmp = temp_path_for(&out);
        std::fs::write(&tmp, "co-tenant content").expect("seed tmp");

        let result = csv_sink(&out, false);
        let err = result.err().expect("must refuse stale tmp");
        let msg = err.to_string();
        assert!(
            msg.contains("Stale temporary output file"),
            "err msg should mention StaleTempFile, got: {}",
            msg
        );
        assert!(
            msg.contains(&tmp.display().to_string()),
            "err msg should include the .tmp path, got: {}",
            msg
        );

        // The planted content must still be on disk — the sink MUST NOT
        // have truncated it.
        let still_there = std::fs::read_to_string(&tmp).expect("tmp still readable");
        assert_eq!(
            still_there, "co-tenant content",
            "co-tenant content must not be truncated"
        );

        // And the typed routing must be config-class (exit 2).
        assert_eq!(
            crate::exit::map_error_to_exit_code(&err),
            crate::exit::EXIT_CONFIG_ERROR,
        );
    }

    /// HIGH #13: when `commit_tmp` fails, the sink MUST preserve the
    /// `.tmp` file so the user can recover their data. The error message
    /// MUST include the `.tmp` path verbatim. We simulate the rename
    /// failure by removing write permission from the parent directory
    /// after the `.tmp` is fully written but before `finalize` runs.
    ///
    /// On Unix this works because the rename target is a *new entry* in
    /// the parent directory, which requires write permission on the
    /// parent. The existing `.tmp` file's data is untouched.
    #[cfg(unix)]
    #[test]
    fn finalize_failure_preserves_tmp_with_recovery_message() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let out = dir.path().join("out.csv");

        let mut sink = csv_sink(&out, false).expect("sink");
        sink.on_headers(&["a".into()]).expect("headers");
        let row = build_row(&["a"], vec![Value::Bytes(b"recovery-data".to_vec())]);
        sink.on_row(&row).expect("row");

        // Snapshot the .tmp path before finalize.
        let tmp = temp_path_for(&out);

        // Strip write/exec on the parent directory so the
        // hardlink/rename creating a NEW entry in the directory fails
        // with EACCES. The existing .tmp inode is untouched: data is
        // still readable inode-wise once we restore perms, and as far
        // as the kernel is concerned the file's contents are preserved.
        let dir_path = dir.path().to_path_buf();
        let original_perms = std::fs::metadata(&dir_path).expect("perms").permissions();
        std::fs::set_permissions(&dir_path, std::fs::Permissions::from_mode(0o500))
            .expect("strip write perm");

        let finalize_result = sink.finalize();

        // Restore perms BEFORE asserting so we can read the .tmp file
        // and so the tempdir cleanup works regardless of test outcome.
        std::fs::set_permissions(&dir_path, original_perms).expect("restore perms");

        let err = finalize_result
            .err()
            .expect("commit must fail under EACCES");

        // The error MUST mention the .tmp path so the user can recover.
        let chained = err
            .chain()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(": ");
        assert!(
            chained.contains(&tmp.display().to_string()),
            "error chain must reference the .tmp path for recovery: {}",
            chained
        );
        assert!(
            chained.contains("preserved"),
            "error chain should explain that data is preserved: {}",
            chained
        );
        // Top-level message also mentions the target so the operator
        // can correlate.
        assert!(
            chained.contains(&out.display().to_string()),
            "error chain must reference the target path: {}",
            chained
        );

        // Critical assertion: the .tmp must still exist on disk.
        assert!(
            tmp.exists(),
            "HIGH #13: .tmp must survive a finalize failure for user recovery"
        );

        // And the target must NOT have been created.
        assert!(
            !out.exists(),
            "target must not appear when commit fails (no half-written state)"
        );

        // The .tmp must contain the data we wrote (not be truncated).
        let preserved = std::fs::read_to_string(&tmp).expect("read preserved tmp");
        assert!(
            preserved.contains("recovery-data"),
            ".tmp content must be intact: {}",
            preserved
        );
    }
}
