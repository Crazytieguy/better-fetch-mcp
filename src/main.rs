#![warn(clippy::pedantic)]

use regex::Regex;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData as McpError, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::fs;

#[derive(Clone)]
struct FetchServer {
    cache_dir: Arc<PathBuf>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FetchInput {
    url: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FileInfo {
    path: String,
    source_url: String,
    content_type: String,
    lines: usize,
    words: usize,
    characters: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FetchOutput {
    files: Vec<FileInfo>,
}

#[derive(Debug)]
struct FetchResult {
    url: String,
    content: String,
    is_html: bool,
    is_markdown: bool,
}

#[derive(Debug)]
enum FetchAttempt {
    Success(FetchResult),
    HttpError { url: String, status: u16 },
    NetworkError { url: String },
}

async fn fetch_url(client: &reqwest::Client, url: &str) -> FetchAttempt {
    match client
        .get(url)
        .header(
            "Accept",
            "text/markdown, text/x-markdown, text/plain, text/html;q=0.5, */*;q=0.1",
        )
        .header(
            "User-Agent",
            "llms-fetch-mcp/0.1.1 (+https://github.com/crazytieguy/llms-fetch-mcp)",
        )
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status().as_u16();
            if response.status().is_success() {
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                let is_html = content_type.contains("text/html");
                let is_markdown = content_type.contains("text/markdown")
                    || content_type.contains("text/x-markdown");

                match response.text().await {
                    Ok(content) => FetchAttempt::Success(FetchResult {
                        url: url.to_string(),
                        content,
                        is_html,
                        is_markdown,
                    }),
                    Err(_) => FetchAttempt::NetworkError {
                        url: url.to_string(),
                    },
                }
            } else {
                FetchAttempt::HttpError {
                    url: url.to_string(),
                    status,
                }
            }
        }
        Err(_) => FetchAttempt::NetworkError {
            url: url.to_string(),
        },
    }
}

fn get_url_variations(url: &str) -> Vec<String> {
    let mut variations = vec![url.to_string()];

    let url_lower = url.to_lowercase();
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if url_lower.ends_with(".md") || url_lower.ends_with(".txt") {
        return variations;
    }

    // Don't try variations for URLs with query parameters
    if url.contains('?') {
        return variations;
    }

    let base = url.trim_end_matches('/');

    // Check if this is a GitHub URL by parsing the domain
    let is_github = if let Ok(parsed) = url::Url::parse(url) {
        parsed.domain() == Some("github.com")
    } else {
        false
    };

    // For GitHub URLs, try converting to raw.githubusercontent.com
    if is_github
        && let Ok(parsed) = url::Url::parse(url)
    {
        let path = parsed.path();
        // GitHub URL format: /owner/repo/tree/branch/path or /owner/repo/blob/branch/path
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // Handle /blob/ URLs - convert directly to raw
        if parts.len() >= 4 && parts[2] == "blob" {
            let owner = parts[0];
            let repo = parts[1];
            // Everything after /blob/ is branch/path, but we can't reliably split them
            // For single-segment branch names (most common), this works
            let branch_and_path = parts[3..].join("/");
            variations.push(format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/{branch_and_path}"
            ));
        }

        // Handle /tree/ URLs - add README.md
        if parts.len() >= 4 && parts[2] == "tree" {
            let owner = parts[0];
            let repo = parts[1];
            // Assume first segment after /tree/ is branch name (works for most cases)
            // Limitation: Won't work for branch names with slashes like "feature/auth"
            let branch = parts[3];
            let subpath = if parts.len() > 4 {
                parts[4..].join("/")
            } else {
                String::new()
            };

            let raw_base = if subpath.is_empty() {
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}")
            } else {
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{subpath}")
            };

            variations.push(format!("{raw_base}/README.md"));
        }
    }

    variations.push(format!("{base}.md"));
    if is_github {
        variations.push(format!("{base}/README.md"));
    }
    variations.push(format!("{base}/index.md"));
    variations.push(format!("{base}/llms.txt"));
    variations.push(format!("{base}/llms-full.txt"));

    variations
}

