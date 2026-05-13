use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use repolens::scanner::Scanner;
use std::fs;
use tempfile::TempDir;

// Helper function to create a test repository with various file structures
fn create_test_repo(size: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    match size {
        "small" => {
            // 10 files in simple structure
            fs::create_dir_all(root.join("src")).unwrap();
            for i in 0..5 {
                fs::write(
                    root.join(format!("src/file{}.rs", i)),
                    format!("fn test{i}() {{}}"),
                )
                .unwrap();
            }
            fs::write(root.join("README.md"), "# Test Project").unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"test\"\nversion = \"0.1.0\"",
            )
            .unwrap();
            fs::write(root.join(".gitignore"), "target/\n*.log").unwrap();
            fs::write(root.join("LICENSE"), "MIT License").unwrap();
            fs::write(root.join(".env"), "API_KEY=test").unwrap();
        }
        "medium" => {
            // ~50 files with nested directories
            fs::create_dir_all(root.join("src/modules/auth")).unwrap();
            fs::create_dir_all(root.join("src/modules/api")).unwrap();
            fs::create_dir_all(root.join("tests")).unwrap();
            fs::create_dir_all(root.join("docs")).unwrap();
            fs::create_dir_all(root.join(".github/workflows")).unwrap();

            // Source files
            for i in 0..15 {
                fs::write(
                    root.join(format!("src/file{}.rs", i)),
                    format!("pub fn function{i}() {{\n    println!(\"test\");\n}}\n"),
                )
                .unwrap();
            }
            for i in 0..10 {
                fs::write(
                    root.join(format!("src/modules/auth/auth{}.rs", i)),
                    format!("pub struct Auth{i};\nimpl Auth{i} {{}}\n"),
                )
                .unwrap();
            }
            for i in 0..10 {
                fs::write(
                    root.join(format!("src/modules/api/handler{}.rs", i)),
                    format!("async fn handle{i}() {{\n    // handler\n}}\n"),
                )
                .unwrap();
            }

            // Test files
            for i in 0..8 {
                fs::write(
                    root.join(format!("tests/test{}.rs", i)),
                    format!("#[test]\nfn test_case{i}() {{\n    assert!(true);\n}}\n"),
                )
                .unwrap();
            }

            // Docs
            for i in 0..5 {
                fs::write(
                    root.join(format!("docs/doc{}.md", i)),
                    format!("# Documentation {i}\n\nContent here.\n"),
                )
                .unwrap();
            }

            // Workflows
            fs::write(
                root.join(".github/workflows/ci.yml"),
                "name: CI\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest",
            )
            .unwrap();
            fs::write(
                root.join(".github/workflows/release.yml"),
                "name: Release\non:\n  release:\njobs:\n  build:\n    runs-on: ubuntu-latest",
            )
            .unwrap();

            // Root files
            fs::write(
                root.join("README.md"),
                "# Medium Test Project\n\nDescription here.\n",
            )
            .unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
            )
            .unwrap();
            fs::write(root.join(".gitignore"), "target/\n*.log\n.env\n").unwrap();
        }
        "large" => {
            // ~200 files with deep nesting
            fs::create_dir_all(root.join("src/core")).unwrap();
            fs::create_dir_all(root.join("src/modules")).unwrap();
            fs::create_dir_all(root.join("src/utils")).unwrap();
            fs::create_dir_all(root.join("src/api/v1")).unwrap();
            fs::create_dir_all(root.join("src/api/v2")).unwrap();
            fs::create_dir_all(root.join("tests/unit")).unwrap();
            fs::create_dir_all(root.join("tests/integration")).unwrap();
            fs::create_dir_all(root.join("benches")).unwrap();
            fs::create_dir_all(root.join("examples")).unwrap();
            fs::create_dir_all(root.join("docs/api")).unwrap();
            fs::create_dir_all(root.join("docs/guides")).unwrap();

            // Core files (40)
            for i in 0..40 {
                fs::write(
                    root.join(format!("src/core/module{}.rs", i)),
                    format!("pub mod module{i};\npub struct Core{i};\nimpl Core{i} {{\n    pub fn new() -> Self {{\n        Self\n    }}\n}}\n"),
                ).unwrap();
            }

            // Module files (50)
            for i in 0..50 {
                fs::write(
                    root.join(format!("src/modules/mod{}.rs", i)),
                    format!(
                        "pub fn process{i}(input: &str) -> String {{\n    input.to_string()\n}}\n"
                    ),
                )
                .unwrap();
            }

            // Utils (30)
            for i in 0..30 {
                fs::write(
                    root.join(format!("src/utils/util{}.rs", i)),
                    format!("pub fn helper{i}() {{\n    // helper function\n}}\n"),
                )
                .unwrap();
            }

            // API files (30)
            for i in 0..15 {
                fs::write(
                    root.join(format!("src/api/v1/endpoint{}.rs", i)),
                    format!("pub async fn handle_v1_{i}() {{\n    // v1 handler\n}}\n"),
                )
                .unwrap();
                fs::write(
                    root.join(format!("src/api/v2/endpoint{}.rs", i)),
                    format!("pub async fn handle_v2_{i}() {{\n    // v2 handler\n}}\n"),
                )
                .unwrap();
            }

            // Tests (30)
            for i in 0..20 {
                fs::write(
                    root.join(format!("tests/unit/test{}.rs", i)),
                    format!("#[test]\nfn unit_test{i}() {{\n    assert_eq!(1, 1);\n}}\n"),
                )
                .unwrap();
            }
            for i in 0..10 {
                fs::write(
                    root.join(format!("tests/integration/integration{}.rs", i)),
                    format!("#[tokio::test]\nasync fn integration_test{i}() {{\n    // test\n}}\n"),
                )
                .unwrap();
            }

            // Benchmarks (10)
            for i in 0..10 {
                fs::write(
                    root.join(format!("benches/bench{}.rs", i)),
                    format!("use criterion::{{criterion_group, criterion_main, Criterion}};\nfn bench{i}(c: &mut Criterion) {{}}\n"),
                ).unwrap();
            }

            // Examples (10)
            for i in 0..10 {
                fs::write(
                    root.join(format!("examples/example{}.rs", i)),
                    format!("fn main() {{\n    println!(\"Example {i}\");\n}}\n"),
                )
                .unwrap();
            }

            // Docs (20)
            for i in 0..10 {
                fs::write(
                    root.join(format!("docs/api/api{}.md", i)),
                    format!("# API Documentation {i}\n\n## Endpoints\n\nContent here.\n"),
                )
                .unwrap();
                fs::write(
                    root.join(format!("docs/guides/guide{}.md", i)),
                    format!("# Guide {i}\n\n## Introduction\n\nGuide content here.\n"),
                )
                .unwrap();
            }

            // Root files
            fs::write(
                root.join("README.md"),
                "# Large Test Project\n\nA comprehensive test repository.\n",
            )
            .unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"large_test\"\nversion = \"1.0.0\"\n\n[dependencies]\n",
            )
            .unwrap();
            fs::write(
                root.join(".gitignore"),
                "target/\n*.log\n.env\nCargo.lock\n",
            )
            .unwrap();
            fs::write(
                root.join("LICENSE"),
                "MIT License\n\nFull license text...\n",
            )
            .unwrap();
        }
        _ => panic!("Unknown size: {}", size),
    }

    temp_dir
}

