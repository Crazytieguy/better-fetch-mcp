use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const CHARS_PER_TOKEN: f64 = 4.0;
const TOC_BUDGET: usize = 1000;
const FULL_CONTENT_THRESHOLD: usize = 2000;

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub line_number: usize,
    pub text: String,
}

fn extract_headings(markdown: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut current_heading: Option<(usize, HeadingLevel)> = None;
    let mut text_buffer = String::new();
    let line_tracker = LineTracker::new(markdown);

    let parser = Parser::new_ext(markdown, Options::all()).into_offset_iter();

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let line_num = line_tracker.line_at_offset(range.start);
                current_heading = Some((line_num, level));
                text_buffer.clear();
            }
            Event::Text(text) if current_heading.is_some() => {
                text_buffer.push_str(&text);
            }
            Event::Code(code) if current_heading.is_some() => {
                text_buffer.push('`');
                text_buffer.push_str(&code);
                text_buffer.push('`');
            }
            Event::SoftBreak | Event::HardBreak if current_heading.is_some() => {
                text_buffer.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((line_num, heading_level)) = current_heading.take() {
                    let trimmed_text = text_buffer.trim();
                    if !trimmed_text.is_empty() {
                        let level_num = heading_level_to_u8(heading_level);
                        let heading_text =
                            format!("{} {}", "#".repeat(level_num as usize), trimmed_text);

                        headings.push(Heading {
                            level: level_num,
                            line_number: line_num,
                            text: heading_text,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    headings
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

struct LineTracker {
    line_offsets: Vec<usize>,
}

impl LineTracker {
    fn new(text: &str) -> Self {
        let mut offsets = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                offsets.push(i + 1);
            }
        }
        Self {
            line_offsets: offsets,
        }
    }

    fn line_at_offset(&self, offset: usize) -> usize {
        match self.line_offsets.binary_search(&offset) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn estimate_tokens(headings: &[Heading], max_level: u8) -> usize {
    let filtered: Vec<_> = headings.iter().filter(|h| h.level <= max_level).collect();

    if filtered.is_empty() {
        return 0;
    }

    let max_line_num = filtered.last().unwrap().line_number;
    let line_num_width = format!("{max_line_num}").len().max(3);

    let total_chars: usize = filtered
        .iter()
        .map(|h| {
            let arrow = 1;
            let heading_len = h.text.chars().count();
            let newline = 1;
            line_num_width + arrow + heading_len + newline
        })
        .sum();

    (total_chars as f64 / CHARS_PER_TOKEN).ceil() as usize
}

fn find_optimal_level(headings: &[Heading], budget: usize) -> Option<u8> {
    if headings.is_empty() {
        return None;
    }

    let max_level = headings.iter().map(|h| h.level).max().unwrap_or(1);

    let mut best = None;
    for level in 1..=max_level {
        let tokens = estimate_tokens(headings, level);
        if tokens <= budget {
            best = Some(level);
        } else {
            break;
        }
    }

    best
}

fn render_toc(headings: &[Heading], max_level: u8) -> String {
    let filtered: Vec<_> = headings.iter().filter(|h| h.level <= max_level).collect();

    if filtered.is_empty() {
        return String::new();
    }

    let max_line_num = filtered.last().unwrap().line_number;
    let width = format!("{max_line_num}").len().max(3);

    filtered
        .iter()
        .map(|h| format!("{:>width$}→{}", h.line_number, h.text, width = width))
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn generate_toc(markdown: &str, total_chars: usize) -> Option<String> {
    let estimated_tokens = (total_chars as f64 / CHARS_PER_TOKEN).ceil() as usize;
    if estimated_tokens < FULL_CONTENT_THRESHOLD {
        return None;
    }

    let headings = extract_headings(markdown);
    if headings.is_empty() {
        return None;
    }

    let optimal_level = find_optimal_level(&headings, TOC_BUDGET)?;

    let toc = render_toc(&headings, optimal_level);
    if toc.is_empty() { None } else { Some(toc) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_headings() {
        let md = "# H1\n## H2\n### H3";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].line_number, 1);
        assert_eq!(headings[0].text, "# H1");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "## H2");
    }

    #[test]
    fn test_ignore_fenced_code_blocks() {
        let md = "# Real\n```\n# Fake\n```\n## Also Real";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "# Real");
        assert_eq!(headings[1].text, "## Also Real");
    }

    #[test]
    fn test_ignore_indented_code_blocks() {
        let md = "# Real\n\n    # Not a heading (indented)\n\n## Real2";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "# Real");
        assert_eq!(headings[1].text, "## Real2");
    }

    #[test]
    fn test_setext_headings() {
        let md = "H1\n==\n\nH2\n--";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].level, 2);
    }

    #[test]
    fn test_escaped_headings() {
        let md = "# Real\n\\# Not a heading";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Real");
    }

    #[test]
    fn test_unicode_headings() {
        let md = "# 你好世界\n## 🎉 Emoji Heading";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert!(headings[0].text.contains("你好世界"));
        assert!(headings[1].text.contains("🎉"));
    }

    #[test]
    fn test_inline_code_in_headings() {
        let md = "# Use `cargo build`\n## Another with `code`";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert!(headings[0].text.contains("`cargo build`"));
    }

    #[test]
    fn test_level_selection() {
        let headings = vec![
            Heading {
                level: 1,
                line_number: 1,
                text: "# ".repeat(50),
            },
            Heading {
                level: 2,
                line_number: 2,
                text: "## ".repeat(50),
            },
            Heading {
                level: 3,
                line_number: 3,
                text: "### ".repeat(50),
            },
        ];

        let level = find_optimal_level(&headings, 100);
        assert!(level.is_some());
        assert!(level.unwrap() >= 1);
    }

    #[test]
    fn test_render_format() {
        let headings = vec![
            Heading {
                level: 1,
                line_number: 5,
                text: "# Title".to_string(),
            },
            Heading {
                level: 2,
                line_number: 123,
                text: "## Subtitle".to_string(),
            },
        ];
        let toc = render_toc(&headings, 2);
        assert!(toc.contains("  5→# Title"));
        assert!(toc.contains("123→## Subtitle"));
    }

    #[test]
    fn test_render_filters_by_level() {
        let headings = vec![
            Heading {
                level: 1,
                line_number: 1,
                text: "# H1".to_string(),
            },
            Heading {
                level: 2,
                line_number: 2,
                text: "## H2".to_string(),
            },
            Heading {
                level: 3,
                line_number: 3,
                text: "### H3".to_string(),
            },
        ];
        let toc = render_toc(&headings, 2);
        assert!(toc.contains("# H1"));
        assert!(toc.contains("## H2"));
        assert!(!toc.contains("### H3"));
    }

    #[test]
    fn test_empty_headings() {
        let headings: Vec<Heading> = vec![];
        let toc = render_toc(&headings, 3);
        assert_eq!(toc, "");
    }

    #[test]
    fn test_generate_toc_skips_small_docs() {
        let small_md = "# Title\nSome content.";
        let toc = generate_toc(small_md, small_md.len());
        assert!(toc.is_none());
    }

    #[test]
    fn test_generate_toc_returns_some_for_large_docs() {
        let large_md = format!("# Title\n{}\n## Section", "content\n".repeat(1000));
        let toc = generate_toc(&large_md, large_md.len());
        assert!(toc.is_some());
    }

    #[test]
    fn test_empty_heading_text_filtered() {
        let md = "# Real Heading\n#\n## Another Real";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "# Real Heading");
        assert_eq!(headings[1].text, "## Another Real");
    }

    #[test]
    fn test_budget_pressure_returns_none() {
        let headings = vec![
            Heading {
                level: 1,
                line_number: 1,
                text: "# ".to_string() + &"x".repeat(10000),
            },
            Heading {
                level: 1,
                line_number: 2,
                text: "# ".to_string() + &"x".repeat(10000),
            },
        ];

        let level = find_optimal_level(&headings, 10);
        assert!(level.is_none());
    }

    #[test]
    fn test_generate_toc_handles_budget_exceeded() {
        let md = format!(
            "{}# Very Long Heading {}\n{}",
            "content\n".repeat(1000),
            "x".repeat(10000),
            "more\n".repeat(1000)
        );
        let toc = generate_toc(&md, md.len());
        assert!(toc.is_none());
    }

    #[test]
    fn test_width_calculation_consistency() {
        let headings = vec![
            Heading {
                level: 1,
                line_number: 1,
                text: "# Title".to_string(),
            },
            Heading {
                level: 1,
                line_number: 100000,
                text: "# Large Line Number".to_string(),
            },
        ];

        let tokens = estimate_tokens(&headings, 1);
        let rendered = render_toc(&headings, 1);
        let actual_chars = rendered.chars().count();
        let estimated_chars = (tokens as f64 * CHARS_PER_TOKEN) as usize;

        assert!(
            (actual_chars as i32 - estimated_chars as i32).abs() < 50,
            "Estimate should be close to actual: estimated {} chars, actual {} chars",
            estimated_chars,
            actual_chars
        );
    }

    #[test]
    fn test_heading_with_bold_text() {
        let md = "# Heading with **bold** text";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Heading with bold text");
    }

    #[test]
    fn test_heading_with_italic_text() {
        let md = "# Heading with *italic* text";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Heading with italic text");
    }

    #[test]
    fn test_heading_with_bold_italic() {
        let md = "# Heading with ***bold italic***";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Heading with bold italic");
    }

    #[test]
    fn test_heading_with_link() {
        let md = "# Heading with [link text](url)";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Heading with link text");
    }

    #[test]
    fn test_heading_with_strikethrough() {
        let md = "# Heading with ~~strikethrough~~";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Heading with strikethrough");
    }

    #[test]
    fn test_heading_with_image() {
        let md = "# Heading with ![image](url)";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Heading with image");
    }

    #[test]
    fn test_heading_with_mixed_inline_elements() {
        let md = "# Mix of `code` and **bold** and [link](url)";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Mix of `code` and bold and link");
    }

    #[test]
    fn test_heading_full_link() {
        let md = "# [Full link heading](url)";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Full link heading");
    }

    #[test]
    fn test_heading_multiple_formatted_parts() {
        let md = "# Multiple **bold** and *italic* parts";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "# Multiple bold and italic parts");
    }

    // Snapshot tests
    mod snapshots {
        use super::*;

        #[test]
        fn snapshot_simple_toc() {
            let md = "# Introduction\n\nSome content here.\n\n## Getting Started\n\nMore content.\n\n### Installation\n\nInstall instructions.\n\n### Configuration\n\nConfig details.\n\n## Advanced Usage\n\nAdvanced stuff.";
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_complex_headings() {
            let md = r#"# API Reference

## Methods

### `Array.prototype.map()`

Map implementation.

### `Array.prototype.filter()`

Filter implementation.

## Classes

### `Promise<T>`

Promise API.

#### Constructor

Promise constructor.

#### Methods

Promise methods.

## Types

### `Awaited<Type>`

Awaited type.
"#;
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_inline_formatting() {
            let md = r#"# User Guide

## Using **bold** and *italic*

Content here.

## Working with [links](https://example.com)

More content.

## Running `cargo build`

Build instructions.

## ~~Deprecated~~ Features

Old features.
"#;
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_large_document() {
            let sections = (1..=20)
                .map(|i| format!("## Section {}\n\nContent for section {}.\n\n", i, i))
                .collect::<String>();
            let md = format!("# Large Document\n\n{}", sections);
            let toc = generate_toc(&md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_deeply_nested() {
            let md = r#"# Main

## Level 2

### Level 3

#### Level 4

##### Level 5

###### Level 6

Content here.

## Another Section

### Nested

#### More Nested

Content.
"#;
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_real_world_readme() {
            let md = r#"# Project Name

![Build Status](https://example.com/badge.svg)

## Features

- Feature 1
- Feature 2

## Installation

### Prerequisites

You need Node.js installed.

### Quick Start

Run `npm install`.

## Usage

### Basic Example

```javascript
const lib = require('lib');
lib.doSomething();
```

### Advanced Configuration

Edit the config file.

## API Reference

### `doSomething(options)`

Does something cool.

### `doSomethingElse()`

Does something else.

## Contributing

See CONTRIBUTING.md.

## License

MIT
"#;
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_unicode_content() {
            let md = r#"# 开始使用

## 安装 Installation

安装说明。

## 配置 Configuration

配置详情。

## 🎉 新功能

### ✨ Feature 1

Details.

### 🚀 Feature 2

More details.
"#;
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_real_world_svelte_llms() {
            let md = include_str!("../test-fixtures/svelte-llms.txt");
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_real_world_astro_excerpt() {
            let md = include_str!("../test-fixtures/astro-excerpt.txt");
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_real_world_convex_excerpt() {
            let md = include_str!("../test-fixtures/convex-excerpt.txt");
            let toc = generate_toc(md, md.len());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }
    }

    // Regular unit tests for edge cases (not snapshots)
    mod large_files {
        use super::*;

        #[test]
        fn test_astro_llms_full_exceeds_budget() {
            // Full Astro docs: 2.4MB, 424+ H1 headings
            // Even H1-only would exceed 1000 token budget
            let md = include_str!("../test-fixtures/astro-llms-full.txt");
            let toc = generate_toc(md, md.len());
            assert!(
                toc.is_none(),
                "Should not generate ToC when even H1s exceed budget"
            );
        }

        #[test]
        fn test_convex_llms_full_exceeds_budget() {
            // Full Convex docs: 1.8MB, 296+ H1 headings
            let md = include_str!("../test-fixtures/convex-llms-full.txt");
            let toc = generate_toc(md, md.len());
            assert!(
                toc.is_none(),
                "Should not generate ToC when even H1s exceed budget"
            );
        }
    }
}
