//! Code quality rules
//!
//! This module provides rules for checking code quality aspects, including:
//! - Test files and directories
//! - Linting configuration
//! - Editor configuration files
//! - Code coverage configuration
//! - API documentation
//! - Complexity analysis tools
//! - Dead code detection tools
//! - Naming convention enforcement

use crate::error::RepoLensError;

use crate::config::Config;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;
use crate::utils::language_detection::{Language, detect_languages};

/// Rules for checking code quality
pub struct QualityRules;

#[async_trait::async_trait]
impl RuleCategory for QualityRules {
    /// Get the category name
    fn name(&self) -> &'static str {
        "quality"
    }

    /// Run all quality-related rules
    ///
    /// # Arguments
    ///
    /// * `scanner` - The scanner to access repository files
    /// * `config` - The configuration with enabled rules
    ///
    /// # Returns
    ///
    /// A vector of findings for quality issues
    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        // Check for tests
        if config.is_rule_enabled("quality/tests") {
            findings.extend(check_tests(scanner).await?);
        }

        // Check for linting configuration
        if config.is_rule_enabled("quality/linting") {
            findings.extend(check_linting(scanner).await?);
        }

        // Check for editor configuration
        if config.is_rule_enabled("files/editorconfig") {
            findings.extend(check_editorconfig(scanner).await?);
        }

        // Check for coverage configuration
        if config.is_rule_enabled("quality/coverage") {
            findings.extend(check_coverage_config(scanner).await?);
        }

        // Check for API documentation
        if config.is_rule_enabled("quality/api-docs") {
            findings.extend(check_api_docs(scanner).await?);
        }

        // Check for complexity analysis configuration
        if config.is_rule_enabled("quality/complexity") {
            findings.extend(check_complexity_config(scanner).await?);
        }

        // Check for dead code detection
        if config.is_rule_enabled("quality/dead-code") {
            findings.extend(check_dead_code_config(scanner).await?);
        }

        // Check for naming convention enforcement
        if config.is_rule_enabled("quality/naming-conventions") {
            findings.extend(check_naming_conventions_config(scanner).await?);
        }

        Ok(findings)
    }
}

/// Check for test files and test configuration
///
/// Verifies that the repository has tests and appropriate test configuration.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for test-related issues
async fn check_tests(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Check for test directories
    let test_dirs = ["test", "tests", "__tests__", "spec", "specs"];
    let has_test_dir = test_dirs.iter().any(|d| scanner.directory_exists(d));

    // Check for test files
    let test_file_patterns = ["*.test.*", "*.spec.*", "*_test.*", "*Test.*"];
    let has_test_files = test_file_patterns
        .iter()
        .any(|p| !scanner.files_matching_pattern(p).is_empty());

    if !has_test_dir && !has_test_files {
        findings.push(
            Finding::new(
                "QUALITY001",
                "quality",
                Severity::Info,
                "No tests detected",
            )
            .with_description(
                "Tests are important for ensuring code quality and catching regressions.",
            )
            .with_remediation(
                "Add tests to your project. Consider using a testing framework appropriate for your language.",
            ),
        );
    }

    // Check if package.json has test script
    if scanner.file_exists("package.json") {
        if let Ok(content) = scanner.read_file("package.json") {
            if !content.contains(r#""test""#) || content.contains(r#""test": "echo"#) {
                findings.push(
                    Finding::new(
                        "QUALITY002",
                        "quality",
                        Severity::Info,
                        "No test script defined in package.json",
                    )
                    .with_description(
                        "A 'test' script in package.json enables running tests with 'npm test'.",
                    ),
                );
            }
        }
    }

    Ok(findings)
}

