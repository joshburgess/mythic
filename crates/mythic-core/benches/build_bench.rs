use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use mythic_core::bench_utils::generate_site;
use mythic_core::build;
use mythic_core::config::SiteConfig;
use mythic_core::page::Page;

fn noop_render(pages: &mut [Page]) {
    for page in pages {
        page.rendered_html = Some(page.raw_content.clone());
    }
}

type NoTemplate = fn(&Page, &SiteConfig) -> anyhow::Result<String>;

fn bench_full_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_build");
    group.sample_size(10);

    for size in &[100, 1000] {
        let dir = tempfile::tempdir().unwrap();
        generate_site(dir.path(), *size, 42);
        let config = mythic_core::config::load_config(&dir.path().join("mythic.toml")).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Clean output between runs
                let output = dir.path().join("public");
                if output.exists() {
                    std::fs::remove_dir_all(&output).ok();
                }
                build::build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_incremental_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_build");
    group.sample_size(10);

    let dir = tempfile::tempdir().unwrap();
    generate_site(dir.path(), 1000, 42);
    let config = mythic_core::config::load_config(&dir.path().join("mythic.toml")).unwrap();

    // First full build
    build::build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();

    group.bench_function("1000_pages_no_change", |b| {
        b.iter(|| {
            build::build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();
        });
    });

    group.finish();
}

fn bench_markdown_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("markdown_rendering");
    group.sample_size(10);

    let dir = tempfile::tempdir().unwrap();
    generate_site(dir.path(), 500, 42);
    let config = mythic_core::config::load_config(&dir.path().join("mythic.toml")).unwrap();
    let pages = mythic_core::content::discover_content(&config, dir.path()).unwrap();

    group.bench_function("500_pages", |b| {
        b.iter(|| {
            let mut pages = pages.clone();
            noop_render(&mut pages);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_build,
    bench_incremental_build,
    bench_markdown_rendering
);
criterion_main!(benches);