fn url_to_path(base_dir: &Path, url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parsed = url::Url::parse(url)?;
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
        Path::new(last_segment).extension().is_none()
    };

    if needs_index {
        path.push("index");
    }

    if let Some(query) = parsed.query() {
        // Security: Sanitize query parameters for filesystem safety
        let safe_query = query.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let current_ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let new_ext = if current_ext.is_empty() {
            format!("?{safe_query}")
        } else {
            format!("{current_ext}?{safe_query}")
        };
        path.set_extension(new_ext);
    }

    // Security: Verify final path is within base directory
    if !path.starts_with(base_dir) {
        return Err("Path traversal detected".into());
    }

    Ok(path)
}

async fn ensure_gitignore(base_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = base_dir.join(".gitignore");

    if !gitignore_path.exists() {
        fs::create_dir_all(base_dir).await?;
        fs::write(&gitignore_path, "*\n").await?;
    }

    Ok(())
}

fn remove_elements(document: &scraper::Html, selectors: &[&str]) -> String {
    let mut cleaned = document.html();

    for selector_str in selectors {
        if let Ok(selector) = scraper::Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let elem_html = element.html();
                cleaned = cleaned.replace(&elem_html, "");
            }
        }
    }

    cleaned
}

fn simplify_images(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut result = html.to_string();

    if let Ok(img_selector) = Selector::parse("img") {
        for img in document.select(&img_selector) {
            let img_html = img.html();

            let alt = img.value().attr("alt").unwrap_or("");
            let src = img.value().attr("src").unwrap_or("");

            let role = img.value().attr("role").unwrap_or("");

            let is_decorative =
                role == "presentation" || role == "none" || alt.is_empty() && src.contains("icon");

            let simple_img = if !is_decorative && !alt.is_empty() && !src.is_empty() {
                format!("![{alt}]({src})")
            } else if !is_decorative && !src.is_empty() {
                format!("![image]({src})")
            } else {
                String::new()
            };

            result = result.replace(&img_html, &simple_img);
        }
    }

    result
}

fn clean_html(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    let remove_selectors = &[
        "script",
        "style",
        "noscript",
        "iframe",
        "nav",
        "[role=banner]",
        "[role=navigation]",
        "[role=contentinfo]",
        "[role=complementary]",
        "[role=search]",
        "[aria-label*=navigation]",
        "[aria-label*=Navigation]",
        "[aria-label*=breadcrumb]",
        "[aria-label*=Breadcrumb]",
        "[aria-label*=search]",
        "[aria-label*=Search]",
        ".navigation",
        ".nav",
        ".navbar",
        ".nav-bar",
        ".site-header",
        ".site-footer",
        ".page-header",
        ".page-footer",
        ".breadcrumb",
        ".breadcrumbs",
        "#navigation",
        "#nav",
        "#navbar",
        "#breadcrumb",
        "#breadcrumbs",
    ];

    let cleaned_step1 = remove_elements(&document, remove_selectors);
    let cleaned_step2 = simplify_images(&cleaned_step1);
    let document2 = Html::parse_document(&cleaned_step2);

    let main_selectors = [
        ".markdown-body",
        "main",
        "[role=main]",
        ".main-content",
        "#main-content",
        "#content",
        ".content",
        ".docs-content",
        ".documentation",
        ".page-content",
    ];

    for main_sel in &main_selectors {
        if let Ok(selector) = Selector::parse(main_sel)
            && let Some(main_element) = document2.select(&selector).next()
        {
            return main_element.html();
        }
    }

    if let Ok(body_selector) = Selector::parse("body")
        && let Some(body) = document2.select(&body_selector).next()
    {
        return body.html();
    }

    cleaned_step2
}

