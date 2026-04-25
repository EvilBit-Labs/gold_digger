//! Output format dispatch, sink construction, and safe output-file creation.
//!
//! The live query-execution path no longer materialises the full result
//! set into a `Vec<mysql::Row>`. The [`build_sink`] function selects a
//! [`crate::sink::RowSink`] implementation based on the requested format
//! (`--format` > file extension > error) and `src/run.rs` streams rows
//! into it directly. The legacy [`write_output`] helper, which took a
//! fully-materialised `Vec<mysql::Row>`, is retained for test-only use so
//! snapshot tests can feed in-memory row vectors without going through
//! the streaming pipeline.
//!
//! Path-safety (todo #024): the output file is created through
//! [`create_output_file`], which enforces `O_NOFOLLOW` + `0o600` + no-clobber
//! defaults on Unix. Passing `--force` opts into overwriting an existing file
//! but still refuses to follow symlinks.

use std::fs::{File, OpenOptions};
use std::path::Path;

use anyhow::Result;

use crate::cli::{Cli, OutputFormat};
use crate::exit::{ConfigError, GoldDiggerError};
use crate::sink::{RowSink, csv_sink, json_sink, tsv_sink};

/// Resolves the output format from `--format` or the file extension,
/// returning a typed config error (exit 2) when neither is usable.
///
/// Format selection (todo #019): if `--format` is absent AND the output file
/// extension is unknown (or missing), returns [`GoldDiggerError::Config`]
/// instead of silently defaulting to TSV. The previous behaviour surfaced a
/// "silent format selection" hazard — an `.xml` or `.yaml` output path would
/// quietly emit tab-separated data with no signal to the caller.
pub fn resolve_output_format(output_file: &Path, cli: &Cli) -> Result<OutputFormat> {
    if let Some(format) = cli.format {
        Ok(format)
    } else {
        OutputFormat::from_extension(output_file).ok_or_else(|| {
            GoldDiggerError::Config(ConfigError::UnresolvableFormat(format!(
                "Cannot infer output format from '{}'. Recognised extensions: .csv, .json, .tsv, .tab, .txt. Pass --format <csv|json|tsv> to select explicitly.",
                output_file.display()
            )))
            .into()
        })
    }
}

/// Builds a format-specific [`RowSink`] for streaming query results.
///
/// The returned sink owns a file handle on the sibling `<output>.tmp`
/// path; [`RowSink::finalize`] renames the `.tmp` onto `output` on
/// successful completion, and the sink's `Drop` cleans up on failure.
pub fn build_sink(output_file: &Path, cli: &Cli) -> Result<Box<dyn RowSink>> {
    let format = resolve_output_format(output_file, cli)?;
    build_sink_for_format(output_file, format, cli.force, cli.pretty)
}

/// Builds a format-specific [`RowSink`] for streaming query results
/// using already-resolved format / force / pretty values.
///
/// Preferred over [`build_sink`] on code paths that hold a
/// [`crate::config::ResolvedConfig`] — the format has already been
/// determined at resolution time, so re-deriving it would re-run the
/// extension dispatch and reintroduce the chance of a "silent format"
/// regression.
pub fn build_sink_for_format(
    output_file: &Path,
    format: OutputFormat,
    force: bool,
    pretty: bool,
) -> Result<Box<dyn RowSink>> {
    match format {
        OutputFormat::Csv => csv_sink(output_file, force),
        OutputFormat::Json => json_sink(output_file, force, pretty),
        OutputFormat::Tsv => tsv_sink(output_file, force),
    }
}

