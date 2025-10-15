use std::path::PathBuf;

fn test_url_to_path(base_dir: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let base = PathBuf::from(base_dir);

    // Test the actual url_to_path function
    // Since it's not public, we'll need to copy the logic or make it public
    println!("Testing URL: {}", url);
    println!("Base dir: {}", base_dir);

    use url::Url;
    let parsed = Url::parse(url)?;
    let domain = parsed.host_str().ok_or("No host in URL")?;

    let mut path = base.join(domain);

    let url_path = parsed.path().trim_start_matches('/');

    // Security: Sanitize path components to prevent directory traversal
    if !url_path.is_empty() {
        for component in url_path.split('/') {
            if component == ".." || component == "." {
                return Err("Invalid path component in URL".into());
            }
            if !component.is_empty() {
                path.push(component);
            }
        }
    }

    // Determine if we need to add an index file
    let needs_index = if url_path.is_empty() {
        true
    } else {
        let last_segment = url_path.split('/').next_back().unwrap_or("");
        std::path::Path::new(last_segment).extension().is_none()
    };

    if needs_index {
        path.push("index");
    }

    if let Some(query) = parsed.query() {
        let current_ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let new_ext = if current_ext.is_empty() {
            format!("?{query}")
        } else {
            format!("{current_ext}?{query}")
        };
        path.set_extension(new_ext);
    }

    // Security: Verify final path is within base directory
    println!("  Generated path: {:?}", path);
    println!("  Starts with base? {}", path.starts_with(&base));

    if !path.starts_with(&base) {
        return Err("Path traversal detected".into());
    }

    println!("  ✓ Path validated successfully\n");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing edge cases for URL to path conversion\n");
    println!("{}", "=".repeat(60));
    println!();

    // Test 1: URL with query parameters
    test_url_to_path(".llms-fetch-mcp", "https://httpbin.org/get?test=value")?;

    // Test 2: Deep path
    test_url_to_path(
        ".llms-fetch-mcp",
        "https://developer.mozilla.org/en-US/docs/Web/JavaScript",
    )?;

    // Test 3: URL ending in .md
    test_url_to_path(
        ".llms-fetch-mcp",
        "https://raw.githubusercontent.com/anthropics/anthropic-sdk-python/main/README.md",
    )?;

    // Test 4: URL ending in .txt
    test_url_to_path(".llms-fetch-mcp", "https://www.ietf.org/rfc/rfc2616.txt")?;

    println!("{}", "=".repeat(60));
    println!("All edge case tests passed!");

    Ok(())
}
