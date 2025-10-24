use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use llms_fetch_mcp::toc::TocConfig;
use std::hint::black_box;

const REACT_LEARN: &str = include_str!("../test-fixtures/react-learn.txt");
const VUE_INTRO: &str = include_str!("../test-fixtures/vue-intro.txt");
const PYTHON_TUTORIAL: &str = include_str!("../test-fixtures/python-tutorial.txt");
const ASTRO_EXCERPT: &str = include_str!("../test-fixtures/astro-excerpt.txt");
const CONVEX_EXCERPT: &str = include_str!("../test-fixtures/convex-excerpt.txt");
const ASTRO_FULL: &str = include_str!("../test-fixtures/astro-llms-full.txt");
const CONVEX_FULL: &str = include_str!("../test-fixtures/convex-llms-full.txt");

fn bench_vue_intro(c: &mut Criterion) {
    c.bench_function("toc_vue_intro", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(VUE_INTRO),
                black_box(VUE_INTRO.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_react_learn(c: &mut Criterion) {
    c.bench_function("toc_react_learn", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(REACT_LEARN),
                black_box(REACT_LEARN.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_python_tutorial(c: &mut Criterion) {
    c.bench_function("toc_python_tutorial", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(PYTHON_TUTORIAL),
                black_box(PYTHON_TUTORIAL.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_astro_excerpt(c: &mut Criterion) {
    c.bench_function("toc_astro_excerpt", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(ASTRO_EXCERPT),
                black_box(ASTRO_EXCERPT.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_convex_excerpt(c: &mut Criterion) {
    c.bench_function("toc_convex_excerpt", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(CONVEX_EXCERPT),
                black_box(CONVEX_EXCERPT.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_astro_full(c: &mut Criterion) {
    c.bench_function("toc_astro_full", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(ASTRO_FULL),
                black_box(ASTRO_FULL.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_convex_full(c: &mut Criterion) {
    c.bench_function("toc_convex_full", |b| {
        b.iter(|| {
            llms_fetch_mcp::toc::generate_toc(
                black_box(CONVEX_FULL),
                black_box(CONVEX_FULL.len()),
                &TocConfig::default(),
            )
        });
    });
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("toc_scaling");

    let docs = [
        ("vue_intro", VUE_INTRO, 10),
        ("react_learn", REACT_LEARN, 12),
        ("astro_excerpt", ASTRO_EXCERPT, 14),
        ("python_tutorial", PYTHON_TUTORIAL, 5),
        ("convex_excerpt", CONVEX_EXCERPT, 17),
    ];

    for (name, md, heading_count) in docs {
        group.throughput(Throughput::Elements(heading_count));
        group.bench_with_input(BenchmarkId::new("doc", name), &md, |b, md| {
            b.iter(|| {
                llms_fetch_mcp::toc::generate_toc(
                    black_box(md),
                    black_box(md.len()),
                    &TocConfig::default(),
                )
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vue_intro,
    bench_react_learn,
    bench_python_tutorial,
    bench_astro_excerpt,
    bench_convex_excerpt,
    bench_astro_full,
    bench_convex_full,
    bench_scaling,
);
criterion_main!(benches);
