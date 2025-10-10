use scraper::{Html, Selector, ElementRef, Node};
use scraper::node::Element;

fn remove_elements(document: &Html, selectors: &[&str]) -> String {
    use scraper::html::Html as InnerHtml;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_urls = vec![
        "https://react.dev",
        "https://go.dev/doc",
        "https://tailwindcss.com/docs",
        "https://vuejs.org/guide",
    ];

    println!("Fetching test pages...\n");

    for url in test_urls {
        println!("Testing: {}", url);
        println!("{}", "=".repeat(80));

        let response = reqwest::blocking::get(url)?;
        let html = response.text()?;

        let cleaned = clean_html(&html);
        let markdown = html2md::parse_html(&cleaned);

        let lines: Vec<&str> = markdown.lines().collect();
        let preview_lines = &lines[..lines.len().min(50)];

        println!("First 50 lines of converted markdown:");
        println!("{}", "-".repeat(80));
        for (i, line) in preview_lines.iter().enumerate() {
            println!("{:3} | {}", i + 1, line);
        }
        println!("\nTotal lines: {}", lines.len());
        println!("\n\n");
    }

    Ok(())
}
