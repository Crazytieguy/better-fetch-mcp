// Table of Contents generation for markdown documents.
//
// This module extracts headings from markdown and generates compact table of contents
// summaries for navigation. Headings are preserved exactly as they appear in the source,
// including all markdown syntax (links, formatting, trailing hashes, etc).
//
// Design philosophy: Preserve exact source content rather than reconstructing cleaned text.
// This maintains fidelity to the original document and avoids complex event handling.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Configuration for `ToC` generation.
/// Budget and threshold are in bytes (not tokens).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TocConfig {
    pub toc_budget: usize,
    pub full_content_threshold: usize,
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            toc_budget: 4000,
            full_content_threshold: 8000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub line_number: usize,
    pub text: String,
}

/// Extracts heading information from markdown.
///
/// Uses pulldown-cmark events and their byte offsets to extract heading text,
/// automatically excluding trailing links (which are typically anchor links).
///
/// Streams through events with a state machine:
/// - Track when we enter/exit a heading
/// - Record the byte offset of the first link within each heading
/// - Extract text from heading start to link start (or heading end if no link)
fn extract_headings(markdown: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    // Track current heading: (level, start_byte, first_link_byte)
    let mut current_heading: Option<(HeadingLevel, usize, Option<usize>)> = None;

    for (event, range) in Parser::new_ext(markdown, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_heading = Some((level, range.start, None));
            }
            Event::Start(Tag::Link { .. }) => {
                // Record first link position if we're inside a heading
                if let Some((_, _, link_pos)) = &mut current_heading
                    && link_pos.is_none()
                {
                    *link_pos = Some(range.start);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, start, link_at)) = current_heading.take() {
                    // Extract text: heading start to link start (or heading end if no link)
                    let content_end = link_at.unwrap_or(range.end);
                    let text = markdown.get(start..content_end).unwrap_or("").trim();

                    if !text.is_empty() {
                        // Calculate line number by counting newlines before heading
                        let line_number =
                            markdown[..start].chars().filter(|&c| c == '\n').count() + 1;

                        let level_num = match level {
                            HeadingLevel::H1 => 1,
                            HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3,
                            HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5,
                            HeadingLevel::H6 => 6,
                        };

                        headings.push(Heading {
                            level: level_num,
                            line_number,
                            text: text.to_string(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    headings
}

fn find_optimal_level(headings: &[Heading], budget: usize) -> Option<u8> {
    if headings.is_empty() {
        return None;
    }

    let max_level = headings.iter().map(|h| h.level).max().unwrap_or(1);

    let mut best = None;
    for level in 1..=max_level {
        let rendered = render_toc(headings, level);
        if rendered.is_empty() {
            continue; // Skip levels with no headings
        }

        let byte_size = rendered.len();
        if byte_size <= budget {
            best = Some(level);
        } else {
            break; // Stop when budget exceeded
        }
    }

    best
}

fn render_toc(headings: &[Heading], max_level: u8) -> String {
    let filtered: Vec<_> = headings.iter().filter(|h| h.level <= max_level).collect();

    if filtered.is_empty() {
        return String::new();
    }

    debug_assert!(!filtered.is_empty());
    let max_line_num = filtered.last().unwrap().line_number;
    let width = format!("{max_line_num}").len().max(3);

    filtered
        .iter()
        .map(|h| format!("{:>width$}→{}", h.line_number, h.text, width = width))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn generate_toc(markdown: &str, total_chars: usize, config: &TocConfig) -> Option<String> {
    if total_chars < config.full_content_threshold {
        return None;
    }

    let headings = extract_headings(markdown);
    if headings.is_empty() {
        return None;
    }

    let optimal_level = find_optimal_level(&headings, config.toc_budget)?;

    let toc = render_toc(&headings, optimal_level);
    if toc.is_empty() { None } else { Some(toc) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> TocConfig {
        TocConfig::default()
    }

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
    fn test_trailing_links_excluded() {
        // Trailing anchor links should be excluded
        let md = "## Writing markup with JSX [](#writing-markup-with-jsx)";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "## Writing markup with JSX");

        // Multiple trailing links - stops at first one
        let md2 = "### Title [](#anchor1) [more](#anchor2)";
        let headings2 = extract_headings(md2);
        assert_eq!(headings2.len(), 1);
        assert_eq!(headings2[0].text, "### Title");

        // No link - full text preserved
        let md3 = "# Simple Heading";
        let headings3 = extract_headings(md3);
        assert_eq!(headings3.len(), 1);
        assert_eq!(headings3[0].text, "# Simple Heading");

        // Link with text in middle - still excluded (stops at first link)
        let md4 = "## Title [link](url) more text";
        let headings4 = extract_headings(md4);
        assert_eq!(headings4.len(), 1);
        assert_eq!(headings4[0].text, "## Title");
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

        let level = find_optimal_level(&headings, 400);
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
        let toc = generate_toc(small_md, small_md.len(), &default_config());
        assert!(toc.is_none());
    }

    #[test]
    fn test_generate_toc_returns_some_for_large_docs() {
        let large_md = format!("# Title\n{}\n## Section", "content\n".repeat(1000));
        let toc = generate_toc(&large_md, large_md.len(), &default_config());
        assert!(toc.is_some());
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
        let toc = generate_toc(&md, md.len(), &default_config());
        assert!(toc.is_none());
    }

    #[test]
    fn test_simple_toc_behavior() {
        // Small doc should return None (< 2000 tokens)
        let md = "# Introduction\n\nSome content here.\n\n## Getting Started\n\nMore content.\n\n### Installation\n\nInstall instructions.\n\n### Configuration\n\nConfig details.\n\n## Advanced Usage\n\nAdvanced stuff.";
        let toc = generate_toc(md, md.len(), &default_config());
        assert!(toc.is_none(), "Small documents should not generate ToC");
    }

    #[test]
    fn test_deeply_nested_levels() {
        // Verify all 6 heading levels are recognized
        let md = r#"# Main

## Level 2

### Level 3

#### Level 4

##### Level 5

###### Level 6
"#;
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 6);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[3].level, 4);
        assert_eq!(headings[4].level, 5);
        assert_eq!(headings[5].level, 6);
    }

    // Snapshot tests with real-world documentation
    mod snapshots {
        use super::*;

        #[test]
        fn snapshot_astro_excerpt() {
            let md = include_str!("../test-fixtures/astro-excerpt.txt");
            let toc = generate_toc(md, md.len(), &default_config());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_convex_excerpt() {
            let md = include_str!("../test-fixtures/convex-excerpt.txt");
            let toc = generate_toc(md, md.len(), &default_config());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_react_learn() {
            let md = include_str!("../test-fixtures/react-learn.txt");
            let toc = generate_toc(md, md.len(), &default_config());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_vue_intro() {
            let md = include_str!("../test-fixtures/vue-intro.txt");
            let toc = generate_toc(md, md.len(), &default_config());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_python_tutorial() {
            let md = include_str!("../test-fixtures/python-tutorial.txt");
            let toc = generate_toc(md, md.len(), &default_config());
            insta::assert_snapshot!(toc.unwrap_or_default());
        }
    }

    mod config_snapshots {
        use super::*;

        #[test]
        fn snapshot_small_budget_react() {
            // With a small budget (1500 bytes), should only include H1s
            let md = include_str!("../test-fixtures/react-learn.txt");
            let config = TocConfig {
                toc_budget: 1500,
                full_content_threshold: 8000,
            };
            let toc = generate_toc(md, md.len(), &config);
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_large_budget_react() {
            // With a large budget (10000 bytes), should include deeper levels
            let md = include_str!("../test-fixtures/react-learn.txt");
            let config = TocConfig {
                toc_budget: 10000,
                full_content_threshold: 8000,
            };
            let toc = generate_toc(md, md.len(), &config);
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_low_threshold_small_doc() {
            // With a low threshold (2000 bytes), should generate ToC for smaller docs
            let md = include_str!("../test-fixtures/convex-excerpt.txt");
            let config = TocConfig {
                toc_budget: 4000,
                full_content_threshold: 2000,
            };
            let toc = generate_toc(md, md.len(), &config);
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_astro_full_large_budget() {
            // With a very large budget (50000 bytes), should generate H1-only ToC for astro-llms-full
            let md = include_str!("../test-fixtures/astro-llms-full.txt");
            let config = TocConfig {
                toc_budget: 50000,
                full_content_threshold: 8000,
            };
            let toc = generate_toc(md, md.len(), &config);
            insta::assert_snapshot!(toc.unwrap_or_default());
        }

        #[test]
        fn snapshot_convex_full_large_budget() {
            // With a very large budget (50000 bytes), should generate H1-only ToC for convex-llms-full
            let md = include_str!("../test-fixtures/convex-llms-full.txt");
            let config = TocConfig {
                toc_budget: 50000,
                full_content_threshold: 8000,
            };
            let toc = generate_toc(md, md.len(), &config);
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
            let toc = generate_toc(md, md.len(), &default_config());
            assert!(
                toc.is_none(),
                "Should not generate ToC when even H1s exceed budget"
            );
        }

        #[test]
        fn test_convex_llms_full_exceeds_budget() {
            // Full Convex docs: 1.8MB, 296+ H1 headings
            let md = include_str!("../test-fixtures/convex-llms-full.txt");
            let toc = generate_toc(md, md.len(), &default_config());
            assert!(
                toc.is_none(),
                "Should not generate ToC when even H1s exceed budget"
            );
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_custom_budget_allows_more_headings() {
            let md = include_str!("../test-fixtures/python-tutorial.txt");

            let small_budget = TocConfig {
                toc_budget: 500,
                full_content_threshold: 2000,
            };
            let large_budget = TocConfig {
                toc_budget: 10000,
                full_content_threshold: 2000,
            };

            let toc_small = generate_toc(md, md.len(), &small_budget);
            let toc_large = generate_toc(md, md.len(), &large_budget);

            assert!(toc_small.is_some());
            assert!(toc_large.is_some());

            let small_len = toc_small.unwrap().len();
            let large_len = toc_large.unwrap().len();
            assert!(
                large_len >= small_len,
                "Larger budget should allow same or more headings"
            );
        }

        #[test]
        fn test_higher_threshold_skips_more_docs() {
            let md = include_str!("../test-fixtures/vue-intro.txt");

            let low_threshold = TocConfig {
                toc_budget: 1000,
                full_content_threshold: 1000,
            };
            let high_threshold = TocConfig {
                toc_budget: 1000,
                full_content_threshold: 100000,
            };

            let toc_low = generate_toc(md, md.len(), &low_threshold);
            let toc_high = generate_toc(md, md.len(), &high_threshold);

            assert!(toc_low.is_some(), "Low threshold should generate ToC");
            assert!(toc_high.is_none(), "High threshold should skip ToC");
        }

        #[test]
        fn test_zero_threshold_always_generates() {
            let small_md = "# Title\nContent.";

            let config = TocConfig {
                toc_budget: 1000,
                full_content_threshold: 0,
            };

            let toc = generate_toc(small_md, small_md.len(), &config);
            assert!(toc.is_some(), "Zero threshold should always generate ToC");
        }

        #[test]
        fn test_tiny_budget_returns_none() {
            let md = include_str!("../test-fixtures/react-learn.txt");

            let tiny_budget = TocConfig {
                toc_budget: 10,
                full_content_threshold: 2000,
            };

            let toc = generate_toc(md, md.len(), &tiny_budget);
            assert!(
                toc.is_none(),
                "Budget too small for even H1s should return None"
            );
        }

        #[test]
        fn test_config_default_values() {
            let config = TocConfig::default();
            assert_eq!(config.toc_budget, 4000);
            assert_eq!(config.full_content_threshold, 8000);
        }
    }
}