/// Writes a fully-materialised `Vec<mysql::Row>` by driving the
/// streaming sink pipeline.
///
/// This function is retained for snapshot tests and the empty-result
/// branch in [`crate::run`]. New code on the live path should use
/// [`build_sink`] with `conn.query_iter` directly to avoid pulling the
/// entire result set into memory.
///
/// Path-safety (todo #024): the output file is created through
/// [`create_output_file`], which enforces `O_NOFOLLOW` + `0o600` + no-clobber
/// defaults on Unix. Passing `--force` opts into overwriting an existing file
/// but still refuses to follow symlinks.
pub fn write_output(rows: Vec<mysql::Row>, output_file: &Path, cli: &Cli) -> Result<()> {
    let mut sink = build_sink(output_file, cli)?;

    // Extract headers from the first row (if any) before iterating data;
    // empty result sets still emit a valid envelope / header row because
    // the sinks treat `on_headers` as always-called.
    let headers: Vec<String> = if let Some(first) = rows.first() {
        first
            .columns_ref()
            .iter()
            .map(|col| col.name_str().to_string())
            .collect()
    } else {
        Vec::new()
    };
    sink.on_headers(&headers)?;

    for row in rows.iter() {
        sink.on_row(row)?;
    }

    sink.finalize()?;
    Ok(())
}

/// Safely creates the output file with path-safety guards (todo #024).
///
/// On Unix:
///   - `O_NOFOLLOW` — refuses to follow a symlink at the target (an
///     attacker-placed symlink at a predictable path like `/tmp/out.json`
///     cannot be used to clobber `/etc/cron.d/x`).
///   - `mode = 0o600` — created files are owner read/write only, not
///     world-readable, since query results often contain sensitive data.
///   - Without `--force`: `create_new(true)` — refuses to overwrite an
///     existing file. An adversary racing to pre-place a file therefore
///     cannot win (the kernel enforces `O_EXCL | O_CREAT` atomically).
///   - With `--force`: the file is truncated and rewritten, but
///     `O_NOFOLLOW` is preserved so a symlink at the target still fails.
///
/// On Windows:
///   - `custom_flags` / `mode` are no-ops on the Windows `OpenOptions`
///     surface; we fall back to `create_new(true)` without `--force` and
///     `create(true) + truncate(true)` with `--force`. Windows does not
///     have POSIX symlinks at the same layer, so the risk profile is
///     different and the baseline behaviour is acceptable.
///
/// Errors map to [`GoldDiggerError::Io`] (exit 5) for filesystem failures
/// and [`GoldDiggerError::Config`] (exit 2) for the "file exists, pass
/// --force to overwrite" case.
pub fn create_output_file(output_file: &Path, force: bool) -> Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let mut opts = OpenOptions::new();
        opts.write(true)
            // O_NOFOLLOW: fail if the final path component is a symlink.
            // Prevents an attacker-placed symlink at a predictable output
            // path from redirecting the write to an unintended target.
            .custom_flags(libc::O_NOFOLLOW)
            .mode(0o600);

        if force {
            // Force-overwrite: truncate existing content, but still refuse
            // to follow a symlink (O_NOFOLLOW above).
            opts.create(true).truncate(true);
        } else {
            // Default: exclusive-create. Fails if target already exists
            // (regular file OR symlink — the kernel rejects before open).
            opts.create_new(true);
        }

        opts.open(output_file)
            .map_err(|e| classify_output_open_error(e, output_file, force))
    }

    #[cfg(not(unix))]
    {
        let mut opts = OpenOptions::new();
        opts.write(true);

        if force {
            opts.create(true).truncate(true);
        } else {
            opts.create_new(true);
        }

        opts.open(output_file)
            .map_err(|e| classify_output_open_error(e, output_file, force))
    }
}

/// Classifies an `io::Error` from opening the output file into the
/// appropriate [`GoldDiggerError`] variant so the exit code is stable.
///
/// `AlreadyExists` (only possible when `create_new(true)` is set, i.e.
/// `--force` was NOT passed) is a user-facing policy error: the operator
/// is being told to pass `--force` to opt in. All other errors
/// (`NotFound` for missing parent directory, `PermissionDenied`, Unix
/// `ELOOP` from `O_NOFOLLOW`) are genuine filesystem failures that route
/// to `EXIT_IO_ERROR` (5).
fn classify_output_open_error(e: std::io::Error, output_file: &Path, force: bool) -> anyhow::Error {
    if e.kind() == std::io::ErrorKind::AlreadyExists && !force {
        return GoldDiggerError::Config(ConfigError::OutputExists {
            path: output_file.to_path_buf(),
        })
        .into();
    }
    anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
        "Failed to create output file {}",
        output_file.display()
    ))
}