fn clean_markdown(markdown: &str) -> String {
    static EMPTY_LINK_BRACKET: OnceLock<Regex> = OnceLock::new();
    static EMPTY_LINK: OnceLock<Regex> = OnceLock::new();
    static ZERO_WIDTH_CHARS: OnceLock<Regex> = OnceLock::new();
    static EXCESSIVE_NEWLINES: OnceLock<Regex> = OnceLock::new();

    let empty_link_bracket = EMPTY_LINK_BRACKET
        .get_or_init(|| Regex::new(r"\[\]\([^\)]*\)\[").expect("Invalid regex pattern"));
    let empty_link =
        EMPTY_LINK.get_or_init(|| Regex::new(r"\[\]\([^\)]*\)").expect("Invalid regex pattern"));
    let zero_width = ZERO_WIDTH_CHARS.get_or_init(|| {
        Regex::new(r"\[[\u{200B}\u{200C}\u{200D}\u{FEFF}]+\]").expect("Invalid regex pattern")
    });
    let excessive_newlines =
        EXCESSIVE_NEWLINES.get_or_init(|| Regex::new(r"\n{3,}").expect("Invalid regex pattern"));

    let mut result = markdown.to_string();
    result = empty_link_bracket.replace_all(&result, "[").to_string();
    result = empty_link.replace_all(&result, "").to_string();
    result = zero_width.replace_all(&result, "").to_string();
    result = excessive_newlines.replace_all(&result, "\n\n").to_string();

    result
}

fn count_stats(content: &str) -> (usize, usize, usize) {
    let lines = content.lines().count();
    let words = content.split_whitespace().count();
    let characters = content.chars().count();
    (lines, words, characters)
}

#[tool_router]
impl FetchServer {
    fn new(cache_dir: Option<PathBuf>) -> Self {
        let cache_path = cache_dir.unwrap_or_else(|| PathBuf::from(".llms-fetch-mcp"));
        // Ensure cache_dir is absolute for security (prevents relative path bypass)
        let absolute_cache = cache_path
            .canonicalize()
            .unwrap_or_else(|_| {
                // If path doesn't exist, make it absolute relative to current dir
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("/tmp"))
                    .join(&cache_path)
            });

        Self {
            cache_dir: Arc::new(absolute_cache),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Fetch web content and cache it locally with intelligent format detection. For best results, start with the root URL of a documentation site (e.g., https://docs.example.com) to discover llms.txt or llms-full.txt files, which provide LLM-optimized documentation structure. The tool automatically tries multiple format variations (.md, /README.md for GitHub, /index.md, /llms.txt, /llms-full.txt) concurrently. HTML is automatically cleaned and converted to Markdown. Returns cached file paths with content type and statistics."
    )]
    async fn fetch(
        &self,
        params: Parameters<FetchInput>,
    ) -> Result<rmcp::Json<FetchOutput>, McpError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                McpError::internal_error(format!("Failed to create HTTP client: {e}"), None)
            })?;

        let variations = get_url_variations(&params.0.url);

        let mut fetch_tasks = Vec::new();
        for url in &variations {
            let client_clone = client.clone();
            let url_clone = url.clone();
            fetch_tasks.push(tokio::spawn(async move {
                fetch_url(&client_clone, &url_clone).await
            }));
        }

        let mut results = Vec::new();
        let mut errors = Vec::new();
        for task in fetch_tasks {
            if let Ok(attempt) = task.await {
                match attempt {
                    FetchAttempt::Success(result) => results.push(result),
                    FetchAttempt::HttpError { url, status } => {
                        errors.push(format!("{url}: HTTP {status}"));
                    }
                    FetchAttempt::NetworkError { url } => {
                        errors.push(format!("{url}: network error"));
                    }
                }
            }
        }

        if results.is_empty() {
            let error_details = if errors.is_empty() {
                format!("tried {} variations", variations.len())
            } else {
                errors.join("; ")
            };
            return Err(McpError::resource_not_found(
                format!(
                    "Failed to fetch content from {} ({})",
                    params.0.url, error_details
                ),
                None,
            ));
        }

        ensure_gitignore(&self.cache_dir).await.map_err(|e| {
            McpError::internal_error(format!("Failed to create .gitignore: {e}"), None)
        })?;

        let mut file_infos = Vec::new();

        let has_non_html = results.iter().any(|r| !r.is_html);

        for result in results {
            let url_lower = result.url.to_lowercase();
            let content_type = if url_lower.contains("/llms-full.txt") {
                "llms-full"
            } else if url_lower.contains("/llms.txt") {
                "llms"
            } else if result.is_markdown {
                "markdown"
            } else if result.is_html {
                "html-converted"
            } else {
                "text"
            };

            if has_non_html && result.is_html {
                continue;
            }

            let content_to_save = if result.is_html && !result.is_markdown {
                let cleaned = clean_html(&result.content);
                let markdown = html2md::parse_html(&cleaned);
                clean_markdown(&markdown)
            } else {
                result.content.clone()
            };

            let file_path = url_to_path(&self.cache_dir, &result.url)
                .map_err(|e| McpError::internal_error(format!("Failed to parse URL: {e}"), None))?;

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    McpError::internal_error(format!("Failed to create directory: {e}"), None)
                })?;
            }

            // Atomic write: temp file + rename to prevent corruption from concurrent writes
            let temp_path = file_path.with_extension("tmp");
            fs::write(&temp_path, &content_to_save).await.map_err(|e| {
                McpError::internal_error(format!("Failed to write temp file: {e}"), None)
            })?;
            fs::rename(&temp_path, &file_path).await.map_err(|e| {
                McpError::internal_error(format!("Failed to finalize file: {e}"), None)
            })?;

            let (lines, words, characters) = count_stats(&content_to_save);
            file_infos.push(FileInfo {
                path: file_path.to_string_lossy().to_string(),
                source_url: result.url.clone(),
                content_type: content_type.to_string(),
                lines,
                words,
                characters,
            });
        }

        Ok(rmcp::Json(FetchOutput { files: file_infos }))
    }
}

