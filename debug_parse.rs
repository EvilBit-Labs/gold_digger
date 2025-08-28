use std::fs;

fn main() {
    let content = fs::read_to_string("/tmp/tree_debug.txt").unwrap();
    let dependencies = parse_dependency_tree(&content);

    println!("Total dependencies found: {}", dependencies.len());
    println!(
        "Dependencies containing 'rustls': {:?}",
        dependencies.iter().filter(|d| d.contains("rustls")).collect::<Vec<_>>()
    );

    // Check if rustls is directly present
    if dependencies.contains(&"rustls".to_string()) {
        println!("✓ rustls found in dependencies");
    } else {
        println!("✗ rustls NOT found in dependencies");
    }

    // Show first 20 dependencies
    println!("First 20 dependencies: {:?}", &dependencies[..std::cmp::min(20, dependencies.len())]);
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
