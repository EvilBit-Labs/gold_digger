use anyhow::Error;
use std::process;

/// Exit code constants as defined in the product specification
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_NO_ROWS: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;
pub const EXIT_DB_AUTH_ERROR: i32 = 3;
pub const EXIT_QUERY_ERROR: i32 = 4;
pub const EXIT_IO_ERROR: i32 = 5;

/// Maps an error to the appropriate exit code and exits the process
///
/// # Arguments
///
/// * `error` - The error to map to an exit code
/// * `context` - Optional context message to log before exiting
///
/// This function never returns as it calls `process::exit`
pub fn exit_with_error(error: Error, context: Option<&str>) -> ! {
    let exit_code = map_error_to_exit_code(&error);
    let error_msg = error.to_string();

    // Log the error with context if provided
    if let Some(ctx) = context {
        eprintln!("{}: {}", ctx, error_msg);
    } else {
        eprintln!("{}", error_msg);
    }

    process::exit(exit_code);
}

/// Maps an error to the appropriate exit code without exiting
///
/// # Arguments
///
/// * `error` - The error to map to an exit code
///
/// # Returns
///
/// The appropriate exit code for the given error
pub fn map_error_to_exit_code(error: &Error) -> i32 {
    let error_string = error.to_string().to_lowercase();

    // Check for specific error patterns
    if error_string.contains("no records found") || error_string.contains("no rows") {
        return EXIT_NO_ROWS;
    }

    if error_string.contains("missing")
        || error_string.contains("invalid") && !error_string.contains("invalid certificate format") && !error_string.contains("type conversion")
        || error_string.contains("configuration")
        || error_string.contains("mutually exclusive")
        || error_string.contains("tls feature not enabled")
        || error_string.contains("certificate file not found")
    {
        return EXIT_CONFIG_ERROR;
    }

    if error_string.contains("access denied")
        || error_string.contains("authentication")
        || error_string.contains("connection")
        || error_string.contains("tls connection failed")
        || error_string.contains("tls handshake failed")
        || error_string.contains("certificate validation failed")
        || error_string.contains("unsupported tls version")
        || error_string.contains("mysql") && (error_string.contains("auth") || error_string.contains("connect"))
    {
        return EXIT_DB_AUTH_ERROR;
    }

    if error_string.contains("query")
        || error_string.contains("sql")
        || error_string.contains("syntax")
        || error_string.contains("type conversion")
        || error_string.contains("from_value")
    {
        return EXIT_QUERY_ERROR;
    }

    if error_string.contains("file")
        || error_string.contains("io")
        || error_string.contains("read")
        || error_string.contains("write")
        || error_string.contains("permission")
        || error_string.contains("invalid certificate format")
    {
        return EXIT_IO_ERROR;
    }

    // Default to query error for unknown errors
    EXIT_QUERY_ERROR
}

/// Exits with success code (0)
///
/// # Arguments
///
/// * `message` - Optional success message to print before exiting
pub fn exit_success(message: Option<&str>) -> ! {
    if let Some(msg) = message {
        println!("{}", msg);
    }
    process::exit(EXIT_SUCCESS);
}

/// Exits with no rows code (1)
///
/// # Arguments
///
/// * `message` - Optional message to print before exiting
pub fn exit_no_rows(message: Option<&str>) -> ! {
    if let Some(msg) = message {
        eprintln!("{}", msg);
    }
    process::exit(EXIT_NO_ROWS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_map_error_to_exit_code_no_rows() {
        let error = anyhow!("No records found in database");
        assert_eq!(map_error_to_exit_code(&error), EXIT_NO_ROWS);

        let error = anyhow!("Query returned no rows");
        assert_eq!(map_error_to_exit_code(&error), EXIT_NO_ROWS);
    }

    #[test]
    fn test_map_error_to_exit_code_config() {
        let error = anyhow!("Missing database URL");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("Invalid configuration");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("Mutually exclusive flags");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("TLS feature not enabled");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("Certificate file not found");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_db_auth() {
        let error = anyhow!("Access denied for user");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Authentication failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Connection refused");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("MySQL authentication error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("TLS connection failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("TLS handshake failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Certificate validation failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Unsupported TLS version");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_query() {
        let error = anyhow!("Query execution failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("SQL syntax error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("Type conversion error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("from_value error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        // Test specific type conversion errors from our implementation
        let error = anyhow!("Type conversion error: Invalid month value 13 in date");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("Type conversion failed during row processing");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_io() {
        let error = anyhow!("File not found");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("IO error occurred");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Permission denied");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Failed to read file");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Failed to write file");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Invalid certificate format");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_default() {
        let error = anyhow!("Unknown error occurred");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);
    }

    #[test]
    fn test_exit_code_constants() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_NO_ROWS, 1);
        assert_eq!(EXIT_CONFIG_ERROR, 2);
        assert_eq!(EXIT_DB_AUTH_ERROR, 3);
        assert_eq!(EXIT_QUERY_ERROR, 4);
        assert_eq!(EXIT_IO_ERROR, 5);
    }
}
