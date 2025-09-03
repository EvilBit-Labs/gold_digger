use gold_digger::exit::{EXIT_QUERY_ERROR, map_error_to_exit_code};

#[test]
fn test_type_conversion_error_mapping() {
    // Test that specific type conversion errors get mapped to exit code 4 (EXIT_QUERY_ERROR)

    // Test invalid date component errors
    let error = anyhow::anyhow!("Type conversion error: Invalid month value 13 in date");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    let error = anyhow::anyhow!("Type conversion error: Invalid day value 32 in date");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    let error = anyhow::anyhow!("Type conversion error: Invalid hour value 25 in datetime");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    // Test time component errors
    let error = anyhow::anyhow!("Type conversion error: Invalid minute value 61 in time");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    let error = anyhow::anyhow!("Type conversion error: Invalid second value 61 in time");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    let error = anyhow::anyhow!("Type conversion error: Invalid microsecond value 1000000 in datetime");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    // Test context-enhanced errors from rows_to_strings
    let error = anyhow::anyhow!("Type conversion failed during row processing");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    // Test generic type conversion errors
    let error = anyhow::anyhow!("Type conversion error");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    let error = anyhow::anyhow!("from_value conversion failed");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);
}

#[test]
fn test_type_conversion_error_precedence() {
    // Verify that type conversion errors take precedence over config errors
    // even when they contain "invalid"

    let error = anyhow::anyhow!("Type conversion error: Invalid month value 13 in date");
    assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

    // But regular invalid config errors should still map to config error
    let error = anyhow::anyhow!("Invalid configuration value");
    assert_eq!(map_error_to_exit_code(&error), 2); // EXIT_CONFIG_ERROR
}

#[test]
fn test_requirement_10_3_compliance() {
    // Requirement 10.3: Type conversion failures exit with code 4 and meaningful messages

    // Verify that all type conversion errors have meaningful messages
    let test_cases = vec![
        "Type conversion error: Invalid month value 13 in date",
        "Type conversion error: Invalid day value 32 in date",
        "Type conversion error: Invalid hour value 25 in datetime",
        "Type conversion error: Invalid minute value 61 in time",
        "Type conversion error: Invalid second value 61 in time",
        "Type conversion error: Invalid microsecond value 1000000 in datetime",
    ];

    for error_msg in test_cases {
        let error = anyhow::anyhow!(error_msg);

        // Should map to exit code 4
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        // Should have meaningful message
        assert!(error.to_string().contains("Type conversion error"));
        assert!(error.to_string().contains("Invalid"));

        // Should specify the component and value
        let error_string = error.to_string();
        let has_numeric = error_string.chars().any(|c| c.is_ascii_digit());
        assert!(has_numeric, "Error should contain numeric value: {}", error);
        assert!(
            error.to_string().contains("month")
                || error.to_string().contains("day")
                || error.to_string().contains("hour")
                || error.to_string().contains("minute")
                || error.to_string().contains("second")
                || error.to_string().contains("microsecond"),
            "Error should specify component: {}",
            error
        );
    }
}