/// Check for linting configuration files
///
/// Verifies that appropriate linting tools are configured based on
/// the project type (JavaScript, Python, Ruby, Go, Rust, etc.).
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing linting configuration
async fn check_linting(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Linting config files by language/tool
    let linting_configs = [
        // JavaScript/TypeScript
        (".eslintrc", "ESLint"),
        (".eslintrc.js", "ESLint"),
        (".eslintrc.json", "ESLint"),
        (".eslintrc.yml", "ESLint"),
        ("eslint.config.js", "ESLint"),
        ("biome.json", "Biome"),
        // Formatting
        (".prettierrc", "Prettier"),
        (".prettierrc.js", "Prettier"),
        (".prettierrc.json", "Prettier"),
        // Python
        ("pyproject.toml", "Python tooling"),
        (".flake8", "Flake8"),
        ("setup.cfg", "Python tooling"),
        (".pylintrc", "Pylint"),
        ("ruff.toml", "Ruff"),
        // Ruby
        (".rubocop.yml", "RuboCop"),
        // Go
        (".golangci.yml", "golangci-lint"),
        (".golangci.yaml", "golangci-lint"),
        // Rust
        ("rustfmt.toml", "rustfmt"),
        (".rustfmt.toml", "rustfmt"),
        ("clippy.toml", "Clippy"),
    ];

    // Detect project type
    let is_js_project = scanner.file_exists("package.json");
    let is_python_project =
        scanner.file_exists("pyproject.toml") || scanner.file_exists("requirements.txt");
    let is_ruby_project = scanner.file_exists("Gemfile");
    let is_go_project = scanner.file_exists("go.mod");
    let is_rust_project = scanner.file_exists("Cargo.toml");

    let has_linting = linting_configs.iter().any(|(f, _)| scanner.file_exists(f));

    if !has_linting
        && (is_js_project
            || is_python_project
            || is_ruby_project
            || is_go_project
            || is_rust_project)
    {
        let suggestion = if is_js_project {
            "ESLint for linting and Prettier for formatting"
        } else if is_python_project {
            "Ruff or Flake8 for linting"
        } else if is_ruby_project {
            "RuboCop for linting"
        } else if is_go_project {
            "golangci-lint for linting"
        } else {
            "Clippy for linting and rustfmt for formatting"
        };

        findings.push(
            Finding::new(
                "QUALITY003",
                "quality",
                Severity::Info,
                "No linting configuration detected",
            )
            .with_description(
                "Linting tools help maintain consistent code style and catch potential issues.",
            )
            .with_remediation(format!("Consider adding {} to your project.", suggestion)),
        );
    }

    Ok(findings)
}

/// Check for .editorconfig file
///
/// Verifies that an .editorconfig file exists to maintain consistent
/// coding styles across editors and IDEs.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing .editorconfig
async fn check_editorconfig(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    if !scanner.file_exists(".editorconfig") {
        findings.push(
            Finding::new(
                "QUALITY004",
                "quality",
                Severity::Info,
                ".editorconfig file is missing",
            )
            .with_description(
                "EditorConfig helps maintain consistent coding styles across different editors and IDEs.",
            )
            .with_remediation("Create a .editorconfig file to define coding style preferences."),
        );
    }

    Ok(findings)
}

/// Check for code coverage configuration
///
/// Verifies that the repository has coverage tools configured, either via
/// config files or CI workflow steps.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing coverage configuration
async fn check_coverage_config(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let coverage_files = [
        "tarpaulin.toml",
        ".coveragerc",
        "codecov.yml",
        ".codecov.yml",
        ".nycrc",
        ".nycrc.json",
        ".coveralls.yml",
        "jest.config.js",
        "jest.config.ts",
    ];

    let has_coverage_file = coverage_files.iter().any(|f| scanner.file_exists(f));

    // Check package.json for coverage config
    let has_coverage_in_package_json = if scanner.file_exists("package.json") {
        scanner
            .read_file("package.json")
            .map(|content| content.contains("coverage"))
            .unwrap_or(false)
    } else {
        false
    };

    // Check CI workflows for coverage commands
    let has_coverage_in_ci = if scanner.directory_exists(".github/workflows") {
        scanner
            .files_in_directory(".github/workflows")
            .iter()
            .filter(|f| f.path.ends_with(".yml") || f.path.ends_with(".yaml"))
            .any(|file| {
                scanner
                    .read_file(&file.path)
                    .map(|content| {
                        let lower = content.to_lowercase();
                        lower.contains("tarpaulin")
                            || lower.contains("coverage")
                            || lower.contains("codecov")
                            || lower.contains("coveralls")
                    })
                    .unwrap_or(false)
            })
    } else {
        false
    };

    if !has_coverage_file && !has_coverage_in_package_json && !has_coverage_in_ci {
        let languages = detect_languages(scanner);
        let suggestion = if languages.contains(&Language::Rust) {
            "cargo-tarpaulin for Rust code coverage"
        } else if languages.contains(&Language::JavaScript) {
            "Jest coverage, nyc, or c8 for JavaScript/TypeScript code coverage"
        } else if languages.contains(&Language::Python) {
            "coverage.py or pytest-cov for Python code coverage"
        } else if languages.contains(&Language::Go) {
            "'go test -cover' for Go code coverage"
        } else {
            "a code coverage tool appropriate for your language"
        };

        findings.push(
            Finding::new(
                "QUALITY005",
                "quality",
                Severity::Info,
                "No code coverage configuration detected",
            )
            .with_description(
                "Code coverage helps identify untested code paths and improve test quality.",
            )
            .with_remediation(format!("Consider adding {}.", suggestion)),
        );
    }

    Ok(findings)
}

