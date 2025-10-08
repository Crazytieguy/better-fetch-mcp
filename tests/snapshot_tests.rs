use std::path::PathBuf;
use insta::assert_debug_snapshot;

#[tokio::test]
async fn test_fetch_output_structure() {
    let _temp_dir = tempfile::tempdir().unwrap();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let test_url = "https://httpbin.org/html";

    match client
        .get(test_url)
        .header("Accept", "text/markdown, text/plain, text/html;q=0.5, */*;q=0.1")
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

            let snapshot_data = format!(
                "URL: {}\nContent-Type: {}\nIs HTML: {}\nIs Markdown: {}",
                test_url, content_type, is_html, is_markdown
            );

            assert_debug_snapshot!("fetch_httpbin_metadata", snapshot_data);
        }
        Ok(response) => {
            panic!("Request failed with status: {}", response.status());
        }
        Err(e) => {
            panic!("Request failed: {}", e);
        }
    }
}

#[test]
fn test_url_variations_snapshot() {
    let regular_url = "https://example.com/docs";
    let variations = get_url_variations(regular_url);
    assert_debug_snapshot!("url_variations_regular", variations);

    let md_url = "https://example.com/readme.md";
    let md_variations = get_url_variations(md_url);
    assert_debug_snapshot!("url_variations_md_file", md_variations);

    let txt_url = "https://example.com/file.txt";
    let txt_variations = get_url_variations(txt_url);
    assert_debug_snapshot!("url_variations_txt_file", txt_variations);
}

#[test]
fn test_content_type_detection() {
    let test_cases = vec![
        ("text/html; charset=utf-8", true, false),
        ("text/markdown", false, true),
        ("text/x-markdown", false, true),
        ("text/plain", false, false),
        ("application/json", false, false),
    ];

    for (content_type, expected_html, expected_markdown) in &test_cases {
        let is_html = content_type.contains("text/html");
        let is_markdown = content_type.contains("text/markdown")
            || content_type.contains("text/x-markdown");

        assert_eq!(
            is_html, *expected_html,
            "Content-Type '{}' HTML detection failed",
            content_type
        );
        assert_eq!(
            is_markdown, *expected_markdown,
            "Content-Type '{}' Markdown detection failed",
            content_type
        );
    }

    let snapshot_data = test_cases
        .iter()
        .map(|(ct, is_html, is_md)| {
            format!("{ct} -> HTML: {is_html}, MD: {is_md}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert_debug_snapshot!("content_type_detection", snapshot_data);
}

#[test]
fn test_file_stats_calculation() {
    let test_content = "# Example\n\nThis is a test.\n\nWith multiple lines.";
    let (lines, words, characters) = count_stats(test_content);

    let stats = format!(
        "Content length: {} bytes\nLines: {}\nWords: {}\nCharacters: {}",
        test_content.len(),
        lines,
        words,
        characters
    );

    assert_debug_snapshot!("file_stats", stats);
}

#[test]
fn test_path_generation() {
    let base = PathBuf::from("/cache");

    let test_cases = [
        "https://example.com/docs",
        "https://example.com/docs/page.md",
        "https://example.com/",
        "https://docs.rust-lang.org/book/ch01-00-getting-started.html",
    ];

    let paths: Vec<String> = test_cases
        .iter()
        .map(|url| {
            url_to_path(&base, url)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("Error: {e}"))
        })
        .collect();

    assert_debug_snapshot!("path_generation", paths);
}

fn get_url_variations(url: &str) -> Vec<String> {
    let mut variations = vec![url.to_string()];

    let url_lower = url.to_lowercase();
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if url_lower.ends_with(".md") || url_lower.ends_with(".txt") {
        return variations;
    }

    let base = url.trim_end_matches('/');
    variations.push(format!("{base}.md"));
    variations.push(format!("{base}/index.md"));
    variations.push(format!("{base}/llms.txt"));
    variations.push(format!("{base}/llms-full.txt"));

    variations
}

fn url_to_path(base_dir: &std::path::Path, url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parsed = url::Url::parse(url)?;
    let domain = parsed.host_str().ok_or("No host in URL")?;

    let mut path = base_dir.join(domain);

    let url_path = parsed.path().trim_start_matches('/');
    if !url_path.is_empty() {
        path.push(url_path);
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

    Ok(path)
}

fn count_stats(content: &str) -> (usize, usize, usize) {
    let lines = content.lines().count();
    let words = content.split_whitespace().count();
    let characters = content.chars().count();
    (lines, words, characters)
}
