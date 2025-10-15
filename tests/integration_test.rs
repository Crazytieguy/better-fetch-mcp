use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore] // Run with `cargo test -- --ignored`
fn test_fetch_convex_docs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let input = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "fetch",
            "arguments": {
                "url": "https://docs.convex.dev/"
            }
        }
    });

    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg(cache_dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.to_string().as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Output: {}", stdout);

    // Check that files were created
    let docs_convex_path = cache_dir.join("docs.convex.dev");
    assert!(
        docs_convex_path.exists(),
        "Cache directory should be created"
    );

    // Check gitignore was created
    let gitignore = cache_dir.join(".gitignore");
    assert!(gitignore.exists(), "Gitignore should be created");
    let gitignore_content = fs::read_to_string(gitignore).unwrap();
    assert_eq!(gitignore_content, "*\n");
}

#[test]
#[ignore]
fn test_fetch_svelte_docs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let input = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "fetch",
            "arguments": {
                "url": "https://svelte.dev"
            }
        }
    });

    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg(cache_dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.to_string().as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Output: {}", stdout);

    // Check that files were created
    let svelte_path = cache_dir.join("svelte.dev");
    assert!(svelte_path.exists(), "Cache directory should be created");
}

#[test]
fn test_url_variations_logic() {
    // Test that .md URLs don't generate variations
    let md_url = "https://example.com/page.md";
    let variations = get_url_variations(md_url);
    assert_eq!(variations.len(), 1);
    assert_eq!(variations[0], md_url);

    // Test that regular URLs generate all variations
    let regular_url = "https://example.com/page";
    let variations = get_url_variations(regular_url);
    assert_eq!(variations.len(), 5);
    assert_eq!(variations[0], "https://example.com/page");
    assert_eq!(variations[1], "https://example.com/page.md");
    assert_eq!(variations[2], "https://example.com/page/index.md");
    assert_eq!(variations[3], "https://example.com/page/llms.txt");
    assert_eq!(variations[4], "https://example.com/page/llms-full.txt");
}

fn get_url_variations(url: &str) -> Vec<String> {
    let mut variations = vec![url.to_string()];

    let url_lower = url.to_lowercase();
    if url_lower.ends_with(".md") || url_lower.ends_with(".txt") {
        return variations;
    }

    let base = url.trim_end_matches('/');
    variations.push(format!("{}.md", base));
    variations.push(format!("{}/index.md", base));
    variations.push(format!("{}/llms.txt", base));
    variations.push(format!("{}/llms-full.txt", base));

    variations
}
