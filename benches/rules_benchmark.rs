use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use repolens::config::Config;
use repolens::rules::engine::RulesEngine;
use repolens::scanner::Scanner;
use std::fs;
use tempfile::TempDir;

// Helper function to create a test repository for rule benchmarking
fn create_rules_test_repo(scenario: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    match scenario {
        "minimal" => {
            // Minimal repo with just required files
            fs::write(root.join("README.md"), "# Test Project").unwrap();
            fs::write(root.join("LICENSE"), "MIT License").unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
        }
        "typical" => {
            // Typical repo structure with various files
            fs::create_dir_all(root.join("src")).unwrap();
            fs::create_dir_all(root.join("tests")).unwrap();
            fs::create_dir_all(root.join(".github/workflows")).unwrap();

            // Source files
            fs::write(
                root.join("src/main.rs"),
                "fn main() {\n    println!(\"Hello, world!\");\n}\n",
            )
            .unwrap();
            fs::write(
                root.join("src/lib.rs"),
                "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
            )
            .unwrap();

            // Tests
            fs::write(
                root.join("tests/integration.rs"),
                "#[test]\nfn test_add() {\n    assert_eq!(test::add(2, 2), 4);\n}\n",
            )
            .unwrap();

            // Documentation
            fs::write(
                root.join("README.md"),
                "# Test Project\n\n## Installation\n\n```bash\ncargo install test\n```\n\n## Usage\n\nExample usage here.\n",
            )
            .unwrap();
            fs::write(root.join("LICENSE"), "MIT License\n\nCopyright (c) 2024\n").unwrap();
            fs::write(
                root.join("CHANGELOG.md"),
                "# Changelog\n\n## [0.1.0] - 2024-01-01\n\n- Initial release\n",
            )
            .unwrap();

            // Configuration
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1.0\"\n",
            )
            .unwrap();
            fs::write(
                root.join(".gitignore"),
                "target/\nCargo.lock\n*.swp\n.env\n",
            )
            .unwrap();

            // Workflows
            fs::write(
                root.join(".github/workflows/ci.yml"),
                "name: CI\n\non:\n  push:\n    branches: [main]\n  pull_request:\n\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v3\n      - run: cargo test\n",
            )
            .unwrap();
        }
        "complex" => {
            // Complex repo with many files and potential issues
            fs::create_dir_all(root.join("src/core")).unwrap();
            fs::create_dir_all(root.join("src/api")).unwrap();
            fs::create_dir_all(root.join("tests")).unwrap();
            fs::create_dir_all(root.join("docs")).unwrap();
            fs::create_dir_all(root.join(".github/workflows")).unwrap();

            // Multiple source files
            for i in 0..10 {
                fs::write(
                    root.join(format!("src/core/module{}.rs", i)),
                    format!("pub struct Module{i} {{\n    data: String,\n}}\n\nimpl Module{i} {{\n    pub fn new() -> Self {{\n        Self {{ data: String::new() }}\n    }}\n}}\n"),
                ).unwrap();
            }

            // API files with potential secrets (for secrets detection)
            fs::write(
                root.join("src/api/client.rs"),
                "const API_KEY: &str = \"sk_test_1234567890abcdefghijklmnop\";\n\npub fn make_request() {\n    // API call\n}\n",
            )
            .unwrap();
            fs::write(
                root.join("src/api/config.rs"),
                "pub struct Config {\n    pub database_url: String,\n}\n",
            )
            .unwrap();

            // Tests
            for i in 0..5 {
                fs::write(
                    root.join(format!("tests/test{}.rs", i)),
                    format!("#[test]\nfn test{i}() {{\n    assert!(true);\n}}\n"),
                )
                .unwrap();
            }

            // Documentation
            fs::write(
                root.join("README.md"),
                "# Complex Test Project\n\n## Features\n\n- Feature 1\n- Feature 2\n\n## Installation\n\n```bash\ncargo install complex_test\n```\n\n## Configuration\n\nSee docs for details.\n",
            )
            .unwrap();
            fs::write(root.join("LICENSE"), "MIT License\n").unwrap();
            fs::write(
                root.join("CONTRIBUTING.md"),
                "# Contributing\n\n## Guidelines\n\n1. Fork the repo\n2. Create a branch\n3. Make changes\n4. Submit PR\n",
            )
            .unwrap();
            fs::write(
                root.join("SECURITY.md"),
                "# Security Policy\n\n## Reporting\n\nPlease report security issues to security@example.com\n",
            )
            .unwrap();

            // Multiple docs
            for i in 0..5 {
                fs::write(
                    root.join(format!("docs/guide{}.md", i)),
                    format!("# Guide {i}\n\nContent here.\n"),
                )
                .unwrap();
            }

            // Cargo.toml with dependencies
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"complex_test\"\nversion = \"1.0.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1.0\"\ntokio = \"1.0\"\nrequests = \"0.9\"\n\n[dev-dependencies]\ncriterion = \"0.5\"\n",
            )
            .unwrap();

            // Cargo.lock for dependency checking
            fs::write(
                root.join("Cargo.lock"),
                "[[package]]\nname = \"complex_test\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.195\"\n",
            )
            .unwrap();

            // .gitignore
            fs::write(
                root.join(".gitignore"),
                "target/\nCargo.lock\n*.swp\n.env\n.DS_Store\n",
            )
            .unwrap();

            // Workflows
            fs::write(
                root.join(".github/workflows/ci.yml"),
                "name: CI\n\non:\n  push:\n  pull_request:\n\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v3\n      - run: cargo test\n  lint:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v3\n      - run: cargo clippy\n",
            )
            .unwrap();

            // Large file (for large file detection)
            let large_content = "x".repeat(15 * 1024 * 1024); // 15MB
            fs::write(root.join("large_file.bin"), large_content).unwrap();

            // Temporary files (for temp file detection)
            fs::write(root.join("test.tmp"), "temporary").unwrap();
            fs::write(root.join("backup.bak"), "backup").unwrap();
        }
        _ => panic!("Unknown scenario: {}", scenario),
    }

    temp_dir
}