fn benchmark_scanner_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_initialization");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let path = temp_dir.path().to_path_buf();

            b.iter(|| {
                let scanner = Scanner::new(black_box(path.clone()));
                black_box(scanner);
            });
        });
    }

    group.finish();
}

fn benchmark_file_exists(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_exists");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| {
                black_box(scanner.file_exists("README.md"));
                black_box(scanner.file_exists("nonexistent.txt"));
            });
        });
    }

    group.finish();
}

fn benchmark_files_with_extensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("files_with_extensions");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| {
                let files = scanner.files_with_extensions(black_box(&["rs", "md", "toml"]));
                black_box(files);
            });
        });
    }

    group.finish();
}

fn benchmark_files_matching_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("files_matching_pattern");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| {
                let files = scanner.files_matching_pattern(black_box("*.rs"));
                black_box(files);
            });
        });
    }

    group.finish();
}

fn benchmark_files_larger_than(c: &mut Criterion) {
    let mut group = c.benchmark_group("files_larger_than");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| {
                let files = scanner.files_larger_than(black_box(1000));
                black_box(files);
            });
        });
    }

    group.finish();
}

fn benchmark_read_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_file");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| {
                let content = scanner.read_file(black_box("README.md"));
                let _ = black_box(content);
            });
        });
    }

    group.finish();
}

fn benchmark_files_in_directory(c: &mut Criterion) {
    let mut group = c.benchmark_group("files_in_directory");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = create_test_repo(size);
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| {
                let files = scanner.files_in_directory(black_box("src"));
                black_box(files);
            });
        });
    }

    group.finish();
}

fn benchmark_repository_name(c: &mut Criterion) {
    let temp_dir = create_test_repo("medium");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());

    c.bench_function("repository_name", |b| {
        b.iter(|| {
            let name = scanner.repository_name();
            black_box(name);
        });
    });
}

criterion_group!(
    benches,
    benchmark_scanner_initialization,
    benchmark_file_exists,
    benchmark_files_with_extensions,
    benchmark_files_matching_pattern,
    benchmark_files_larger_than,
    benchmark_read_file,
    benchmark_files_in_directory,
    benchmark_repository_name,
);
criterion_main!(benches);
