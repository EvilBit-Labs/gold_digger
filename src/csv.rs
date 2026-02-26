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

/// Writes rows to a CSV output using the provided writer with generic field types.
///
/// This version accepts any type that can be converted to bytes, providing
/// better performance by avoiding unnecessary string allocations.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator over fields.
/// * `output` - A writer to output the CSV data.
///
/// # Returns
///
/// A Result indicating success or failure.
///
/// # Performance
///
/// This function is more efficient than the string-based version when working
/// with data that's already in byte format, as it avoids UTF-8 validation overhead.
pub fn write_bytes<R, F, T, W>(rows: R, output: W) -> anyhow::Result<()>
where
    R: IntoIterator<Item = F>,
    F: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
    W: Write,
{
    let mut wtr = WriterBuilder::new()
        .quote_style(QuoteStyle::Necessary)
        .from_writer(output);

    for row in rows {
        wtr.write_record(row)?;
    }

    Ok(())
}

/// Writes rows to a CSV output using the provided writer with streaming support.
///
/// This version processes records one at a time, minimizing memory usage
/// for large datasets.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator over fields.
/// * `output` - A writer to output the CSV data.
///
/// # Returns
///
/// A Result indicating success or failure.
pub fn write_streaming<R, F, T, W>(rows: R, output: W) -> anyhow::Result<()>
where
    R: Iterator<Item = F>,
    F: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
    W: Write,
{
    let mut wtr = WriterBuilder::new()
        .quote_style(QuoteStyle::Necessary)
        .from_writer(output);

    for row in rows {
        wtr.write_record(row)?;
    }

    Ok(())
}
