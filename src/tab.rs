//! Tab-separated (TSV) writer mirroring the CSV writer's contract.
//!
//! Wraps the `csv` crate with `\t` as delimiter and [`QuoteStyle::Necessary`].
//! Selected when the output file extension is `.tsv` or `.txt`, or as the
//! default fallback when no recognised extension is present.

use std::io::{BufWriter, Write};

use csv::{QuoteStyle, WriterBuilder};

/// Writes rows to a tab-delimited output using the provided writer.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator over fields.
/// * `output` - A writer to output the tab-delimited data.
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
    let buffered_output = BufWriter::with_capacity(64 * 1024, output);
    let mut wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .quote_style(QuoteStyle::Necessary)
        .from_writer(buffered_output);

    for row in rows {
        wtr.write_record(row)?;
    }

    wtr.flush()?;
    Ok(())
}
