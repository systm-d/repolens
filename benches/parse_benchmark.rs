use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use repolens::rules::categories::dependencies::{
    parse_cargo_lock, parse_package_lock, parse_requirements_txt,
};
use repolens::scanner::Scanner;
use std::fs;
use tempfile::TempDir;

fn generate_cargo_lock(size: &str) -> String {
    let mut content = String::from("[[package]]\nname = \"repolens\"\nversion = \"0.1.0\"\n\n");
    let count = match size {
        "small" => 10,
        "medium" => 50,
        "large" => 200,
        _ => 10,
    };

    for i in 0..count {
        content.push_str(&format!(
            "[[package]]\nname = \"dep{}\"\nversion = \"1.0.{}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"abcd{}\"\n\n",
            i, i, i
        ));
    }
    content
}

fn generate_package_lock(size: &str) -> String {
    let count = match size {
        "small" => 10,
        "medium" => 50,
        "large" => 200,
        _ => 10,
    };

    let mut packages = String::from(r#"{"name": "test", "version": "1.0.0"}"#);
    for i in 0..count {
        packages.push_str(&format!(
            r#", "dep{}": {{"name": "dep{}", "version": "1.0.{}", "resolved": "https://registry.npmjs.org/dep{}", "integrity": "sha512-abcd{}"}}  "#,
            i, i, i, i, i
        ));
    }

    format!(r#"{{"packages": {{{}}}}}"#, packages)
}

fn generate_requirements_txt(size: &str) -> String {
    let count = match size {
        "small" => 10,
        "medium" => 50,
        "large" => 200,
        _ => 10,
    };

    let mut content = String::new();
    for i in 0..count {
        content.push_str(&format!("package{}==1.0.{}\n", i, i));
    }
    content
}

fn benchmark_parse_cargo_lock(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_cargo_lock");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = TempDir::new().unwrap();
            let lock_content = generate_cargo_lock(size);
            fs::write(temp_dir.path().join("Cargo.lock"), lock_content).unwrap();
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| parse_cargo_lock(black_box(&scanner)));
        });
    }

    group.finish();
}

fn benchmark_parse_package_lock(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_package_lock");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = TempDir::new().unwrap();
            let lock_content = generate_package_lock(size);
            fs::write(temp_dir.path().join("package-lock.json"), lock_content).unwrap();
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| parse_package_lock(black_box(&scanner)));
        });
    }

    group.finish();
}

fn benchmark_parse_requirements_txt(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_requirements_txt");

    for size in &["small", "medium", "large"] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let temp_dir = TempDir::new().unwrap();
            let req_content = generate_requirements_txt(size);
            fs::write(temp_dir.path().join("requirements.txt"), req_content).unwrap();
            let scanner = Scanner::new(temp_dir.path().to_path_buf());

            b.iter(|| parse_requirements_txt(black_box(&scanner)));
        });
    }

    group.finish();
}

fn benchmark_parse_cargo_lock_repeated(c: &mut Criterion) {
    // Benchmark parsing the same file multiple times (cache behavior)
    let temp_dir = TempDir::new().unwrap();
    let lock_content = generate_cargo_lock("medium");
    fs::write(temp_dir.path().join("Cargo.lock"), lock_content).unwrap();
    let scanner = Scanner::new(temp_dir.path().to_path_buf());

    c.bench_function("parse_cargo_lock_repeated", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let _ = parse_cargo_lock(black_box(&scanner));
            }
        });
    });
}

fn benchmark_parse_multiple_formats(c: &mut Criterion) {
    // Benchmark parsing multiple different lock file formats
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::write(root.join("Cargo.lock"), generate_cargo_lock("small")).unwrap();
    fs::write(
        root.join("package-lock.json"),
        generate_package_lock("small"),
    )
    .unwrap();
    fs::write(
        root.join("requirements.txt"),
        generate_requirements_txt("small"),
    )
    .unwrap();

    let scanner = Scanner::new(root.to_path_buf());

    c.bench_function("parse_multiple_formats", |b| {
        b.iter(|| {
            let _ = parse_cargo_lock(black_box(&scanner));
            let _ = parse_package_lock(black_box(&scanner));
            let _ = parse_requirements_txt(black_box(&scanner));
        });
    });
}

criterion_group!(
    benches,
    benchmark_parse_cargo_lock,
    benchmark_parse_package_lock,
    benchmark_parse_requirements_txt,
    benchmark_parse_cargo_lock_repeated,
    benchmark_parse_multiple_formats,
);
criterion_main!(benches);
