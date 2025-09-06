//! Output parsing utilities for test support
//!
//! This module provides utilities for parsing and validating Gold Digger output
//! in different formats (CSV, JSON, TSV).

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// Re-export parsing types from integration module
pub use crate::integration::common::{CsvParseResult, JsonParseResult, OutputParser, TsvParseResult};

/// Simplified output parsing utilities
pub struct OutputParsingUtils;

impl OutputParsingUtils {
    /// Parse output file based on file extension
    pub fn parse_output_file(file_path: &Path) -> Result<ParsedOutput> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read output file: {}", file_path.display()))?;

        let extension = file_path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "csv" => {
                let csv_result = OutputParser::parse_csv(&content)?;
                Ok(ParsedOutput::Csv(csv_result))
            },
            "json" => {
                let json_result = OutputParser::parse_json(&content)?;
                Ok(ParsedOutput::Json(json_result))
            },
            "tsv" => {
                let tsv_result = OutputParser::parse_tsv(&content)?;
                Ok(ParsedOutput::Tsv(tsv_result))
            },
            _ => Err(anyhow::anyhow!("Unsupported file extension: {}", extension)),
        }
    }

    /// Validate CSV format compliance
    pub fn validate_csv_format(content: &str) -> Result<CsvValidation> {
        let parse_result = OutputParser::parse_csv(content)?;

        // Basic RFC4180 validation
        let has_headers = !parse_result.headers.is_empty();
        let consistent_columns = parse_result
            .rows
            .iter()
            .all(|row| row.len() == parse_result.column_count);

        Ok(CsvValidation {
            is_rfc4180_compliant: has_headers && consistent_columns,
            has_headers,
            consistent_column_count: consistent_columns,
            row_count: parse_result.row_count,
            column_count: parse_result.column_count,
        })
    }

    /// Validate JSON format compliance
    pub fn validate_json_format(content: &str) -> Result<JsonValidation> {
        let parse_result = OutputParser::parse_json(content)?;

        // Check for expected structure: {"data": [...]}
        let json: Value = serde_json::from_str(content)?;
        let has_data_field = json.get("data").is_some();
        let data_is_array = json.get("data").map(|d| d.is_array()).unwrap_or(false);

        Ok(JsonValidation {
            has_data_field,
            data_is_array,
            row_count: parse_result.row_count,
            column_count: parse_result.column_count,
        })
    }

    /// Validate TSV format compliance
    pub fn validate_tsv_format(content: &str) -> Result<TsvValidation> {
        let parse_result = OutputParser::parse_tsv(content)?;

        // Check for tab delimiters
        let has_tabs = content.contains('\t');
        let has_headers = !parse_result.headers.is_empty();

        Ok(TsvValidation {
            has_tab_delimiters: has_tabs,
            has_headers,
            row_count: parse_result.row_count,
            column_count: parse_result.column_count,
        })
    }

    /// Compare output across different formats for consistency
    pub fn compare_format_consistency(
        csv_content: &str,
        json_content: &str,
        tsv_content: &str,
    ) -> Result<FormatConsistency> {
        let csv_result = OutputParser::parse_csv(csv_content)?;
        let json_result = OutputParser::parse_json(json_content)?;
        let tsv_result = OutputParser::parse_tsv(tsv_content)?;

        let row_counts_match =
            csv_result.row_count == json_result.row_count && json_result.row_count == tsv_result.row_count;

        let column_counts_match =
            csv_result.column_count == json_result.column_count && json_result.column_count == tsv_result.column_count;

        Ok(FormatConsistency {
            row_counts_match,
            column_counts_match,
            csv_row_count: csv_result.row_count,
            json_row_count: json_result.row_count,
            tsv_row_count: tsv_result.row_count,
            csv_column_count: csv_result.column_count,
            json_column_count: json_result.column_count,
            tsv_column_count: tsv_result.column_count,
        })
    }
}

/// Parsed output enumeration
#[derive(Debug)]
pub enum ParsedOutput {
    /// CSV parsed output
    Csv(CsvParseResult),
    /// JSON parsed output
    Json(JsonParseResult),
    /// TSV parsed output
    Tsv(TsvParseResult),
}

