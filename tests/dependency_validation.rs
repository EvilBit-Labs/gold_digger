use std::process::Command;

/// Test to verify that rustls is always present (TLS always available)
#[test]
fn test_rustls_always_available() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-f",
            "{p} {f}",
            "--no-default-features",
            "--features",
            "json,csv",
        ])
        .output()
        .expect("Failed to run cargo tree");

    let tree_output = String::from_utf8(output.stdout).unwrap();

    // Verify rustls is present (TLS always available)
    assert!(tree_output.contains("rustls"), "rustls dependency not found in tree: {}", tree_output);

    // Verify rustls-native-certs is present for platform certificate store integration
    assert!(
        tree_output.contains("rustls-native-certs"),
        "rustls-native-certs dependency not found in tree: {}",
        tree_output
    );

    // Verify native-tls is NOT present (rustls-only implementation)
    assert!(!tree_output.contains("native-tls"), "native-tls found in rustls-only implementation: {}", tree_output);
}

/// Test to verify that TLS dependencies are always present (TLS always available)
#[test]
fn test_tls_always_available() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-f",
            "{p} {f}",
            "--no-default-features",
            "--features",
            "json,csv",
        ])
        .output()
        .expect("Failed to run cargo tree");

    let tree_output = String::from_utf8(output.stdout).unwrap();

    // Verify native-tls is NOT present (rustls-only implementation)
    assert!(
        !tree_output.contains("native-tls"),
        "native-tls dependency found in rustls-only implementation: {}",
        tree_output
    );

    // Verify rustls IS present (TLS always available)
    assert!(
        tree_output.contains("rustls"),
        "rustls dependency not found - TLS should always be available: {}",
        tree_output
    );
}

/// Test to verify TLS is always available (rustls-only implementation)
#[test]
fn test_tls_always_enabled() {
    // Test with basic features (TLS always available)
    let output = Command::new("cargo")
        .args([
            "tree",
            "-f",
            "{p} {f}",
            "--no-default-features",
            "--features",
            "json,csv",
        ])
        .output()
        .expect("Failed to run cargo tree");

    let tree_output = String::from_utf8(output.stdout).unwrap();

    // Should contain mysql with rustls (rustls-only implementation)
    assert!(
        tree_output.contains("mysql") && tree_output.contains("rustls"),
        "mysql with rustls not found - TLS should always be available: {}",
        tree_output
    );

    // Should contain rustls-native-certs for platform certificate store integration
    assert!(
        tree_output.contains("rustls-native-certs"),
        "rustls-native-certs not found - should always be available: {}",
        tree_output
    );

    // Should NOT contain native-tls (rustls-only implementation)
    assert!(!tree_output.contains("native-tls"), "native-tls found in rustls-only implementation: {}", tree_output);
}

/// Test to verify TLS dependencies are always present (TLS always available)
#[test]
fn test_tls_dependencies_always_present() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-f",
            "{p} {f}",
            "--no-default-features",
            "--features",
            "json,csv",
            "--no-dev-dependencies", // Exclude dev dependencies to focus on production dependencies
        ])
        .output()
        .expect("Failed to run cargo tree");

    let tree_output = String::from_utf8(output.stdout).unwrap();

    // Verify native-tls is NOT present (rustls-only implementation)
    assert!(
        !tree_output.contains("native-tls"),
        "native-tls dependency found in rustls-only implementation: {}",
        tree_output
    );

    // Verify rustls IS present (TLS always available)
    assert!(
        tree_output.contains("rustls"),
        "rustls dependency not found - TLS should always be available: {}",
        tree_output
    );
}

/// Helper function to parse cargo tree output and extract dependency names
fn parse_dependency_tree(tree_output: &str) -> Vec<String> {
    tree_output
        .lines()
        .filter_map(|line| {
            // Remove tree drawing characters and extract package name
            let cleaned = line.trim_start_matches(&['├', '│', '└', '─', ' '][..]);

            // Parse lines like "mysql v26.0.1" or "native-tls v0.2.11"
            if let Some(first_space) = cleaned.find(' ') {
                let dep_name = &cleaned[..first_space];
                if !dep_name.is_empty() {
                    Some(dep_name.to_string())
                } else {
                    None
                }
            } else if !cleaned.is_empty() {
                // Handle lines with just the package name
                Some(cleaned.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Test the dependency tree parsing logic
#[test]
fn test_dependency_tree_parsing() {
    let sample_output = r#"mysql v26.0.1
├── native-tls v0.2.11
│   ├── lazy_static v1.4.0
│   └── libc v0.2.147
└── serde v1.0.183"#;

    let dependencies = parse_dependency_tree(sample_output);

    println!("Parsed dependencies: {:?}", dependencies);

    assert!(dependencies.contains(&"mysql".to_string()));
    assert!(dependencies.contains(&"native-tls".to_string()));
    assert!(dependencies.contains(&"lazy_static".to_string()));
    assert!(dependencies.contains(&"libc".to_string()));
    assert!(dependencies.contains(&"serde".to_string()));
}

/// Test to verify feature combinations work correctly (rustls-only implementation)
#[test]
fn test_feature_combinations() {
    // Test json + csv (common combination, TLS always available)
    let output = Command::new("cargo")
        .args([
            "tree",
            "-f",
            "{p} {f}",
            "--no-default-features",
            "--features",
            "json,csv",
        ])
        .output()
        .expect("Failed to run cargo tree with json,csv features");

    let tree_output = String::from_utf8(output.stdout).unwrap();

    // Should have rustls (TLS always available)
    assert!(tree_output.contains("rustls"), "rustls not found with json,csv features: {}", tree_output);

    // Should NOT have native-tls (rustls-only implementation)
    assert!(!tree_output.contains("native-tls"), "native-tls found in rustls-only implementation: {}", tree_output);

    // Should have serde_json and csv dependencies
    assert!(
        tree_output.contains("serde_json") || tree_output.contains("serde"),
        "JSON dependencies not found with json feature: {}",
        tree_output
    );

    assert!(tree_output.contains("csv"), "CSV dependency not found with csv feature: {}", tree_output);
}

/// Legacy test to verify cargo-deny is available for CI validation
#[test]
fn test_cargo_deny_available() {
    let output = std::process::Command::new("cargo").args(["deny", "--version"]).output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("✓ cargo-deny is available: {}", version.trim());
        },
        _ => {
            // Don't panic in tests - just skip if cargo-deny isn't installed
            println!("⚠ cargo-deny not installed - install with: cargo install cargo-deny");
        },
    }
}
