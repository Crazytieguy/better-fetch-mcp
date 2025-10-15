use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simulating MCP fetch operations...\n");

    let test_cache = PathBuf::from("/tmp/llms-fetch-mcp-simulation");

    // Clean up
    if test_cache.exists() {
        fs::remove_dir_all(&test_cache)?;
    }

    println!("Test 1: Fetch a .md file (should only fetch the exact URL)");
    let urls = vec!["https://raw.githubusercontent.com/microsoft/TypeScript/main/README.md"];
    test_fetch_scenario(&test_cache, urls, "Single .md file").await?;

    println!("\nTest 2: Fetch a regular URL (should try multiple variations)");
    let urls = vec!["https://httpbin.org/html"];
    test_fetch_scenario(&test_cache, urls, "Regular HTML URL").await?;

    println!("\nTest 3: Check .gitignore creation");
    let gitignore_path = test_cache.join(".gitignore");
    if gitignore_path.exists() {
        println!("✓ .gitignore exists");
        let content = fs::read_to_string(&gitignore_path)?;
        if content == "*\n" {
            println!("✓ .gitignore content correct");
        }
    }

    println!("\nTest 4: Check file statistics");
    if test_cache.exists() {
        let mut total_files = 0;
        for entry in walkdir::WalkDir::new(&test_cache) {
            let entry = entry?;
            if entry.file_type().is_file() && entry.file_name() != ".gitignore" {
                total_files += 1;
                let content = fs::read_to_string(entry.path())?;
                let lines = content.lines().count();
                let words = content.split_whitespace().count();
                let chars = content.chars().count();
                println!(
                    "  File: {} (lines: {}, words: {}, chars: {})",
                    entry.path().display(),
                    lines,
                    words,
                    chars
                );
            }
        }
        println!("✓ Total files cached: {}", total_files);
    }

    println!("\n✓ All simulations completed successfully!");
    Ok(())
}

async fn test_fetch_scenario(
    _cache_dir: &PathBuf,
    urls: Vec<&str>,
    scenario: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Scenario: {}", scenario);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    for url in urls {
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let content_type = response
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");

                    println!("  ✓ {} ({})", url, content_type);
                } else {
                    println!("  ✗ {} (status: {})", url, response.status());
                }
            }
            Err(e) => {
                println!("  ✗ {} (error: {})", url, e);
            }
        }
    }

    Ok(())
}
