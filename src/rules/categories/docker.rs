//! Docker rules
//!
//! This module provides rules for checking Dockerfile best practices, including:
//! - Dockerfile presence and `.dockerignore` configuration
//! - Pinned base image tags
//! - Security practices (USER instruction, secrets in ENV/ARG)
//! - Health checks and multi-stage builds
//! - COPY patterns

use crate::config::Config;
use crate::error::RepoLensError;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;

/// Rules for checking Dockerfile best practices
pub struct DockerRules;

#[async_trait::async_trait]
impl RuleCategory for DockerRules {
    fn name(&self) -> &'static str {
        "docker"
    }

    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        if config.is_rule_enabled("docker/dockerfile-presence") {
            findings.extend(check_dockerfile_presence(scanner).await?);
        }

        if config.is_rule_enabled("docker/dockerignore") {
            findings.extend(check_dockerignore(scanner).await?);
        }

        if config.is_rule_enabled("docker/from-pinning") {
            findings.extend(check_pinned_tag(scanner).await?);
        }

        if config.is_rule_enabled("docker/user") {
            findings.extend(check_user_instruction(scanner).await?);
        }

        if config.is_rule_enabled("docker/healthcheck") {
            findings.extend(check_healthcheck(scanner).await?);
        }

        if config.is_rule_enabled("docker/multistage") {
            findings.extend(check_multi_stage(scanner).await?);
        }

        if config.is_rule_enabled("docker/secrets-in-env") {
            findings.extend(check_secrets_in_env(scanner).await?);
        }

        if config.is_rule_enabled("docker/copy-all") {
            findings.extend(check_copy_all(scanner).await?);
        }

        Ok(findings)
    }
}

/// Find all Dockerfile paths in the repository
fn find_dockerfiles(scanner: &Scanner) -> Vec<String> {
    let mut paths = Vec::new();
    if scanner.file_exists("Dockerfile") {
        paths.push("Dockerfile".to_string());
    }
    for file_info in scanner.files_matching_pattern("Dockerfile.*") {
        // Exclude patterns like Dockerfile.md or similar non-Dockerfile files
        // but include Dockerfile.dev, Dockerfile.prod, etc.
        paths.push(file_info.path.clone());
    }
    for file_info in scanner.files_matching_pattern("*.Dockerfile") {
        paths.push(file_info.path.clone());
    }
    paths.sort();
    paths.dedup();
    paths
}

/// DOCKER001: Dockerfile absent but docker-compose.yml or .dockerignore exists
async fn check_dockerfile_presence(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let dockerfiles = find_dockerfiles(scanner);
    let has_dockerfile = !dockerfiles.is_empty();

    let has_compose = scanner.file_exists("docker-compose.yml")
        || scanner.file_exists("docker-compose.yaml")
        || scanner.file_exists("compose.yml")
        || scanner.file_exists("compose.yaml");
    let has_dockerignore = scanner.file_exists(".dockerignore");

    if !has_dockerfile && (has_compose || has_dockerignore) {
        findings.push(
            Finding::new(
                "DOCKER001",
                "docker",
                Severity::Warning,
                "Dockerfile is missing but Docker-related files exist",
            )
            .with_description(
                "A docker-compose file or .dockerignore was found, but no Dockerfile exists. \
                 This suggests Docker is intended but the Dockerfile may be missing.",
            )
            .with_remediation("Create a Dockerfile to define your container image."),
        );
    }

    Ok(findings)
}

/// DOCKER002: .dockerignore absent when Dockerfile exists
async fn check_dockerignore(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let dockerfiles = find_dockerfiles(scanner);
    let has_dockerfile = !dockerfiles.is_empty();
    let has_dockerignore = scanner.file_exists(".dockerignore");

    if has_dockerfile && !has_dockerignore {
        findings.push(
            Finding::new(
                "DOCKER002",
                "docker",
                Severity::Warning,
                ".dockerignore file is missing",
            )
            .with_description(
                "A Dockerfile exists but no .dockerignore was found. Without a .dockerignore, \
                 unnecessary files may be included in the Docker build context, increasing \
                 image size and potentially leaking sensitive data.",
            )
            .with_remediation(
                "Create a .dockerignore file to exclude unnecessary files from the build context \
                 (e.g., .git, node_modules, .env).",
            ),
        );
    }

    Ok(findings)
}