fn benchmark_full_audit(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_audit");
    // Reduce sample size for slower benchmarks
    group.sample_size(10);

    for scenario in &["minimal", "typical", "complex"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(scenario),
            scenario,
            |b, &scenario| {
                b.iter(|| {
                    let temp_dir = create_rules_test_repo(scenario);
                    let config = Config::default();
                    let scanner = Scanner::new(temp_dir.path().to_path_buf());
                    let engine = RulesEngine::new(config);

                    // Use tokio runtime for async execution
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    let results = runtime.block_on(async { engine.run(&scanner).await });

                    black_box(results.unwrap());
                });
            },
        );
    }

    group.finish();
}

fn benchmark_single_category_secrets(c: &mut Criterion) {
    let temp_dir = create_rules_test_repo("complex");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());
    let config = Config::default();

    c.bench_function("single_category_secrets", |b| {
        b.iter(|| {
            let mut engine = RulesEngine::new(config.clone());
            engine.set_only_categories(vec!["secrets".to_string()]);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(async { engine.run(black_box(&scanner)).await });

            black_box(results.unwrap());
        });
    });
}

fn benchmark_single_category_files(c: &mut Criterion) {
    let temp_dir = create_rules_test_repo("complex");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());
    let config = Config::default();

    c.bench_function("single_category_files", |b| {
        b.iter(|| {
            let mut engine = RulesEngine::new(config.clone());
            engine.set_only_categories(vec!["files".to_string()]);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(async { engine.run(black_box(&scanner)).await });

            black_box(results.unwrap());
        });
    });
}

fn benchmark_single_category_docs(c: &mut Criterion) {
    let temp_dir = create_rules_test_repo("complex");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());
    let config = Config::default();

    c.bench_function("single_category_docs", |b| {
        b.iter(|| {
            let mut engine = RulesEngine::new(config.clone());
            engine.set_only_categories(vec!["docs".to_string()]);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(async { engine.run(black_box(&scanner)).await });

            black_box(results.unwrap());
        });
    });
}

fn benchmark_single_category_security(c: &mut Criterion) {
    let temp_dir = create_rules_test_repo("complex");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());
    let config = Config::default();

    c.bench_function("single_category_security", |b| {
        b.iter(|| {
            let mut engine = RulesEngine::new(config.clone());
            engine.set_only_categories(vec!["security".to_string()]);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(async { engine.run(black_box(&scanner)).await });

            black_box(results.unwrap());
        });
    });
}

fn benchmark_single_category_workflows(c: &mut Criterion) {
    let temp_dir = create_rules_test_repo("typical");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());
    let config = Config::default();

    c.bench_function("single_category_workflows", |b| {
        b.iter(|| {
            let mut engine = RulesEngine::new(config.clone());
            engine.set_only_categories(vec!["workflows".to_string()]);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(async { engine.run(black_box(&scanner)).await });

            black_box(results.unwrap());
        });
    });
}

fn benchmark_multiple_categories(c: &mut Criterion) {
    let temp_dir = create_rules_test_repo("complex");
    let scanner = Scanner::new(temp_dir.path().to_path_buf());
    let config = Config::default();

    c.bench_function("multiple_categories", |b| {
        b.iter(|| {
            let mut engine = RulesEngine::new(config.clone());
            engine.set_only_categories(vec![
                "secrets".to_string(),
                "files".to_string(),
                "docs".to_string(),
            ]);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            let results = runtime.block_on(async { engine.run(black_box(&scanner)).await });

            black_box(results.unwrap());
        });
    });
}

fn benchmark_rules_engine_creation(c: &mut Criterion) {
    let config = Config::default();

    c.bench_function("rules_engine_creation", |b| {
        b.iter(|| {
            let engine = RulesEngine::new(black_box(config.clone()));
            black_box(engine);
        });
    });
}

fn benchmark_different_presets(c: &mut Criterion) {
    let mut group = c.benchmark_group("different_presets");
    group.sample_size(10);

    for preset in &["opensource", "enterprise", "strict"] {
        group.bench_with_input(BenchmarkId::from_parameter(preset), preset, |b, &preset| {
            b.iter(|| {
                let temp_dir = create_rules_test_repo("typical");
                let config = Config {
                    preset: preset.to_string(),
                    ..Default::default()
                };
                let scanner = Scanner::new(temp_dir.path().to_path_buf());
                let engine = RulesEngine::new(config);

                let runtime = tokio::runtime::Runtime::new().unwrap();
                let results = runtime.block_on(async { engine.run(&scanner).await });

                black_box(results.unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_full_audit,
    benchmark_single_category_secrets,
    benchmark_single_category_files,
    benchmark_single_category_docs,
    benchmark_single_category_security,
    benchmark_single_category_workflows,
    benchmark_multiple_categories,
    benchmark_rules_engine_creation,
    benchmark_different_presets,
);
criterion_main!(benches);