impl ParsedOutput {
    /// Get the row count regardless of format
    pub fn row_count(&self) -> usize {
        match self {
            ParsedOutput::Csv(result) => result.row_count,
            ParsedOutput::Json(result) => result.row_count,
            ParsedOutput::Tsv(result) => result.row_count,
        }
    }

    /// Get the column count regardless of format
    pub fn column_count(&self) -> usize {
        match self {
            ParsedOutput::Csv(result) => result.column_count,
            ParsedOutput::Json(result) => result.column_count,
            ParsedOutput::Tsv(result) => result.column_count,
        }
    }
}

/// CSV format validation result
#[derive(Debug)]
pub struct CsvValidation {
    /// Whether the CSV is RFC4180 compliant
    pub is_rfc4180_compliant: bool,
    /// Whether headers are present
    pub has_headers: bool,
    /// Whether all rows have consistent column count
    pub consistent_column_count: bool,
    /// Number of data rows
    pub row_count: usize,
    /// Number of columns
    pub column_count: usize,
}

/// JSON format validation result
#[derive(Debug)]
pub struct JsonValidation {
    /// Whether the JSON has a "data" field
    pub has_data_field: bool,
    /// Whether the "data" field is an array
    pub data_is_array: bool,
    /// Number of data rows
    pub row_count: usize,
    /// Number of columns
    pub column_count: usize,
}

/// TSV format validation result
#[derive(Debug)]
pub struct TsvValidation {
    /// Whether tab delimiters are present
    pub has_tab_delimiters: bool,
    /// Whether headers are present
    pub has_headers: bool,
    /// Number of data rows
    pub row_count: usize,
    /// Number of columns
    pub column_count: usize,
}

/// Format consistency comparison result
#[derive(Debug)]
pub struct FormatConsistency {
    /// Whether row counts match across formats
    pub row_counts_match: bool,
    /// Whether column counts match across formats
    pub column_counts_match: bool,
    /// CSV row count
    pub csv_row_count: usize,
    /// JSON row count
    pub json_row_count: usize,
    /// TSV row count
    pub tsv_row_count: usize,
    /// CSV column count
    pub csv_column_count: usize,
    /// JSON column count
    pub json_column_count: usize,
    /// TSV column count
    pub tsv_column_count: usize,
}

/// Assertion utilities for parsed output
pub struct OutputAssertions;

impl OutputAssertions {
    /// Assert that row count matches expected value
    pub fn assert_row_count(output: &ParsedOutput, expected: usize) -> Result<()> {
        let actual = output.row_count();
        if actual != expected {
            return Err(anyhow::anyhow!("Row count mismatch: expected {}, got {}", expected, actual));
        }
        Ok(())
    }

    /// Assert that column count matches expected value
    pub fn assert_column_count(output: &ParsedOutput, expected: usize) -> Result<()> {
        let actual = output.column_count();
        if actual != expected {
            return Err(anyhow::anyhow!("Column count mismatch: expected {}, got {}", expected, actual));
        }
        Ok(())
    }

    /// Assert that CSV is RFC4180 compliant
    pub fn assert_csv_compliance(validation: &CsvValidation) -> Result<()> {
        if !validation.is_rfc4180_compliant {
            return Err(anyhow::anyhow!("CSV is not RFC4180 compliant"));
        }
        Ok(())
    }

    /// Assert that JSON has expected structure
    pub fn assert_json_structure(validation: &JsonValidation) -> Result<()> {
        if !validation.has_data_field {
            return Err(anyhow::anyhow!("JSON missing 'data' field"));
        }
        if !validation.data_is_array {
            return Err(anyhow::anyhow!("JSON 'data' field is not an array"));
        }
        Ok(())
    }

    /// Assert that formats are consistent
    pub fn assert_format_consistency(consistency: &FormatConsistency) -> Result<()> {
        if !consistency.row_counts_match {
            return Err(anyhow::anyhow!(
                "Row counts don't match: CSV={}, JSON={}, TSV={}",
                consistency.csv_row_count,
                consistency.json_row_count,
                consistency.tsv_row_count
            ));
        }
        if !consistency.column_counts_match {
            return Err(anyhow::anyhow!(
                "Column counts don't match: CSV={}, JSON={}, TSV={}",
                consistency.csv_column_count,
                consistency.json_column_count,
                consistency.tsv_column_count
            ));
        }
        Ok(())
    }
}