/// DOCKER003: FROM not pinned (uses :latest or no tag)
async fn check_pinned_tag(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let dockerfiles = find_dockerfiles(scanner);

    for dockerfile_path in &dockerfiles {
        let content = match scanner.read_file(dockerfile_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.to_uppercase().starts_with("FROM ") {
                continue;
            }

            // Parse the FROM instruction: FROM image[:tag] [AS name]
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let image = parts[1];

            // Skip scratch (special Docker image with no tag)
            if image == "scratch" {
                continue;
            }

            // Skip variable references like $BASE_IMAGE or ${BASE_IMAGE}
            if image.starts_with('$') {
                continue;
            }

            // Check if the image has a tag
            let has_tag = if let Some(colon_pos) = image.rfind(':') {
                // Check it's not a port in a registry URL (e.g., registry:5000/image)
                let after_colon = &image[colon_pos + 1..];
                // If after colon contains '/', it's a registry port, not a tag
                !after_colon.contains('/')
            } else {
                false
            };

            if !has_tag {
                findings.push(
                    Finding::new(
                        "DOCKER003",
                        "docker",
                        Severity::Critical,
                        format!("Base image '{}' is not pinned to a specific tag", image),
                    )
                    .with_location(format!("{}:{}", dockerfile_path, line_num + 1))
                    .with_description(
                        "Using an unpinned base image (no tag or implicit :latest) can lead to \
                         non-reproducible builds and unexpected behavior when the upstream \
                         image changes.",
                    )
                    .with_remediation(
                        "Pin the base image to a specific version tag, e.g., 'node:20-alpine' \
                         instead of 'node' or 'node:latest'.",
                    ),
                );
            } else {
                // Check if the tag is 'latest'
                let tag = image.rsplit(':').next().unwrap_or("");
                if tag == "latest" {
                    findings.push(
                        Finding::new(
                            "DOCKER003",
                            "docker",
                            Severity::Critical,
                            format!("Base image '{}' uses the 'latest' tag", image),
                        )
                        .with_location(format!("{}:{}", dockerfile_path, line_num + 1))
                        .with_description(
                            "Using the ':latest' tag is equivalent to not pinning the image. \
                             It can lead to non-reproducible builds.",
                        )
                        .with_remediation(
                            "Pin the base image to a specific version tag, e.g., 'node:20-alpine' \
                             instead of 'node:latest'.",
                        ),
                    );
                }
            }
        }
    }

    Ok(findings)
}

