//! Tab-separated (TSV) writer mirroring the CSV writer's contract.
//!
//! Wraps the `csv` crate with `\t` as delimiter and [`QuoteStyle::Necessary`].
//! Selected when the output file extension is `.tsv` or `.txt`, or as the
//! default fallback when no recognised extension is present.
//!
//! Delegates to [`crate::delimited::write_delimited`]; the only difference
//! between this module and [`crate::csv`] is the delimiter byte (todo #058).

use std::io::Write;

use csv::QuoteStyle;

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
    crate::delimited::write_delimited(rows, output, b'\t', QuoteStyle::Necessary)
}
