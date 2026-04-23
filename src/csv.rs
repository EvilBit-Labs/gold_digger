//! RFC-4180-adjacent CSV writer.
//!
//! Wraps the `csv` crate with [`QuoteStyle::Necessary`] — quotes are emitted
//! only when fields contain delimiters, newlines, or embedded quotes. The
//! writer is generic over any [`Write`] target so output can be streamed to
//! a file, stdout, or an in-memory buffer.

use std::io::{BufWriter, Write};

use csv::{QuoteStyle, WriterBuilder};

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
    let buffered_output = BufWriter::with_capacity(64 * 1024, output); // 64KB buffer for better performance with large datasets
    let mut wtr = WriterBuilder::new()
        .quote_style(QuoteStyle::Necessary)
        .from_writer(buffered_output);

    for row in rows {
        wtr.write_record(row)?;
    }

    wtr.flush()?; // Ensure all data is written
    Ok(())
}