/// Check for API documentation files
///
/// Verifies that API documentation exists (OpenAPI, Swagger, Typedoc, etc.).
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing API documentation
async fn check_api_docs(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let api_doc_files = [
        "openapi.yaml",
        "openapi.yml",
        "openapi.json",
        "swagger.yaml",
        "swagger.yml",
        "swagger.json",
        "typedoc.json",
        ".typedoc.json",
        "Doxyfile",
    ];

    let api_doc_dirs = ["api-docs", "docs/api"];

    let has_api_docs = api_doc_files.iter().any(|f| scanner.file_exists(f))
        || api_doc_dirs.iter().any(|d| scanner.directory_exists(d));

    if !has_api_docs {
        findings.push(
            Finding::new(
                "QUALITY006",
                "quality",
                Severity::Info,
                "No API documentation detected",
            )
            .with_description(
                "API documentation helps consumers understand and integrate with your project.",
            )
            .with_remediation(
                "Consider adding API documentation using OpenAPI/Swagger, Typedoc, or Doxygen.",
            ),
        );
    }

    Ok(findings)
}

/// Check for complexity analysis tool configuration
///
/// Verifies that complexity analysis tools are configured.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing complexity configuration
async fn check_complexity_config(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let mut has_complexity = false;

    // Check for sonar-project.properties
    if scanner.file_exists("sonar-project.properties") {
        has_complexity = true;
    }

    // Check eslintrc files for complexity rule
    if !has_complexity {
        let eslint_files = [
            ".eslintrc",
            ".eslintrc.js",
            ".eslintrc.json",
            ".eslintrc.yml",
            "eslint.config.js",
        ];
        for f in &eslint_files {
            if scanner.file_exists(f) {
                if let Ok(content) = scanner.read_file(f) {
                    if content.contains("complexity") {
                        has_complexity = true;
                        break;
                    }
                }
            }
        }
    }

    // Check pylint config for max-complexity
    if !has_complexity {
        let pylint_files = [".pylintrc", "pyproject.toml", "setup.cfg"];
        for f in &pylint_files {
            if scanner.file_exists(f) {
                if let Ok(content) = scanner.read_file(f) {
                    if content.contains("max-complexity") {
                        has_complexity = true;
                        break;
                    }
                }
            }
        }
    }

    if !has_complexity {
        findings.push(
            Finding::new(
                "QUALITY007",
                "quality",
                Severity::Info,
                "No complexity analysis configuration detected",
            )
            .with_description(
                "Complexity analysis tools help identify overly complex code that may be hard to maintain.",
            )
            .with_remediation(
                "Consider adding complexity rules to your linter or using SonarQube for complexity analysis.",
            ),
        );
    }

    Ok(findings)
}

/// Check for dead code detection tool configuration
///
/// Verifies that dead code detection tools are configured.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing dead code detection
async fn check_dead_code_config(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let mut has_dead_code_tool = false;

    // Check for knip (JavaScript)
    let knip_files = ["knip.json", "knip.config.ts", "knip.config.js"];
    for f in &knip_files {
        if scanner.file_exists(f) {
            has_dead_code_tool = true;
            break;
        }
    }

    // Check package.json for knip
    if !has_dead_code_tool && scanner.file_exists("package.json") {
        if let Ok(content) = scanner.read_file("package.json") {
            if content.contains("knip") {
                has_dead_code_tool = true;
            }
        }
    }

    // Check Cargo.toml for dead code warnings
    if !has_dead_code_tool && scanner.file_exists("Cargo.toml") {
        if let Ok(content) = scanner.read_file("Cargo.toml") {
            if content.contains("dead_code") || content.contains("unused") {
                has_dead_code_tool = true;
            }
        }
    }

    // Check clippy.toml
    if !has_dead_code_tool && scanner.file_exists("clippy.toml") {
        if let Ok(content) = scanner.read_file("clippy.toml") {
            if content.contains("unused") {
                has_dead_code_tool = true;
            }
        }
    }

    if !has_dead_code_tool {
        findings.push(
            Finding::new(
                "QUALITY008",
                "quality",
                Severity::Info,
                "No dead code detection tool configured",
            )
            .with_description(
                "Dead code detection tools help identify unused code that can be removed to improve maintainability.",
            )
            .with_remediation(
                "Consider adding a dead code detection tool such as knip (JS/TS), vulture (Python), or enabling Rust dead_code warnings.",
            ),
        );
    }

    Ok(findings)
}

