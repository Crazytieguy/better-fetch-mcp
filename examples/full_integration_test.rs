use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Full Integration Test for llms-fetch-mcp");
    println!("{}", "=".repeat(70));
    println!();

    let temp_dir = TempDir::new()?;
    let cache_path = temp_dir.path();

    println!("Cache directory: {}\n", cache_path.display());

    // Test cases
    let test_cases = vec![
        ("Query params", "https://httpbin.org/get?test=value"),
        (
            "Deep path",
            "https://developer.mozilla.org/en-US/docs/Web/JavaScript",
        ),
        (
            ".md file",
            "https://raw.githubusercontent.com/anthropics/anthropic-sdk-python/main/README.md",
        ),
        (".txt file", "https://www.ietf.org/rfc/rfc2616.txt"),
    ];

    for (name, url) in &test_cases {
        println!("Test: {} ({})", name, url);
        println!("{}", "-".repeat(70));

        match test_fetch(cache_path, url).await {
            Ok(results) => {
                println!("✓ Fetch successful!");
                for result in results {
                    println!("  File: {}", result.file_path);
                    println!("    Content type: {}", result.content_type);
                    println!(
                        "    Lines: {}, Words: {}, Chars: {}",
                        result.lines, result.words, result.characters
                    );
                    if let Some(preview) = result.preview {
                        println!("    Preview: {}", preview);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed: {}", e);
            }
        }
        println!();
    }

    println!("{}", "=".repeat(70));
    println!("Integration test complete!");

    Ok(())
}

struct FetchResult {
    file_path: String,
    content_type: String,
    lines: usize,
    words: usize,
    characters: usize,
    preview: Option<String>,
}

async fn test_fetch(
    cache_dir: &std::path::Path,
    url: &str,
) -> Result<Vec<FetchResult>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Get URL variations
    let mut variations = vec![url.to_string()];
    let url_lower = url.to_lowercase();
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if !url_lower.ends_with(".md") && !url_lower.ends_with(".txt") {
        let base = url.trim_end_matches('/');
        variations.push(format!("{base}.md"));
        variations.push(format!("{base}/index.md"));
        variations.push(format!("{base}/llms.txt"));
        variations.push(format!("{base}/llms-full.txt"));
    }

    // Try fetching each variation
    let mut successful_fetches = Vec::new();

    for variation_url in &variations {
        match client
            .get(variation_url)
            .header(
                "Accept",
                "text/markdown, text/x-markdown, text/plain, text/html;q=0.5, */*;q=0.1",
            )
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                let is_html = content_type.contains("text/html");
                let is_markdown = content_type.contains("text/markdown")
                    || content_type.contains("text/x-markdown");

                if let Ok(content) = response.text().await {
                    // Generate file path
                    let file_path = url_to_path(cache_dir, variation_url)?;

                    // Determine content type label
                    let content_type_label =
                        if variation_url.to_lowercase().contains("/llms-full.txt") {
                            "llms-full"
                        } else if variation_url.to_lowercase().contains("/llms.txt") {
                            "llms"
                        } else if is_markdown || variation_url.to_lowercase().ends_with(".md") {
                            "markdown"
                        } else if is_html {
                            "html-converted"
                        } else {
                            "text"
                        };

                    // For HTML, we'd normally clean and convert, but for simplicity we'll just save as-is
                    let content_to_save = content.clone();

                    // Create parent directories
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).await?;
                    }

                    // Write file
                    fs::write(&file_path, &content_to_save).await?;

                    // Calculate stats
                    let lines = content_to_save.lines().count();
                    let words = content_to_save.split_whitespace().count();
                    let characters = content_to_save.chars().count();

                    // Get preview (first 100 chars)
                    let preview = if content_to_save.len() > 100 {
                        Some(format!("{}...", &content_to_save[..100].replace('\n', " ")))
                    } else {
                        Some(content_to_save[..content_to_save.len().min(100)].replace('\n', " "))
                    };

                    successful_fetches.push(FetchResult {
                        file_path: file_path.to_string_lossy().to_string(),
                        content_type: content_type_label.to_string(),
                        lines,
                        words,
                        characters,
                        preview,
                    });
                }
            }
            _ => continue,
        }
    }

    if successful_fetches.is_empty() {
        return Err(format!("Failed to fetch {}", url).into());
    }

    Ok(successful_fetches)
}

fn url_to_path(
    base_dir: &std::path::Path,
    url: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    use url::Url;

    let parsed = Url::parse(url)?;
    let domain = parsed.host_str().ok_or("No host in URL")?;

    let mut path = base_dir.join(domain);

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
    if !path.starts_with(base_dir) {
        return Err("Path traversal detected".into());
    }

    Ok(path)
}
