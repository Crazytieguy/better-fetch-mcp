use scraper::{Html, Selector};

fn remove_elements(document: &Html, selectors: &[&str]) -> String {
    let mut cleaned = document.html();

    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let elem_html = element.html();
                cleaned = cleaned.replace(&elem_html, "");
            }
        }
    }

    cleaned
}

fn simplify_images(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut result = html.to_string();

    if let Ok(img_selector) = Selector::parse("img") {
        for img in document.select(&img_selector) {
            let img_html = img.html();

            let alt = img.value().attr("alt").unwrap_or("");
            let src = img.value().attr("src").unwrap_or("");

            let role = img.value().attr("role").unwrap_or("");

            let is_decorative = role == "presentation"
                || role == "none"
                || alt.is_empty() && src.contains("icon");

            let simple_img = if !is_decorative && !alt.is_empty() && !src.is_empty() {
                format!("![{}]({})", alt, src)
            } else if !is_decorative && !src.is_empty() {
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
        "main",
        "article",
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

fn clean_markdown(markdown: &str) -> String {
    let mut result = markdown.to_string();

    result = regex::Regex::new(r"\[\]\([^\)]*\)\[")
        .unwrap()
        .replace_all(&result, "[")
        .to_string();

    result = regex::Regex::new(r"\[\]\([^\)]*\)")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    result = regex::Regex::new(r"\[[\u{200B}\u{200C}\u{200D}\u{FEFF}]+\]")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    result = regex::Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&result, "\n\n")
        .to_string();

    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_urls = vec![
        ("react.dev", "https://react.dev"),
        ("go.dev-doc", "https://go.dev/doc"),
        ("tailwindcss.com-docs", "https://tailwindcss.com/docs"),
        ("vuejs.org-guide", "https://vuejs.org/guide"),
        ("python.org-tutorial", "https://docs.python.org/3/tutorial/"),
        ("rust-lang.org-book", "https://doc.rust-lang.org/book/"),
        ("mdn-javascript", "https://developer.mozilla.org/en-US/docs/Web/JavaScript"),
        ("nextjs.org-docs", "https://nextjs.org/docs"),
        ("github-readme", "https://github.com/anthropics/anthropic-sdk-python"),
        ("stackoverflow", "https://stackoverflow.com/questions/1732348/regex-match-open-tags-except-xhtml-self-contained-tags"),
        ("wikipedia", "https://en.wikipedia.org/wiki/Markdown"),
    ];

    std::fs::create_dir_all(".test-outputs")?;

    println!("Fetching test pages and saving to .test-outputs/...\n");

    for (name, url) in test_urls {
        println!("Testing: {} ({})", name, url);
        println!("{}", "=".repeat(80));

        let response = reqwest::blocking::get(url)?;
        let html = response.text()?;

        let cleaned = clean_html(&html);
        let markdown = html2md::parse_html(&cleaned);
        let markdown = clean_markdown(&markdown);

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
