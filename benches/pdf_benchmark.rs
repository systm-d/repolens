use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use repolens::cli::output::PdfReport;
use repolens::rules::results::{AuditResults, Finding, Severity};

fn make_results(size: usize) -> AuditResults {
    let mut r = AuditResults::new("bench-repo", "opensource");
    let categories = [
        "secrets",
        "files",
        "docs",
        "security",
        "workflows",
        "quality",
    ];
    for i in 0..size {
        let cat = categories[i % categories.len()];
        let sev = match i % 3 {
            0 => Severity::Critical,
            1 => Severity::Warning,
            _ => Severity::Info,
        };
        r.add_finding(
            Finding::new(
                format!("RULE{:04}", i),
                cat,
                sev,
                format!("Finding {i} for {cat}: an example diagnostic message that simulates real audit output"),
            )
            .with_location(format!("path/to/source/file{i}.rs:{}", i + 1))
            .with_description("Description of the issue")
            .with_remediation("Suggested remediation steps"),
        );
    }
    r
}

fn pdf_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf_generation");
    group.sample_size(10);

    for size in [100usize, 1_000, 5_000] {
        let results = make_results(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &results, |b, results| {
            b.iter(|| {
                let renderer = PdfReport::new(false);
                let bytes = renderer
                    .render_to_bytes(black_box(results))
                    .expect("render");
                black_box(bytes.len());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, pdf_generation);
criterion_main!(benches);