/// Check for naming convention enforcement configuration
///
/// Verifies that naming convention rules are configured in linters or editorconfig.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing naming convention enforcement
async fn check_naming_conventions_config(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let mut has_naming = false;

    // Check eslintrc files for naming-convention rule
    let eslint_files = [
        ".eslintrc",
        ".eslintrc.js",
        ".eslintrc.json",
        ".eslintrc.yml",
        "eslint.config.js",
    ];
    for f in &eslint_files {
        if scanner.file_exists(f) {
            if let Ok(content) = scanner.read_file(f) {
                if content.contains("naming-convention") {
                    has_naming = true;
                    break;
                }
            }
        }
    }

    // Check pylintrc for naming-style
    if !has_naming {
        let pylint_files = [".pylintrc", "pyproject.toml", "setup.cfg"];
        for f in &pylint_files {
            if scanner.file_exists(f) {
                if let Ok(content) = scanner.read_file(f) {
                    if content.contains("naming-style") || content.contains("naming_style") {
                        has_naming = true;
                        break;
                    }
                }
            }
        }
    }

    // Check .editorconfig for naming rules
    if !has_naming && scanner.file_exists(".editorconfig") {
        if let Ok(content) = scanner.read_file(".editorconfig") {
            if content.contains("file_header") || content.contains("dotnet_naming") {
                has_naming = true;
            }
        }
    }

    if !has_naming {
        findings.push(
            Finding::new(
                "QUALITY009",
                "quality",
                Severity::Info,
                "No naming convention enforcement detected",
            )
            .with_description(
                "Naming convention enforcement helps maintain consistent code style across the codebase.",
            )
            .with_remediation(
                "Consider adding naming convention rules to your linter (e.g., @typescript-eslint/naming-convention, pylint naming-style).",
            ),
        );
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Scanner;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_check_tests_no_tests_detected() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_tests(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY001"));
    }

    #[tokio::test]
    async fn test_check_tests_package_json_no_test_script() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let package_json = root.join("package.json");

        // Create package.json without test script or with echo test
        fs::write(
            &package_json,
            r#"{"name": "test", "version": "1.0.0", "scripts": {"test": "echo \"No tests\""}}"#,
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_tests(&scanner).await.unwrap();

        // Should find QUALITY002 because test script is just "echo"
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY002"));
    }

    #[tokio::test]
    async fn test_check_linting_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let package_json = root.join("package.json");

        fs::write(&package_json, r#"{"name": "test"}"#).unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linting(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY003"));
    }

    #[tokio::test]
    async fn test_check_editorconfig_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_editorconfig(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY004"));
    }

    #[tokio::test]
    async fn test_check_editorconfig_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join(".editorconfig"), "root = true").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_editorconfig(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_tests_with_test_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("tests/test.rs"), "fn test() {}").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_tests(&scanner).await.unwrap();

        // Has test directory, so no QUALITY001 finding
        assert!(findings.iter().all(|f| f.rule_id != "QUALITY001"));
    }

    #[tokio::test]
    async fn test_check_tests_package_json_with_proper_test_script() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "scripts": {"test": "jest"}}"#,
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_tests(&scanner).await.unwrap();

        // Has proper test script, so no QUALITY002 finding
        assert!(findings.iter().all(|f| f.rule_id != "QUALITY002"));
    }

    #[tokio::test]
    async fn test_check_linting_python_project() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("requirements.txt"), "flask==2.0").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linting(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "QUALITY003"));
    }

    #[tokio::test]
    async fn test_check_linting_ruby_project() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("Gemfile"), "source 'https://rubygems.org'").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linting(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "QUALITY003"));
    }

    #[tokio::test]
    async fn test_check_linting_go_project() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("go.mod"), "module example.com/foo").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linting(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "QUALITY003"));
    }

    #[tokio::test]
    async fn test_check_linting_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linting(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "QUALITY003"));
    }

    #[tokio::test]
    async fn test_check_linting_with_config_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::write(root.join(".eslintrc.json"), "{}").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linting(&scanner).await.unwrap();

        // Has ESLint config, so no QUALITY003 finding
        assert!(findings.iter().all(|f| f.rule_id != "QUALITY003"));
    }

    #[tokio::test]
    async fn test_quality_rules_name() {
        let rules = QualityRules;
        assert_eq!(rules.name(), "quality");
    }

    #[tokio::test]
    async fn test_quality_rules_run() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = QualityRules.run(&scanner, &config).await.unwrap();

        // Should have findings for missing tests and editorconfig
        assert!(!findings.is_empty());
    }

    // ===== QUALITY005: Coverage Config Tests =====

    #[tokio::test]
    async fn test_check_coverage_config_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_coverage_config(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY005"));
    }

    #[tokio::test]
    async fn test_check_coverage_config_with_tarpaulin() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("tarpaulin.toml"), "[report]\nout = [\"html\"]").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_coverage_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY005"));
    }

    #[tokio::test]
    async fn test_check_coverage_config_with_codecov_in_ci() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\njobs:\n  test:\n    steps:\n      - uses: codecov/codecov-action@v4",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_coverage_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY005"));
    }

    #[tokio::test]
    async fn test_check_coverage_config_with_package_json_coverage() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("package.json"),
            r#"{"name": "test", "scripts": {"test": "jest --coverage"}}"#,
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_coverage_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY005"));
    }

    // ===== QUALITY006: API Docs Tests =====

    #[tokio::test]
    async fn test_check_api_docs_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_api_docs(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY006"));
    }

    #[tokio::test]
    async fn test_check_api_docs_with_openapi() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("openapi.yaml"), "openapi: 3.0.0").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_api_docs(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY006"));
    }

    #[tokio::test]
    async fn test_check_api_docs_with_swagger() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("swagger.json"), "{}").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_api_docs(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY006"));
    }

    #[tokio::test]
    async fn test_check_api_docs_with_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::create_dir_all(root.join("docs/api")).unwrap();
        fs::write(root.join("docs/api/index.html"), "<html></html>").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_api_docs(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY006"));
    }

    // ===== QUALITY007: Complexity Config Tests =====

    #[tokio::test]
    async fn test_check_complexity_config_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_complexity_config(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY007"));
    }

    #[tokio::test]
    async fn test_check_complexity_config_with_sonar() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("sonar-project.properties"),
            "sonar.projectKey=test",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_complexity_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY007"));
    }

    #[tokio::test]
    async fn test_check_complexity_config_with_eslint_complexity() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join(".eslintrc.json"),
            r#"{"rules": {"complexity": ["error", 10]}}"#,
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_complexity_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY007"));
    }

    #[tokio::test]
    async fn test_check_complexity_config_with_pylint_max_complexity() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join(".pylintrc"), "[FORMAT]\nmax-complexity=10").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_complexity_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY007"));
    }

    // ===== QUALITY008: Dead Code Config Tests =====

    #[tokio::test]
    async fn test_check_dead_code_config_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dead_code_config(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY008"));
    }

    #[tokio::test]
    async fn test_check_dead_code_config_with_knip() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("knip.json"), r#"{"entry": ["src/index.ts"]}"#).unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dead_code_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY008"));
    }

    #[tokio::test]
    async fn test_check_dead_code_config_with_knip_in_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("package.json"),
            r#"{"devDependencies": {"knip": "^5.0.0"}}"#,
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dead_code_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY008"));
    }

    #[tokio::test]
    async fn test_check_dead_code_config_with_cargo_unused() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"test\"\n\n[lints.rust]\nunused = \"warn\"",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dead_code_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY008"));
    }

    // ===== QUALITY009: Naming Conventions Config Tests =====

    #[tokio::test]
    async fn test_check_naming_conventions_config_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_naming_conventions_config(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "QUALITY009"));
    }

    #[tokio::test]
    async fn test_check_naming_conventions_config_with_eslint() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join(".eslintrc.json"),
            r#"{"rules": {"@typescript-eslint/naming-convention": "error"}}"#,
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_naming_conventions_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY009"));
    }

    #[tokio::test]
    async fn test_check_naming_conventions_config_with_pylint() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join(".pylintrc"),
            "[BASIC]\nvariable-naming-style=snake_case",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_naming_conventions_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY009"));
    }

    #[tokio::test]
    async fn test_check_naming_conventions_config_with_editorconfig() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join(".editorconfig"),
            "root = true\n[*.cs]\ndotnet_naming_rule.example = true",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_naming_conventions_config(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "QUALITY009"));
    }
}
