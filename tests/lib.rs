#[cfg(test)]
mod tests {
    use gold_digger::{FormatWriter, json::JsonWriter, rows_to_strings};
    use gold_digger::{csv, tab};
    use mysql::Row;
    use std::io::Cursor;

    #[test]
    fn test_get_extension_from_filename() {
        assert_eq!(gold_digger::get_extension_from_filename("test.csv"), Some("csv"));
        assert_eq!(gold_digger::get_extension_from_filename("test.json"), Some("json"));
        assert_eq!(gold_digger::get_extension_from_filename("test"), None);
        assert_eq!(gold_digger::get_extension_from_filename("test."), Some(""));
    }

    #[test]
    fn test_rows_to_strings_empty() {
        let rows: Vec<Row> = vec![];
        let result = rows_to_strings(rows).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_json_writer_format() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = JsonWriter::new(&mut cursor, false);

        let columns = vec!["id".to_string(), "item_name".to_string()];
        let row = vec!["1".to_string(), "test".to_string()];

        writer.write_header(&columns).unwrap();
        writer.write_row(&row).unwrap();
        writer.finalize().unwrap();

        let output = String::from_utf8(cursor.into_inner()).unwrap();
        assert!(output.contains(r#"{"data":["#));
        assert!(output.contains(r#"]}"#));
        // With type inference, "1" becomes integer 1, "test" remains string
        assert!(output.contains(r#""id":1"#) || output.contains(r#""id":"1""#));
        assert!(output.contains(r#""item_name":"test""#));
    }

    #[test]
    fn test_csv_generic_iterators() {
        // Test with Vec<Vec<String>> (backward compatibility)
        let data = vec![
            vec!["item_name".to_string(), "age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let mut output = Vec::new();
        csv::write(data, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("item_name,age"));
        assert!(output_str.contains("Alice,30"));
        assert!(output_str.contains("Bob,25"));

        // Test with iterator of iterators
        let data_iter = vec![
            vec!["item_name".to_string(), "age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ]
        .into_iter();

        let mut output2 = Vec::new();
        csv::write(data_iter, &mut output2).unwrap();
        let output_str2 = String::from_utf8(output2).unwrap();

        assert_eq!(output_str, output_str2);
    }

    #[test]
    fn test_csv_bytes_interface() {
        // Test with bytes interface for better performance
        let data = vec![
            vec!["item_name".to_string(), "age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let mut output = Vec::new();
        csv::write_bytes(data, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("item_name,age"));
        assert!(output_str.contains("Alice,30"));
        assert!(output_str.contains("Bob,25"));
    }

    #[test]
    fn test_tsv_generic_iterators() {
        let data = vec![
            vec!["item_name".to_string(), "age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let mut output = Vec::new();
        tab::write(data, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("item_name\tage"));
        assert!(output_str.contains("Alice\t30"));
        assert!(output_str.contains("Bob\t25"));
    }

    // More comprehensive tests would require constructing mock mysql::Row objects,
    // which is non-trivial without a database or more advanced mocking.
}
