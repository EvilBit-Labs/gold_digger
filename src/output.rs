//! Output format dispatch and safe output-file creation.
//!
//! For JSON output, uses [`crate::TypeTransformer`] to preserve native MySQL
//! types (integers as JSON numbers, NULLs as JSON null, etc.). For CSV and
//! TSV, converts rows to strings first via [`crate::rows_to_strings`],
//! ensuring conversion succeeds before creating/truncating the output file.
//!
//! Path-safety (todo #024): the output file is created through
//! [`create_output_file`], which enforces `O_NOFOLLOW` + `0o600` + no-clobber
//! defaults on Unix. Passing `--force` opts into overwriting an existing file
//! but still refuses to follow symlinks.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::{Cli, OutputFormat};
use crate::exit::GoldDiggerError;
use crate::rows_to_strings;

/// Writes output in the specified format.
///
/// For JSON output, uses `TypeTransformer` to preserve native MySQL types (integers
/// as JSON numbers, NULLs as JSON null, etc.). For CSV and TSV, converts rows to
/// strings first via `rows_to_strings`, ensuring conversion succeeds before
/// creating/truncating the output file.
///
/// Format selection (todo #019): if `--format` is absent AND the output file
/// extension is unknown (or missing), returns [`GoldDiggerError::Config`]
/// instead of silently defaulting to TSV. The previous behaviour surfaced a
/// "silent format selection" hazard — an `.xml` or `.yaml` output path would
/// quietly emit tab-separated data with no signal to the caller.
///
/// Path-safety (todo #024): the output file is created through
/// [`create_output_file`], which enforces `O_NOFOLLOW` + `0o600` + no-clobber
/// defaults on Unix. Passing `--force` opts into overwriting an existing file
/// but still refuses to follow symlinks.
pub fn write_output(rows: Vec<mysql::Row>, output_file: &Path, cli: &Cli) -> Result<()> {
    let format = if let Some(format) = &cli.format {
        format.clone()
    } else {
        OutputFormat::from_extension(output_file).ok_or_else(|| {
            GoldDiggerError::Config(format!(
                "Cannot infer output format from '{}'. Recognised extensions: .csv, .json, .tsv, .tab, .txt. Pass --format <csv|json|tsv> to select explicitly.",
                output_file.display()
            ))
        })?
    };

    match format {
        OutputFormat::Csv => {
            let string_rows = rows_to_strings(rows)?;
            let output = create_output_file(output_file, cli.force)?;
            crate::csv::write(string_rows, output)?;
        }
        OutputFormat::Json => {
            use crate::TypeTransformer;

            // Convert rows to JSON maps before creating the file to avoid
            // leaving an empty/truncated file on conversion failure.
            let json_maps: Vec<BTreeMap<String, serde_json::Value>> = rows
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    TypeTransformer::row_to_json(row)
                        .with_context(|| format!("Failed to convert row {}", i + 1))
                })
                .collect::<Result<Vec<_>>>()?;

            let output = create_output_file(output_file, cli.force)?;
            crate::json::write(json_maps, output, cli.pretty)?;
        }
        OutputFormat::Tsv => {
            let string_rows = rows_to_strings(rows)?;
            let output = create_output_file(output_file, cli.force)?;
            crate::tab::write(string_rows, output)?;
        }
    }

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
        return GoldDiggerError::Config(format!(
            "Output file already exists: {}. Pass --force to overwrite.",
            output_file.display()
        ))
        .into();
    }
    anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
        "Failed to create output file {}",
        output_file.display()
    ))
}
