//! Shared delimited-output helper for CSV and TSV writers.
//!
//! [`crate::csv::write`] and [`crate::tab::write`] differ only by their
//! field delimiter (`,` vs `\t`). Extracting a single helper here keeps
//! buffer-size, quote-style, and flush behaviour in lock-step (todo #058).
//!
//! # Single-buffer pipeline
//!
//! Per todo #070, the helper does **not** wrap `output` in an outer
//! [`std::io::BufWriter`]. The `csv` crate's [`csv::WriterBuilder`]
//! already maintains an internal buffer; we configure it via
//! [`csv::WriterBuilder::buffer_capacity`] with
//! [`crate::OUTPUT_BUFFER_CAPACITY`] so there is exactly one memcpy per
//! row instead of two. Callers that pass an unbuffered file directly will
//! still get coalesced 64 KiB writes through the csv-internal buffer.

use std::io::Write;

use anyhow::Context;
use csv::{QuoteStyle, WriterBuilder};

use crate::OUTPUT_BUFFER_CAPACITY;

/// Writes rows to a delimited (CSV/TSV) output using the provided writer.
///
/// Used by both [`crate::csv::write`] and [`crate::tab::write`]. The csv
/// crate's internal buffer (sized via
/// [`csv::WriterBuilder::buffer_capacity`]) is the only buffer in the
/// pipeline; passing in an unbuffered `output` is fine.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator
///   over fields.
/// * `output` - A writer to output the delimited data. Does **not** need
///   to be wrapped in [`std::io::BufWriter`] — see module docs.
/// * `delimiter` - Field delimiter byte (`b','` for CSV, `b'\t'` for TSV).
/// * `quote_style` - Quote style passed through to the csv crate. Both
///   call sites use [`QuoteStyle::Necessary`] today.
///
/// # Returns
///
/// `Ok(())` on success; an error if writing or flushing fails.
pub(crate) fn write_delimited<R, F, W>(
    rows: R,
    output: W,
    delimiter: u8,
    quote_style: QuoteStyle,
) -> anyhow::Result<()>
where
    R: IntoIterator<Item = F>,
    F: IntoIterator<Item = String>,
    W: Write,
{
    let mut wtr = WriterBuilder::new()
        .delimiter(delimiter)
        .quote_style(quote_style)
        .buffer_capacity(OUTPUT_BUFFER_CAPACITY)
        .from_writer(output);

    // Track 1-based row index so write failures point operators at the
    // offending record. The streaming `RowSink` path in `src/sink.rs`
    // already does this; we mirror the breadcrumb here for the
    // non-streaming `csv::write` / `tab::write` callers.
    let mut row_index: u64 = 0;
    for row in rows {
        row_index = row_index.saturating_add(1);
        wtr.write_record(row)
            .with_context(|| format!("Failed to write delimited row {}", row_index))?;
    }

    wtr.flush().with_context(|| {
        format!(
            "Failed to flush delimited writer after {} row(s)",
            row_index
        )
    })?;
    Ok(())
}
