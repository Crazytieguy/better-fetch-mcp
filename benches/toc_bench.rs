use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

fn generate_markdown(num_sections: usize, heading_level: u8) -> String {
    let mut md = String::from("# Main Title\n\nIntroduction text.\n\n");

    for i in 1..=num_sections {
        md.push_str(&format!(
            "{} Section {}\n\nContent for section {}. This is some representative text that might appear in a real document. It includes multiple sentences to make it more realistic.\n\n",
            "#".repeat(heading_level as usize),
            i,
            i
        ));
    }

    md
}

fn generate_nested_markdown(sections_per_level: usize) -> String {
    let mut md = String::from("# Main Title\n\n");

    for i in 1..=sections_per_level {
        md.push_str(&format!("## Section {}\n\nLevel 2 content.\n\n", i));

        for j in 1..=3 {
            md.push_str(&format!(
                "### Subsection {}.{}\n\nLevel 3 content.\n\n",
                i, j
            ));

            for k in 1..=2 {
                md.push_str(&format!(
                    "#### Item {}.{}.{}\n\nLevel 4 content.\n\n",
                    i, j, k
                ));
            }
        }
    }

    md
}

fn generate_complex_markdown(num_sections: usize) -> String {
    let mut md = String::from("# API Documentation\n\nWelcome to the API docs.\n\n");

    for i in 1..=num_sections {
        md.push_str(&format!(
            "## Class `MyClass{}`\n\nDescription of MyClass{}.\n\n",
            i, i
        ));

        md.push_str(&format!(
            "### Constructor\n\nThe constructor for **MyClass{}**.\n\n",
            i
        ));

        md.push_str(&format!(
            "### Methods\n\n#### `method{}()`\n\nDoes something with [parameters](url).\n\n",
            i
        ));

        md.push_str(&format!(
            "#### `anotherMethod{}()`\n\nDoes ~~something~~ useful.\n\n",
            i
        ));
    }

    md
}

fn bench_small_document(c: &mut Criterion) {
    let md = generate_markdown(10, 2);
    let chars = md.len();

    c.bench_function("toc_small_10_sections", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
    });
}

fn bench_medium_document(c: &mut Criterion) {
    let md = generate_markdown(50, 2);
    let chars = md.len();

    c.bench_function("toc_medium_50_sections", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
    });
}

fn bench_large_document(c: &mut Criterion) {
    let md = generate_markdown(200, 2);
    let chars = md.len();

    c.bench_function("toc_large_200_sections", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
    });
}

fn bench_very_large_document(c: &mut Criterion) {
    let md = generate_markdown(1000, 2);
    let chars = md.len();

    c.bench_function("toc_very_large_1000_sections", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
    });
}

fn bench_nested_document(c: &mut Criterion) {
    let md = generate_nested_markdown(10);
    let chars = md.len();

    c.bench_function("toc_nested_hierarchy", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
    });
}

fn bench_complex_formatting(c: &mut Criterion) {
    let md = generate_complex_markdown(20);
    let chars = md.len();

    c.bench_function("toc_complex_formatting", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
    });
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("toc_scaling");

    for size in [10, 50, 100, 200, 500].iter() {
        let md = generate_markdown(*size, 2);
        let chars = md.len();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
        });
    }

    group.finish();
}

fn bench_heading_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("toc_heading_levels");

    for level in 2..=6 {
        let md = generate_markdown(50, level);
        let chars = md.len();

        group.bench_with_input(BenchmarkId::from_parameter(level), &level, |b, _| {
            b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(&md), black_box(chars)));
        });
    }

    group.finish();
}

fn bench_real_world_documents(c: &mut Criterion) {
    let mut group = c.benchmark_group("toc_real_world");

    // Simulate a typical README (small, mixed levels)
    let readme = r#"# Project Name

## Features

- Feature 1
- Feature 2

## Installation

### Prerequisites

Prerequisites here.

### Quick Start

Quick start guide.

## Usage

### Basic Example

Example code.

### Advanced Usage

Advanced examples.

## API Reference

### Class Methods

Methods here.

### Properties

Properties here.

## Contributing

Contributing guide.

## License

MIT License.
"#;
    group.bench_function("readme", |b| {
        b.iter(|| llms_fetch_mcp::toc::generate_toc(black_box(readme), black_box(readme.len())));
    });

    // Simulate API documentation (large, deeply nested)
    let api_docs = generate_complex_markdown(50);
    group.bench_function("api_docs", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(black_box(&api_docs), black_box(api_docs.len()))
        });
    });

    // Simulate tutorial (medium, simple hierarchy)
    let tutorial = generate_markdown(30, 2);
    group.bench_function("tutorial", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(black_box(&tutorial), black_box(tutorial.len()))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_small_document,
    bench_medium_document,
    bench_large_document,
    bench_very_large_document,
    bench_nested_document,
    bench_complex_formatting,
    bench_scaling,
    bench_heading_levels,
    bench_real_world_documents,
);
criterion_main!(benches);
