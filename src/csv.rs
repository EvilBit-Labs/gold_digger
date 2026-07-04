//! RFC-4180-adjacent CSV writer.
//!
//! Wraps the `csv` crate with [`QuoteStyle::Necessary`] — quotes are emitted
//! only when fields contain delimiters, newlines, or embedded quotes. The
//! writer is generic over any [`Write`] target so output can be streamed to
//! a file, stdout, or an in-memory buffer.
//!
//! Both this module and [`crate::tab`] delegate to the shared
//! [`crate::delimited::write_delimited`] helper; the only difference is
//! the field delimiter (`b','` here, `b'\t'` in TSV). See todo #058.

use std::io::Write;

use csv::QuoteStyle;

/// Writes rows to a CSV output using the provided writer.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator over fields.
/// * `output` - A writer to output the CSV data.
///
/// # Returns
///
/// A Result indicating success or failure.
pub fn write<R, F, W>(rows: R, output: W) -> anyhow::Result<()>
where
    R: IntoIterator<Item = F>,
    F: IntoIterator<Item = String>,
    W: Write,
{
    crate::delimited::write_delimited(rows, output, b',', QuoteStyle::Necessary)
}
