#![warn(clippy::pedantic)]

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

async fn fetch_url(client: &reqwest::Client, url: &str) -> Option<FetchResult> {
    match client
        .get(url)
        .header(
            "Accept",
            "text/markdown, text/x-markdown, text/plain, text/html;q=0.5, */*;q=0.1",
        )
        .send()
        .await
    {
        Ok(response) => {
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
                    Ok(content) => Some(FetchResult {
                        url: url.to_string(),
                        content,
                        is_html,
                        is_markdown,
                    }),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
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

fn url_to_path(base_dir: &Path, url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parsed = url::Url::parse(url)?;
    let domain = parsed.host_str().ok_or("No host in URL")?;

    let mut path = base_dir.join(domain);

    let url_path = parsed.path().trim_start_matches('/');

    let needs_index = if url_path.is_empty() {
        true
    } else {
        let last_segment = url_path.split('/').last().unwrap_or("");
        Path::new(last_segment).extension().is_none()
    };

    if !url_path.is_empty() {
        path.push(url_path);
    }

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

            let simple_img = if !alt.is_empty() && !src.is_empty() {
                format!("![{}]({})", alt, src)
            } else if !src.is_empty() {
                format!("![image]({})", src)
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
        "header[role=banner]",
        "footer[role=contentinfo]",
        ".navigation",
        ".nav",
        "#navigation",
        "#nav",
        "[aria-label*=navigation]",
        "[aria-label*=Navigation]",
    ];

    let cleaned_step1 = remove_elements(&document, remove_selectors);
    let cleaned_step2 = simplify_images(&cleaned_step1);
    let document2 = Html::parse_document(&cleaned_step2);

    let main_selectors = [
        "main",
        "article",
        "[role=main]",
        ".main-content",
        "#main-content",
        "#content",
        ".content",
    ];

    for main_sel in &main_selectors {
        if let Ok(selector) = Selector::parse(main_sel) {
            if let Some(main_element) = document2.select(&selector).next() {
                return main_element.html();
            }
        }
    }

    if let Ok(body_selector) = Selector::parse("body") {
        if let Some(body) = document2.select(&body_selector).next() {
            return body.html();
        }
    }

    cleaned_step2
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
        Self {
            cache_dir: Arc::new(cache_dir.unwrap_or_else(|| PathBuf::from(".better-fetch-mcp"))),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Fetch web content and cache it locally with intelligent format detection. Tries documentation-friendly formats (.md, /index.md, /llms.txt, /llms-full.txt) concurrently. HTML is automatically cleaned and converted to Markdown. Returns file paths with content type and statistics.")]
    async fn fetch(&self, params: Parameters<FetchInput>) -> Result<rmcp::Json<FetchOutput>, McpError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| McpError::internal_error(format!("Failed to create HTTP client: {e}"), None))?;

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
        for task in fetch_tasks {
            if let Ok(Some(result)) = task.await {
                results.push(result);
            }
        }

        if results.is_empty() {
            return Err(McpError::invalid_request(
                format!("Failed to fetch {}", params.0.url),
                None,
            ));
        }

        ensure_gitignore(&self.cache_dir).await
            .map_err(|e| McpError::internal_error(format!("Failed to create .gitignore: {e}"), None))?;

        let mut file_infos = Vec::new();

        let has_llm_friendly = results.iter().any(|r| {
            let url_lower = r.url.to_lowercase();
            url_lower.contains("/llms.txt") || url_lower.contains("/llms-full.txt") ||
            r.is_markdown || url_lower.ends_with(".md")
        });

        for result in results {
            let url_lower = result.url.to_lowercase();
            let content_type = if url_lower.contains("/llms-full.txt") {
                "llms-full"
            } else if url_lower.contains("/llms.txt") {
                "llms"
            } else if result.is_markdown || url_lower.ends_with(".md") {
                "markdown"
            } else if result.is_html {
                "html-converted"
            } else {
                "text"
            };

            if has_llm_friendly && result.is_html && !result.is_markdown {
                continue;
            }

            let content_to_save = if result.is_html && !result.is_markdown {
                let cleaned = clean_html(&result.content);
                html2md::parse_html(&cleaned)
            } else {
                result.content.clone()
            };

            let file_path = url_to_path(&self.cache_dir, &result.url)
                .map_err(|e| McpError::internal_error(format!("Failed to parse URL: {e}"), None))?;

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| McpError::internal_error(format!("Failed to create directory: {e}"), None))?;
            }

            fs::write(&file_path, &content_to_save).await
                .map_err(|e| McpError::internal_error(format!("Failed to write file: {e}"), None))?;

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
}
