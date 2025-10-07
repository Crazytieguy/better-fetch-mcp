use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::{tool, tool_router, ErrorData as McpError, ServiceExt};
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
}

async fn fetch_url(client: &reqwest::Client, url: &str) -> Option<FetchResult> {
    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                let is_html = content_type.contains("text/html");

                match response.text().await {
                    Ok(content) => Some(FetchResult {
                        url: url.to_string(),
                        content,
                        is_html,
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

fn url_to_path(base_dir: &Path, url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
            format!("?{}", query)
        } else {
            format!("{}?{}", current_ext, query)
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

    #[tool(description = "Fetch content from a URL and cache it locally. If the URL ends with .md or .txt, only that URL is fetched. Otherwise, multiple variations are tried concurrently (.md, /index.md, /llms.txt, /llms-full.txt). Content is saved to .better-fetch-mcp/<domain>/<path>.")]
    async fn fetch(&self, params: Parameters<FetchInput>) -> Result<rmcp::Json<FetchOutput>, McpError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| McpError::internal_error(format!("Failed to create HTTP client: {}", e), None))?;

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
        for (i, task) in fetch_tasks.into_iter().enumerate() {
            if let Ok(Some(result)) = task.await {
                results.push((i, result));
            }
        }

        if results.is_empty() {
            return Err(McpError::invalid_request(
                format!("Failed to fetch {}", params.0.url),
                None,
            ));
        }

        ensure_gitignore(&self.cache_dir).await
            .map_err(|e| McpError::internal_error(format!("Failed to create .gitignore: {}", e), None))?;

        let only_original = results.len() == 1 && results[0].0 == 0;

        let mut file_infos = Vec::new();

        if only_original && results[0].1.is_html {
            let result = &results[0].1;
            let markdown = html2md::parse_html(&result.content);

            let file_path = url_to_path(&self.cache_dir, &result.url)
                .map_err(|e| McpError::internal_error(format!("Failed to parse URL: {}", e), None))?;

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| McpError::internal_error(format!("Failed to create directory: {}", e), None))?;
            }

            fs::write(&file_path, &markdown).await
                .map_err(|e| McpError::internal_error(format!("Failed to write file: {}", e), None))?;

            let (lines, words, characters) = count_stats(&markdown);
            file_infos.push(FileInfo {
                path: file_path.to_string_lossy().to_string(),
                lines,
                words,
                characters,
            });
        } else {
            for (_, result) in results {
                let file_path = url_to_path(&self.cache_dir, &result.url)
                    .map_err(|e| McpError::internal_error(format!("Failed to parse URL: {}", e), None))?;

                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).await
                        .map_err(|e| McpError::internal_error(format!("Failed to create directory: {}", e), None))?;
                }

                fs::write(&file_path, &result.content).await
                    .map_err(|e| McpError::internal_error(format!("Failed to write file: {}", e), None))?;

                let (lines, words, characters) = count_stats(&result.content);
                file_infos.push(FileInfo {
                    path: file_path.to_string_lossy().to_string(),
                    lines,
                    words,
                    characters,
                });
            }
        }

        Ok(rmcp::Json(FetchOutput { files: file_infos }))
    }
}

impl ServerHandler for FetchServer {}

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

        insta::assert_debug_snapshot!(variations, @r#"
        [
            "https://example.com/docs",
            "https://example.com/docs.md",
            "https://example.com/docs/index.md",
            "https://example.com/docs/llms.txt",
            "https://example.com/docs/llms-full.txt",
        ]
        "#);
    }

    #[test]
    fn test_url_variations_md_file() {
        let url = "https://example.com/docs/readme.md";
        let variations = get_url_variations(url);

        insta::assert_debug_snapshot!(variations, @r#"
        [
            "https://example.com/docs/readme.md",
        ]
        "#);
    }

    #[test]
    fn test_url_variations_txt_file() {
        let url = "https://example.com/docs/file.txt";
        let variations = get_url_variations(url);

        insta::assert_debug_snapshot!(variations, @r#"
        [
            "https://example.com/docs/file.txt",
        ]
        "#);
    }

    #[test]
    fn test_url_to_path_simple() {
        let base = PathBuf::from("/cache");
        let url = "https://example.com/docs/page";
        let path = url_to_path(&base, url).unwrap();

        assert_eq!(path, PathBuf::from("/cache/example.com/docs/page"));
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

        assert_eq!(path, PathBuf::from("/cache/example.com"));
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
