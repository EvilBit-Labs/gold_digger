/// Utility functions for the gold_digger application
use std::sync::OnceLock;

use regex::Regex;

/// Pre-compiled redaction patterns for sensitive information in error messages.
/// Each entry is a (pattern, replacement) tuple compiled once on first use.
static REDACTION_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

/// Returns the pre-compiled redaction regex patterns, initializing them on
/// first call.
fn get_redaction_patterns() -> &'static Vec<(Regex, &'static str)> {
    REDACTION_PATTERNS.get_or_init(|| {
        let pattern_defs: &[(&str, &str)] = &[
            (r"(?i)password\s*[=:]\s*\S+", "***REDACTED***"),
            (r"(?i)identified\s+by\s+\S+", "***REDACTED***"),
            (r"(?i)token\s*[=:]\s*\S+", "***REDACTED***"),
            (r"(?i)token\s+\S+", "***REDACTED***"),
            (r"(?i)api[_-]?key\s*[=:]\s*\S+", "***REDACTED***"),
            (r"(?i)secret\s*[=:]\s*\S+", "***REDACTED***"),
            (r"(?i)secret\s+\S+", "***REDACTED***"),
            (r"(?i)://[^:]+:[^@]+@", "://***:***@"),
        ];

        pattern_defs
            .iter()
            .filter_map(|(pattern, replacement)| match Regex::new(pattern) {
                Ok(re) => Some((re, *replacement)),
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "Warning: Failed to compile regex pattern '{}': {}",
                        pattern, _e
                    );
                    None
                }
            })
            .collect()
    })
}

/// Redacts sensitive information from SQL error messages
///
/// This function uses pre-compiled regex patterns to identify and replace
/// sensitive information such as passwords, tokens, API keys, and secrets
/// with redaction markers. Patterns are compiled once on first call using
/// `OnceLock` for thread-safe lazy initialization.
///
/// # Arguments
/// * `message` - The error message to redact
///
/// # Returns
/// * `String` - The redacted error message
///
/// # Example
/// ```
/// use gold_digger::utils::redact_sql_error;
///
/// let error = "Error: Access denied for user 'test' (using password: YES)";
/// let redacted = redact_sql_error(error);
/// assert!(redacted.contains("***REDACTED***"));
/// assert!(!redacted.contains("password"));
/// ```
pub fn redact_sql_error(message: &str) -> String {
    let mut redacted = message.to_string();

    for (re, replacement) in get_redaction_patterns() {
        redacted = re.replace_all(&redacted, *replacement).to_string();
    }

    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_sql_error() {
        // Test that sensitive information is redacted from error messages
        let error_with_password = "Error: Access denied for user 'test' (using password: YES)";
        let redacted = redact_sql_error(error_with_password);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("password"));

        let error_with_identified_by = "Error: CREATE USER failed with identified by 'secret123'";
        let redacted = redact_sql_error(error_with_identified_by);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("identified by"));

        let error_with_token = "Error: Invalid token abc123";
        let redacted = redact_sql_error(error_with_token);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("token"));

        let error_with_secret = "Error: Invalid secret key";
        let redacted = redact_sql_error(error_with_secret);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("secret"));

        let error_with_key = "Error: api_key=sensitive_value";
        let redacted = redact_sql_error(error_with_key);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("key"));

        let normal_error = "Error: Table 'test.users' doesn't exist";
        let redacted = redact_sql_error(normal_error);
        assert_eq!(redacted, normal_error); // Should be unchanged
    }
}