/// DOCKER004: No USER instruction (running as root)
async fn check_user_instruction(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let dockerfiles = find_dockerfiles(scanner);

    for dockerfile_path in &dockerfiles {
        let content = match scanner.read_file(dockerfile_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let has_user = content
            .lines()
            .any(|line| line.trim().to_uppercase().starts_with("USER "));

        if !has_user {
            findings.push(
                Finding::new(
                    "DOCKER004",
                    "docker",
                    Severity::Warning,
                    format!("No USER instruction in {}", dockerfile_path),
                )
                .with_location(dockerfile_path.as_str())
                .with_description(
                    "Without a USER instruction, the container runs as root by default, \
                     which increases the attack surface if the container is compromised.",
                )
                .with_remediation(
                    "Add a USER instruction to run the container as a non-root user, \
                     e.g., 'USER 1001' or 'USER appuser'.",
                ),
            );
        }
    }

    Ok(findings)
}

/// DOCKER005: No HEALTHCHECK instruction
async fn check_healthcheck(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let dockerfiles = find_dockerfiles(scanner);

    for dockerfile_path in &dockerfiles {
        let content = match scanner.read_file(dockerfile_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let has_healthcheck = content
            .lines()
            .any(|line| line.trim().to_uppercase().starts_with("HEALTHCHECK "));

        if !has_healthcheck {
            findings.push(
                Finding::new(
                    "DOCKER005",
                    "docker",
                    Severity::Warning,
                    format!("No HEALTHCHECK instruction in {}", dockerfile_path),
                )
                .with_location(dockerfile_path.as_str())
                .with_description(
                    "Without a HEALTHCHECK instruction, Docker has no way to determine if the \
                     container's main process is still healthy. This limits orchestrator \
                     capabilities for automatic restart and load balancing.",
                )
                .with_remediation(
                    "Add a HEALTHCHECK instruction, e.g., \
                     'HEALTHCHECK CMD curl -f http://localhost/ || exit 1'.",
                ),
            );
        }
    }

    Ok(findings)
}

/// DOCKER006: No multi-stage build (only 1 FROM)
async fn check_multi_stage(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let dockerfiles = find_dockerfiles(scanner);

    for dockerfile_path in &dockerfiles {
        let content = match scanner.read_file(dockerfile_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let from_count = content
            .lines()
            .filter(|line| line.trim().to_uppercase().starts_with("FROM "))
            .count();

        if from_count == 1 {
            findings.push(
                Finding::new(
                    "DOCKER006",
                    "docker",
                    Severity::Info,
                    format!("No multi-stage build in {}", dockerfile_path),
                )
                .with_location(dockerfile_path.as_str())
                .with_description(
                    "The Dockerfile uses a single stage. Multi-stage builds can reduce final \
                     image size by separating build dependencies from runtime dependencies.",
                )
                .with_remediation(
                    "Consider using a multi-stage build to separate build and runtime stages, \
                     resulting in a smaller and more secure final image.",
                ),
            );
        }
    }

    Ok(findings)
}

/// DOCKER007: Secrets in ENV/ARG instructions
async fn check_secrets_in_env(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let secret_patterns = [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "api-key",
        "private_key",
        "access_key",
        "secret_key",
        "credentials",
    ];

    let dockerfiles = find_dockerfiles(scanner);

    for dockerfile_path in &dockerfiles {
        let content = match scanner.read_file(dockerfile_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let upper = trimmed.to_uppercase();

            if !upper.starts_with("ENV ") && !upper.starts_with("ARG ") {
                continue;
            }

            let lower = trimmed.to_lowercase();
            for pattern in &secret_patterns {
                if lower.contains(pattern) {
                    findings.push(
                        Finding::new(
                            "DOCKER007",
                            "docker",
                            Severity::Warning,
                            format!(
                                "Potential secret in {} instruction",
                                if upper.starts_with("ENV ") {
                                    "ENV"
                                } else {
                                    "ARG"
                                }
                            ),
                        )
                        .with_location(format!("{}:{}", dockerfile_path, line_num + 1))
                        .with_description(format!(
                            "The instruction contains '{}' which may indicate a secret or \
                             credential. Secrets in ENV/ARG instructions are baked into \
                             the image layers and can be extracted.",
                            pattern
                        ))
                        .with_remediation(
                            "Use Docker build secrets (--mount=type=secret) or runtime \
                             environment variables instead of embedding secrets in the image.",
                        ),
                    );
                    break; // Only report once per line
                }
            }
        }
    }

    Ok(findings)
}

/// DOCKER008: COPY . . used without .dockerignore
async fn check_copy_all(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let has_dockerignore = scanner.file_exists(".dockerignore");

    let dockerfiles = find_dockerfiles(scanner);

    for dockerfile_path in &dockerfiles {
        let content = match scanner.read_file(dockerfile_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.to_uppercase().starts_with("COPY ") {
                continue;
            }

            // Check for "COPY . ." or "COPY . /" patterns (with optional flags like --chown)
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            // Find the source and dest, skipping flags (start with --)
            let non_flag_parts: Vec<&str> = parts[1..]
                .iter()
                .filter(|p| !p.starts_with("--"))
                .copied()
                .collect();

            if non_flag_parts.len() >= 2 && non_flag_parts[0] == "." && !has_dockerignore {
                findings.push(
                    Finding::new(
                        "DOCKER008",
                        "docker",
                        Severity::Info,
                        "COPY with entire build context used without .dockerignore",
                    )
                    .with_location(format!("{}:{}", dockerfile_path, line_num + 1))
                    .with_description(
                        "Using 'COPY . .' copies the entire build context into the image. \
                         Without a .dockerignore file, this may include unnecessary files \
                         like .git, node_modules, or sensitive data.",
                    )
                    .with_remediation(
                        "Create a .dockerignore file to exclude unnecessary files, or use \
                         more specific COPY instructions.",
                    ),
                );
            }
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::scanner::Scanner;
    use std::fs;
    use tempfile::TempDir;

    // --- DOCKER001: Dockerfile presence ---

    #[tokio::test]
    async fn test_docker001_missing_dockerfile_with_compose() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("docker-compose.yml"), "version: '3'").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dockerfile_presence(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER001"));
    }

    #[tokio::test]
    async fn test_docker001_missing_dockerfile_with_dockerignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join(".dockerignore"), "node_modules\n.git").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dockerfile_presence(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER001"));
    }

    #[tokio::test]
    async fn test_docker001_no_finding_when_dockerfile_exists() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("Dockerfile"), "FROM ubuntu:22.04").unwrap();
        fs::write(root.join("docker-compose.yml"), "version: '3'").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dockerfile_presence(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_docker001_no_finding_when_no_docker_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("README.md"), "# Project").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dockerfile_presence(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- DOCKER002: .dockerignore ---

    #[tokio::test]
    async fn test_docker002_missing_dockerignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("Dockerfile"), "FROM ubuntu:22.04").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dockerignore(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER002"));
    }

    #[tokio::test]
    async fn test_docker002_no_finding_with_dockerignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("Dockerfile"), "FROM ubuntu:22.04").unwrap();
        fs::write(root.join(".dockerignore"), ".git\nnode_modules").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_dockerignore(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- DOCKER003: Pinned tags ---

    #[tokio::test]
    async fn test_docker003_unpinned_from() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("Dockerfile"), "FROM ubuntu\nRUN apt-get update").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pinned_tag(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER003"));
        assert!(findings.iter().any(|f| f.severity == Severity::Critical));
    }

    #[tokio::test]
    async fn test_docker003_latest_tag() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("Dockerfile"), "FROM node:latest\nRUN npm install").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pinned_tag(&scanner).await.unwrap();

        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "DOCKER003" && f.message.contains("latest"))
        );
    }

    #[tokio::test]
    async fn test_docker003_pinned_tag_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM node:20-alpine AS builder\nFROM nginx:1.25-alpine\nCOPY --from=builder /app /usr/share/nginx/html",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pinned_tag(&scanner).await.unwrap();

        assert!(
            findings.is_empty(),
            "Expected no findings for pinned tags, got: {:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_docker003_scratch_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM golang:1.21 AS builder\nRUN go build\nFROM scratch\nCOPY --from=builder /app /app",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pinned_tag(&scanner).await.unwrap();

        // scratch should not trigger, but golang:1.21 is pinned so no findings
        assert!(findings.is_empty());
    }

    // --- DOCKER004: USER instruction ---

    #[tokio::test]
    async fn test_docker004_no_user() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nRUN apt-get update\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_user_instruction(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER004"));
    }

    #[tokio::test]
    async fn test_docker004_has_user() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nRUN useradd -m app\nUSER app\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_user_instruction(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- DOCKER005: HEALTHCHECK ---

    #[tokio::test]
    async fn test_docker005_no_healthcheck() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("Dockerfile"), "FROM ubuntu:22.04\nCMD [\"bash\"]").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_healthcheck(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER005"));
    }

    #[tokio::test]
    async fn test_docker005_has_healthcheck() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nHEALTHCHECK CMD curl -f http://localhost/ || exit 1\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_healthcheck(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- DOCKER006: Multi-stage build ---

    #[tokio::test]
    async fn test_docker006_single_stage() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nRUN apt-get update\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_multi_stage(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER006"));
        assert!(findings.iter().any(|f| f.severity == Severity::Info));
    }

    #[tokio::test]
    async fn test_docker006_multi_stage_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM node:20 AS builder\nRUN npm install\nFROM nginx:1.25\nCOPY --from=builder /app /usr/share/nginx/html",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_multi_stage(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- DOCKER007: Secrets in ENV/ARG ---

    #[tokio::test]
    async fn test_docker007_secret_in_env() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nENV DB_PASSWORD=mysecret\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_secrets_in_env(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER007"));
    }

    #[tokio::test]
    async fn test_docker007_secret_in_arg() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nARG API_KEY=abc123\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_secrets_in_env(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER007"));
    }

    #[tokio::test]
    async fn test_docker007_no_secrets() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nENV APP_PORT=8080\nENV NODE_ENV=production\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_secrets_in_env(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- DOCKER008: COPY . . without .dockerignore ---

    #[tokio::test]
    async fn test_docker008_copy_all_without_dockerignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nCOPY . .\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_copy_all(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOCKER008"));
    }

    #[tokio::test]
    async fn test_docker008_copy_all_with_dockerignore_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nCOPY . .\nCMD [\"bash\"]",
        )
        .unwrap();
        fs::write(root.join(".dockerignore"), ".git\nnode_modules").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_copy_all(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_docker008_specific_copy_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nCOPY src/ /app/src/\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_copy_all(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- Integration: RuleCategory trait ---

    #[tokio::test]
    async fn test_docker_rules_category_name() {
        let rules = DockerRules;
        assert_eq!(rules.name(), "docker");
    }

    #[tokio::test]
    async fn test_docker_rules_run_integration() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a Dockerfile with multiple issues
        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu\nENV SECRET_TOKEN=abc\nCOPY . .\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();
        let rules = DockerRules;

        let findings = rules.run(&scanner, &config).await.unwrap();

        // Should find multiple issues: no dockerignore, unpinned tag, no USER,
        // no HEALTHCHECK, single stage, secrets in ENV, COPY . . without dockerignore
        let rule_ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
        assert!(
            rule_ids.contains(&"DOCKER002"),
            "Should find DOCKER002 (no .dockerignore)"
        );
        assert!(
            rule_ids.contains(&"DOCKER003"),
            "Should find DOCKER003 (unpinned FROM)"
        );
        assert!(
            rule_ids.contains(&"DOCKER004"),
            "Should find DOCKER004 (no USER)"
        );
        assert!(
            rule_ids.contains(&"DOCKER005"),
            "Should find DOCKER005 (no HEALTHCHECK)"
        );
        assert!(
            rule_ids.contains(&"DOCKER006"),
            "Should find DOCKER006 (single stage)"
        );
        assert!(
            rule_ids.contains(&"DOCKER007"),
            "Should find DOCKER007 (secret in ENV)"
        );
        assert!(
            rule_ids.contains(&"DOCKER008"),
            "Should find DOCKER008 (COPY . . without .dockerignore)"
        );
    }

    #[tokio::test]
    async fn test_docker003_copy_with_flags() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("Dockerfile"),
            "FROM ubuntu:22.04\nCOPY --chown=1000:1000 . /app\nCMD [\"bash\"]",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_copy_all(&scanner).await.unwrap();

        // "." is the source and "/app" is the dest - should trigger without .dockerignore
        assert!(findings.iter().any(|f| f.rule_id == "DOCKER008"));
    }
}