#[tool_handler]
impl ServerHandler for FetchServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Web content fetcher with intelligent format detection for documentation. Tries multiple URL variations (.md, /index.md, /llms.txt, /llms-full.txt) concurrently. Cleans HTML and converts to Markdown. Deduplicates content automatically."
                    .to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cache_dir = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let server = FetchServer::new(cache_dir);

    let running = server
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    running.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_variations_plain_url() {
        let url = "https://example.com/docs";
        let variations = get_url_variations(url);

        assert_eq!(variations.len(), 5);
        assert_eq!(variations[0], "https://example.com/docs");
        assert_eq!(variations[1], "https://example.com/docs.md");
        assert_eq!(variations[2], "https://example.com/docs/index.md");
        assert_eq!(variations[3], "https://example.com/docs/llms.txt");
        assert_eq!(variations[4], "https://example.com/docs/llms-full.txt");
    }

    #[test]
    fn test_url_variations_github() {
        let url = "https://github.com/user/repo/tree/main/docs";
        let variations = get_url_variations(url);

        assert_eq!(variations.len(), 7);
        assert_eq!(variations[0], "https://github.com/user/repo/tree/main/docs");
        assert_eq!(
            variations[1],
            "https://raw.githubusercontent.com/user/repo/main/docs/README.md"
        );
        assert_eq!(
            variations[2],
            "https://github.com/user/repo/tree/main/docs.md"
        );
        assert_eq!(
            variations[3],
            "https://github.com/user/repo/tree/main/docs/README.md"
        );
        assert_eq!(
            variations[4],
            "https://github.com/user/repo/tree/main/docs/index.md"
        );
        assert_eq!(
            variations[5],
            "https://github.com/user/repo/tree/main/docs/llms.txt"
        );
        assert_eq!(
            variations[6],
            "https://github.com/user/repo/tree/main/docs/llms-full.txt"
        );
    }

    #[test]
    fn test_url_variations_md_file() {
        let url = "https://example.com/docs/readme.md";
        let variations = get_url_variations(url);

        assert_eq!(variations.len(), 1);
        assert_eq!(variations[0], "https://example.com/docs/readme.md");
    }

    #[test]
    fn test_url_variations_txt_file() {
        let url = "https://example.com/docs/file.txt";
        let variations = get_url_variations(url);

        assert_eq!(variations.len(), 1);
        assert_eq!(variations[0], "https://example.com/docs/file.txt");
    }

    #[test]
    fn test_url_variations_with_query_params() {
        let url = "https://httpbin.org/get?test=value";
        let variations = get_url_variations(url);

        // Should not add variations for URLs with query parameters
        assert_eq!(variations.len(), 1);
        assert_eq!(variations[0], "https://httpbin.org/get?test=value");
    }

    #[test]
    fn test_url_to_path_simple() {
        let base = PathBuf::from("/cache");
        let url = "https://example.com/docs/page";
        let path = url_to_path(&base, url).unwrap();

        assert_eq!(path, PathBuf::from("/cache/example.com/docs/page/index"));
    }

    #[test]
    fn test_url_to_path_with_extension() {
        let base = PathBuf::from("/cache");
        let url = "https://example.com/docs/page.md";
        let path = url_to_path(&base, url).unwrap();

        assert_eq!(path, PathBuf::from("/cache/example.com/docs/page.md"));
    }

    #[test]
    fn test_url_to_path_root() {
        let base = PathBuf::from("/cache");
        let url = "https://example.com/";
        let path = url_to_path(&base, url).unwrap();

        assert_eq!(path, PathBuf::from("/cache/example.com/index"));
    }

    #[test]
    fn test_count_stats() {
        let content = "Line 1\nLine 2\nLine 3";
        let (lines, words, chars) = count_stats(content);

        assert_eq!(lines, 3);
        assert_eq!(words, 6);
        assert_eq!(chars, 20);
    }

    #[test]
    fn test_count_stats_empty() {
        let content = "";
        let (lines, words, chars) = count_stats(content);

        assert_eq!(lines, 0);
        assert_eq!(words, 0);
        assert_eq!(chars, 0);
    }

    #[test]
    fn test_url_to_path_with_query_params() {
        let base = PathBuf::from(".llms-fetch-mcp");
        let url = "https://httpbin.org/get?test=value";
        let path = url_to_path(&base, url).unwrap();

        eprintln!("Base: {base:?}");
        eprintln!("Path: {path:?}");
        eprintln!("Starts with: {}", path.starts_with(&base));

        assert!(path.starts_with(&base));
        assert!(path.to_string_lossy().contains("?test=value"));
    }

    #[test]
    fn test_url_to_path_deep_path() {
        let base = PathBuf::from(".llms-fetch-mcp");
        let url = "https://developer.mozilla.org/en-US/docs/Web/JavaScript";
        let path = url_to_path(&base, url).unwrap();

        eprintln!("Base: {base:?}");
        eprintln!("Path: {path:?}");
        eprintln!("Starts with: {}", path.starts_with(&base));

        assert!(path.starts_with(&base));
    }

    #[test]
    fn test_url_parser_normalizes_traversal() {
        // The url::Url parser automatically normalizes path traversal attempts
        // This test verifies this behavior, which is good for security
        let base = PathBuf::from("/cache");
        let url = "https://example.com/../etc/passwd";

        let parsed = url::Url::parse(url).unwrap();
        eprintln!("URL: {url}");
        eprintln!("Parsed path: {}", parsed.path());

        // URL parser normalizes "../" to "/" at the root
        assert_eq!(parsed.path(), "/etc/passwd");

        // Our code will place this safely within the cache
        let result = url_to_path(&base, url);
        assert!(result.is_ok());
        let path = result.unwrap();
        // Path is within cache directory - safe
        assert!(path.starts_with(&base));
        assert_eq!(path, PathBuf::from("/cache/example.com/etc/passwd/index"));
    }

    #[test]
    fn test_component_filter_blocks_dots() {
        // If somehow a ".." or "." makes it through URL parsing as a component,
        // our component filter will reject it
        let base = PathBuf::from("/cache");

        // Manually construct a URL that would have ".." as a component
        // (in practice, url::Url normalizes these, but we test the filter anyway)
        let test_cases = vec![
            ("https://example.com/%2e%2e/passwd", "/passwd"), // URL-encoded ".."
        ];

        for (url, _expected_path) in test_cases {
            let parsed = url::Url::parse(url).unwrap();
            eprintln!("Testing URL: {url}");
            eprintln!("Parsed path: {}", parsed.path());

            let result = url_to_path(&base, url);
            eprintln!("Result: {result:?}");

            // Verify the path is safe and within base
            if let Ok(path) = result {
                assert!(path.starts_with(&base));
            }
        }
    }

    #[test]
    fn test_starts_with_protection() {
        // Final check: verify paths stay within base directory
        let base = PathBuf::from("/cache");
        let url = "https://example.com/docs/api/v1/reference";
        let result = url_to_path(&base, url);

        assert!(result.is_ok());
        let path = result.unwrap();

        // Path must be within base directory
        assert!(path.starts_with(&base));
        assert!(path.to_string_lossy().contains("docs/api/v1/reference"));

        // Verify the path structure
        assert_eq!(
            path,
            PathBuf::from("/cache/example.com/docs/api/v1/reference/index")
        );
    }

    #[test]
    fn test_url_variations_github_blob() {
        // Test that /blob/ URLs get converted to raw.githubusercontent.com
        // Note: Can't use .md extension as those return early (no variations)
        let url = "https://github.com/user/repo/blob/main/src/lib.rs";
        let variations = get_url_variations(url);

        // Should have: original + raw + .md + README.md + index.md + llms.txt + llms-full.txt = 7
        assert_eq!(variations.len(), 7);
        assert_eq!(variations[0], "https://github.com/user/repo/blob/main/src/lib.rs");
        assert_eq!(
            variations[1],
            "https://raw.githubusercontent.com/user/repo/main/src/lib.rs"
        );
        // Standard variations also added
        assert_eq!(variations[2], "https://github.com/user/repo/blob/main/src/lib.rs.md");
        assert_eq!(variations[3], "https://github.com/user/repo/blob/main/src/lib.rs/README.md");
    }

    #[test]
    fn test_url_variations_github_malformed() {
        // Test that malformed GitHub URLs don't panic
        let urls = vec![
            "https://github.com/user",           // Too few segments
            "https://github.com/user/repo",       // No tree/blob
            "https://github.com",                 // Root
        ];

        for url in urls {
            let variations = get_url_variations(url);
            // Should return standard variations without crashing
            assert!(!variations.is_empty());
            assert_eq!(variations[0], url);
        }
    }

    #[test]
    fn test_url_to_path_query_sanitization() {
        // Test that filesystem-unsafe characters in query params are sanitized
        let base = PathBuf::from("/cache");

        // Test that slashes in query params get sanitized
        let url1 = "https://example.com/api?path=../etc/passwd";
        let path1 = url_to_path(&base, url1).unwrap();
        let path_str1 = path1.to_string_lossy();
        assert!(path1.starts_with(&base));
        // Slashes in query should be replaced with underscores
        assert!(path_str1.contains("path=.._etc_passwd"), "Path was: {}", path_str1);

        // Test that other unsafe chars (colons, question marks, etc.) get sanitized
        let url2 = "https://example.com/api?name=file:name?test";
        let path2 = url_to_path(&base, url2).unwrap();
        let path_str2 = path2.to_string_lossy();
        assert!(path2.starts_with(&base));
        // Colons and question marks should be replaced with underscores
        assert!(path_str2.contains("file_name_test"), "Path was: {}", path_str2);

        // Test that backslashes in query params get sanitized
        let url3 = "https://example.com/api?path=..\\etc\\passwd";
        let path3 = url_to_path(&base, url3).unwrap();
        let path_str3 = path3.to_string_lossy();
        assert!(path3.starts_with(&base));
        // Backslashes should be replaced with underscores
        assert!(path_str3.contains("path=.._etc_passwd"), "Path was: {}", path_str3);
    }
}
