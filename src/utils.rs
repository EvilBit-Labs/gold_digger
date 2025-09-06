/// Utility functions for the gold_digger application
use regex::Regex;

/// Redacts sensitive information from SQL error messages
///
/// This function uses regex patterns to identify and replace sensitive information
/// such as passwords, tokens, API keys, and secrets with redaction markers.
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

    // Define patterns for sensitive information (case-insensitive)
    let patterns = [
        // Password patterns - using simpler regex to avoid character class issues
        (r"(?i)password\s*[=:]\s*\S+", "***REDACTED***"),
        (r"(?i)identified\s+by\s+\S+", "***REDACTED***"),
        // Token patterns - handle both "token=value" and "token value" formats
        (r"(?i)token\s*[=:]\s*\S+", "***REDACTED***"),
        (r"(?i)token\s+\S+", "***REDACTED***"),
        // API key patterns
        (r"(?i)api[_-]?key\s*[=:]\s*\S+", "***REDACTED***"),
        // Secret patterns - handle both "secret=value" and "secret value" formats
        (r"(?i)secret\s*[=:]\s*\S+", "***REDACTED***"),
        (r"(?i)secret\s+\S+", "***REDACTED***"),
        // Connection string passwords
        (r"(?i)://[^:]+:[^@]+@", "://***:***@"),
    ];

    for (pattern, replacement) in &patterns {
        match Regex::new(pattern) {
            Ok(re) => {
                redacted = re.replace_all(&redacted, *replacement).to_string();
            },
            Err(_e) => {
                // Log regex compilation errors in debug builds for development
                #[cfg(debug_assertions)]
                eprintln!("Warning: Failed to compile regex pattern '{}': {}", pattern, _e);
            },
        }
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
