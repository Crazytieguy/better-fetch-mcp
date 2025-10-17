use dom_smoothie::{Config, Readability, TextMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_urls = vec![
        ("react.dev", "https://react.dev"),
        ("go.dev-doc", "https://go.dev/doc"),
        ("tailwindcss.com-docs", "https://tailwindcss.com/docs"),
        ("vuejs.org-guide", "https://vuejs.org/guide"),
        ("python.org-tutorial", "https://docs.python.org/3/tutorial/"),
        ("rust-lang.org-book", "https://doc.rust-lang.org/book/"),
        (
            "mdn-javascript",
            "https://developer.mozilla.org/en-US/docs/Web/JavaScript",
        ),
        ("nextjs.org-docs", "https://nextjs.org/docs"),
        (
            "github-readme",
            "https://github.com/anthropics/anthropic-sdk-python",
        ),
        (
            "stackoverflow",
            "https://stackoverflow.com/questions/1732348/regex-match-open-tags-except-xhtml-self-contained-tags",
        ),
        ("wikipedia", "https://en.wikipedia.org/wiki/Markdown"),
    ];

    std::fs::create_dir_all(".test-outputs")?;

    println!("Fetching test pages and saving to .test-outputs/...\n");

    for (name, url) in test_urls {
        println!("Testing: {} ({})", name, url);
        println!("{}", "=".repeat(80));

        let response = reqwest::blocking::get(url)?;
        let html = response.text()?;

        let cfg = Config {
            text_mode: TextMode::Markdown,
            ..Default::default()
        };

        let mut readability = Readability::new(html.as_str(), Some(url), Some(cfg))?;
        let article = readability.parse()?;
        let markdown = article.text_content.to_string();

        let output_path = format!(".test-outputs/{}.md", name);
        std::fs::write(&output_path, &markdown)?;

        let lines: Vec<&str> = markdown.lines().collect();
        let preview_lines = &lines[..lines.len().min(50)];

        println!("First 50 lines of converted markdown:");
        println!("{}", "-".repeat(80));
        for (i, line) in preview_lines.iter().enumerate() {
            println!("{:3} | {}", i + 1, line);
        }
        println!("\nTotal lines: {}", lines.len());
        println!("Saved to: {}", output_path);
        println!("\n\n");
    }

    println!("All outputs saved to .test-outputs/ directory");
    println!("You can review full files with: ls -lh .test-outputs/");

    Ok(())
}
