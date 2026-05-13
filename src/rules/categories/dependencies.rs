//! Dependency security rules for checking vulnerabilities using OSV API

use crate::config::Config;
use crate::error::RepoLensError;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct DependencyRules;

#[async_trait::async_trait]
impl RuleCategory for DependencyRules {
    fn name(&self) -> &'static str {
        "dependencies"
    }

    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();
        if config.is_rule_enabled("dependencies/vulnerabilities") {
            findings.extend(check_vulnerabilities(scanner, config).await?);
        }
        if config.is_rule_enabled("dependencies/lock-files") {
            findings.extend(check_lock_files(scanner).await?);
        }
        Ok(findings)
    }
}

/// Lock file mapping for each ecosystem
/// Maps manifest files to their corresponding lock files
struct LockFileMapping {
    manifest: &'static str,
    lock_files: &'static [&'static str],
    ecosystem: &'static str,
}

const LOCK_FILE_MAPPINGS: &[LockFileMapping] = &[
    LockFileMapping {
        manifest: "Cargo.toml",
        lock_files: &["Cargo.lock"],
        ecosystem: "Rust",
    },
    LockFileMapping {
        manifest: "package.json",
        lock_files: &["package-lock.json", "yarn.lock", "pnpm-lock.yaml"],
        ecosystem: "Node.js",
    },
    LockFileMapping {
        manifest: "pyproject.toml",
        lock_files: &["poetry.lock", "uv.lock"],
        ecosystem: "Python",
    },
    LockFileMapping {
        manifest: "Pipfile",
        lock_files: &["Pipfile.lock"],
        ecosystem: "Python (Pipenv)",
    },
    LockFileMapping {
        manifest: "go.mod",
        lock_files: &["go.sum"],
        ecosystem: "Go",
    },
    LockFileMapping {
        manifest: "composer.json",
        lock_files: &["composer.lock"],
        ecosystem: "PHP",
    },
    LockFileMapping {
        manifest: "Gemfile",
        lock_files: &["Gemfile.lock"],
        ecosystem: "Ruby",
    },
    LockFileMapping {
        manifest: "pubspec.yaml",
        lock_files: &["pubspec.lock"],
        ecosystem: "Dart/Flutter",
    },
    LockFileMapping {
        manifest: "Package.swift",
        lock_files: &["Package.resolved"],
        ecosystem: "Swift",
    },
    LockFileMapping {
        manifest: "Podfile",
        lock_files: &["Podfile.lock"],
        ecosystem: "CocoaPods",
    },
];

/// Check for lock files corresponding to detected ecosystems
///
/// Verifies that lock files exist for each detected package manifest.
/// Lock files ensure reproducible builds and protect against supply chain attacks.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing lock files
async fn check_lock_files(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Check standard manifest -> lock file mappings
    for mapping in LOCK_FILE_MAPPINGS {
        if scanner.file_exists(mapping.manifest) {
            let has_lock_file = mapping.lock_files.iter().any(|lf| scanner.file_exists(lf));
            if !has_lock_file {
                let lock_files_str = mapping.lock_files.join(" or ");
                findings.push(
                    Finding::new(
                        "DEP003",
                        "dependencies",
                        Severity::Warning,
                        format!(
                            "Lock file missing for {} ecosystem (expected {})",
                            mapping.ecosystem, lock_files_str
                        ),
                    )
                    .with_location(mapping.manifest)
                    .with_description(
                        "Lock files ensure reproducible builds and protect against supply chain attacks by pinning exact dependency versions."
                    )
                    .with_remediation(format!(
                        "Run your package manager to generate a lock file: {}",
                        get_lock_file_command(mapping.manifest)
                    )),
                );
            }
        }
    }

    // Check for .csproj files -> packages.lock.json
    let csproj_files = scanner.files_matching_pattern("*.csproj");
    if !csproj_files.is_empty() && !scanner.file_exists("packages.lock.json") {
        findings.push(
            Finding::new(
                "DEP003",
                "dependencies",
                Severity::Warning,
                "Lock file missing for .NET ecosystem (expected packages.lock.json)",
            )
            .with_location(
                csproj_files
                    .first()
                    .map(|f| f.path.as_str())
                    .unwrap_or("*.csproj"),
            )
            .with_description(
                "Lock files ensure reproducible builds and protect against supply chain attacks by pinning exact dependency versions."
            )
            .with_remediation(
                "Enable lock file generation: set RestorePackagesWithLockFile to true in your .csproj or run 'dotnet restore --use-lock-file'"
            ),
        );
    }

    Ok(findings)
}

/// Get the command to generate a lock file for a given manifest
fn get_lock_file_command(manifest: &str) -> &'static str {
    match manifest {
        "Cargo.toml" => "cargo build (or cargo generate-lockfile)",
        "package.json" => "npm install (or yarn install, pnpm install)",
        "pyproject.toml" => "poetry lock (or uv lock)",
        "Pipfile" => "pipenv lock",
        "go.mod" => "go mod tidy",
        "composer.json" => "composer install",
        "Gemfile" => "bundle install",
        "pubspec.yaml" => "dart pub get (or flutter pub get)",
        "Package.swift" => "swift package resolve",
        "Podfile" => "pod install",
        _ => "run your package manager's install command",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Cargo,
    Npm,
    PyPI,
    Go,
    Maven,
    Packagist,
    NuGet,     // .NET - OSV supported
    RubyGems,  // Ruby - OSV supported
    CocoaPods, // iOS - NOT in OSV
    SwiftPM,   // Swift - NOT in OSV
    Pub,       // Dart/Flutter - OSV supported
}

impl Ecosystem {
    pub fn as_str(&self) -> &'static str {
        match self {
            Ecosystem::Cargo => "crates.io",
            Ecosystem::Npm => "npm",
            Ecosystem::PyPI => "PyPI",
            Ecosystem::Go => "Go",
            Ecosystem::Maven => "Maven",
            Ecosystem::Packagist => "Packagist",
            Ecosystem::NuGet => "NuGet",
            Ecosystem::RubyGems => "RubyGems",
            Ecosystem::CocoaPods => "CocoaPods",
            Ecosystem::SwiftPM => "SwiftPM",
            Ecosystem::Pub => "Pub",
        }
    }

    /// Check if the ecosystem is supported by the OSV vulnerability database
    pub fn is_osv_supported(&self) -> bool {
        !matches!(self, Ecosystem::CocoaPods | Ecosystem::SwiftPM)
    }
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
}

#[derive(Debug, Clone, Serialize)]
struct OsvQuery {
    package: OsvPackage,
    version: String,
}
#[derive(Debug, Clone, Serialize)]
struct OsvPackage {
    name: String,
    ecosystem: String,
}
#[derive(Debug, Serialize)]
struct OsvBatchQuery {
    queries: Vec<OsvQuery>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OsvVulnerability {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub severity: Vec<OsvSeverity>,
    #[serde(default)]
    pub affected: Vec<OsvAffected>,
    #[serde(default)]
    #[allow(dead_code)]
    pub references: Vec<OsvReference>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OsvSeverity {
    #[serde(rename = "type")]
    pub severity_type: String,
    pub score: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct OsvAffected {
    pub package: Option<OsvAffectedPackage>,
    #[serde(default)]
    pub ranges: Vec<OsvRange>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct OsvAffectedPackage {
    pub name: String,
    #[allow(dead_code)]
    pub ecosystem: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct OsvRange {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub range_type: String,
    #[serde(default)]
    pub events: Vec<OsvEvent>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct OsvEvent {
    #[serde(default)]
    #[allow(dead_code)]
    pub introduced: Option<String>,
    #[serde(default)]
    pub fixed: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct OsvReference {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct OsvBatchResponse {
    results: Vec<OsvBatchResult>,
}
#[derive(Debug, Deserialize)]
struct OsvBatchResult {
    #[serde(default)]
    vulns: Vec<OsvVulnerability>,
}

// GitHub Advisory Database structures
#[derive(Debug, Clone)]
struct GitHubAdvisory {
    pub id: String,
    pub summary: Option<String>,
    pub cvss_score: Option<f64>,
    pub fixed_version: Option<String>,
}

async fn check_vulnerabilities(
    scanner: &Scanner,
    _config: &Config,
) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();
    let mut all_deps: Vec<Dependency> = Vec::new();
    if let Ok(deps) = parse_cargo_lock(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_package_lock(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_requirements_txt(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_go_sum(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_pom_xml(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_gradle_build(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_composer_lock(scanner) {
        all_deps.extend(deps);
    } else if let Ok(deps) = parse_composer_json(scanner) {
        all_deps.extend(deps);
    }
    // New ecosystems
    if let Ok(deps) = parse_nuget_lock(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_gemfile_lock(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_podfile_lock(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_package_resolved(scanner) {
        all_deps.extend(deps);
    }
    if let Ok(deps) = parse_pubspec_lock(scanner) {
        all_deps.extend(deps);
    }
    if all_deps.is_empty() {
        return Ok(findings);
    }

    // Check for ecosystems not supported by OSV and add info findings
    let unsupported_ecosystems: Vec<_> = all_deps
        .iter()
        .filter(|d| !d.ecosystem.is_osv_supported())
        .collect();
    if !unsupported_ecosystems.is_empty() {
        let mut seen_ecosystems = std::collections::HashSet::new();
        for dep in &unsupported_ecosystems {
            if seen_ecosystems.insert(dep.ecosystem) {
                let ecosystem_name = dep.ecosystem.as_str();
                let lock_file = get_ecosystem_lock_file(dep.ecosystem);
                findings.push(
                    Finding::new(
                        format!("DEP004-{}", ecosystem_name),
                        "dependencies",
                        Severity::Info,
                        format!(
                            "{} dependencies detected but vulnerability scanning is not available",
                            ecosystem_name
                        ),
                    )
                    .with_description(format!(
                        "The {} ecosystem is not supported by the OSV vulnerability database. \
                         Dependencies from {} cannot be checked for known vulnerabilities.",
                        ecosystem_name, lock_file
                    ))
                    .with_location(lock_file),
                );
            }
        }
    }

    // Filter to only OSV-supported ecosystems for vulnerability queries
    let osv_supported_deps: Vec<_> = all_deps
        .iter()
        .filter(|d| d.ecosystem.is_osv_supported())
        .cloned()
        .collect();

    // Use a HashSet to deduplicate vulnerabilities by ID
    use std::collections::HashSet;
    let mut seen_vuln_ids: HashSet<String> = HashSet::new();

    // Query OSV API (only for supported ecosystems)
    match query_osv_batch(&osv_supported_deps).await {
        Ok(vulns) => {
            for (dep, vuln_list) in vulns {
                for vuln in vuln_list {
                    if seen_vuln_ids.contains(&vuln.id) {
                        continue; // Skip duplicates
                    }
                    seen_vuln_ids.insert(vuln.id.clone());

                    let cvss_score = extract_cvss_score(&vuln);
                    let sev = determine_severity(&vuln);
                    let fixed = get_fixed_version(&vuln, &dep);

                    let message = if let Some(score) = cvss_score {
                        format!(
                            "Vulnerability {} (CVSS: {}) found in {} {}",
                            vuln.id, score, dep.name, dep.version
                        )
                    } else {
                        format!(
                            "Vulnerability {} found in {} {}",
                            vuln.id, dep.name, dep.version
                        )
                    };

                    let mut f =
                        Finding::new(format!("DEP001-{}", vuln.id), "dependencies", sev, message);
                    if let Some(s) = &vuln.summary {
                        f = f.with_description(s.clone());
                    } else if let Some(d) = &vuln.details {
                        f = f.with_description(d.chars().take(500).collect::<String>());
                    }
                    let rem = if let Some(fix) = fixed {
                        format!(
                            "Upgrade {} to version {} or later. Aliases: {}",
                            dep.name,
                            fix,
                            vuln.aliases.join(", ")
                        )
                    } else {
                        format!(
                            "Check for updates to {}. Vulnerability: {}. Aliases: {}",
                            dep.name,
                            vuln.id,
                            vuln.aliases.join(", ")
                        )
                    };
                    f = f.with_remediation(rem);
                    let loc = get_ecosystem_lock_file(dep.ecosystem);
                    f = f.with_location(loc);
                    findings.push(f);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to query OSV API: {}", e);
        }
    }

    // Query GitHub Advisory Database (only for supported ecosystems)
    match query_github_advisories(&osv_supported_deps).await {
        Ok(vulns) => {
            for (dep, vuln_list) in vulns {
                for vuln in vuln_list {
                    if seen_vuln_ids.contains(&vuln.id) {
                        continue; // Skip duplicates
                    }
                    seen_vuln_ids.insert(vuln.id.clone());

                    let cvss_score = vuln.cvss_score;
                    let sev = determine_severity_from_cvss(cvss_score);

                    let message = if let Some(score) = cvss_score {
                        format!(
                            "Vulnerability {} (CVSS: {}) found in {} {}",
                            vuln.id, score, dep.name, dep.version
                        )
                    } else {
                        format!(
                            "Vulnerability {} found in {} {}",
                            vuln.id, dep.name, dep.version
                        )
                    };

                    let mut f =
                        Finding::new(format!("DEP002-{}", vuln.id), "dependencies", sev, message);
                    if let Some(desc) = &vuln.summary {
                        f = f.with_description(desc.clone());
                    }
                    if let Some(fix) = &vuln.fixed_version {
                        f = f.with_remediation(format!(
                            "Upgrade {} to version {} or later.",
                            dep.name, fix
                        ));
                    } else {
                        f = f.with_remediation(format!(
                            "Check for updates to {}. Vulnerability: {}.",
                            dep.name, vuln.id
                        ));
                    }
                    let loc = get_ecosystem_lock_file(dep.ecosystem);
                    f = f.with_location(loc);
                    findings.push(f);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to query GitHub Advisories: {}", e);
        }
    }

    // If both sources failed, add a warning (only for OSV-supported deps)
    if findings.is_empty() && !osv_supported_deps.is_empty() {
        findings.push(
            Finding::new(
                "DEP000",
                "dependencies",
                Severity::Warning,
                "Could not check dependencies for vulnerabilities",
            )
            .with_description(
                "Failed to query vulnerability databases. Please check your network connection."
                    .to_string(),
            ),
        );
    }

    Ok(findings)
}

/// Get the lock file path for an ecosystem
fn get_ecosystem_lock_file(ecosystem: Ecosystem) -> &'static str {
    match ecosystem {
        Ecosystem::Cargo => "Cargo.lock",
        Ecosystem::Npm => "package-lock.json",
        Ecosystem::PyPI => "requirements.txt",
        Ecosystem::Go => "go.sum",
        Ecosystem::Maven => "pom.xml",
        Ecosystem::Packagist => "composer.lock",
        Ecosystem::NuGet => "packages.lock.json",
        Ecosystem::RubyGems => "Gemfile.lock",
        Ecosystem::CocoaPods => "Podfile.lock",
        Ecosystem::SwiftPM => "Package.resolved",
        Ecosystem::Pub => "pubspec.lock",
    }
}

pub fn parse_cargo_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("Cargo.lock") {
        return Ok(deps);
    }
    let content = scanner.read_file("Cargo.lock").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "Cargo.lock".to_string(),
            source: e,
        })
    })?;
    let lock: toml::Value = toml::from_str(&content)?;
    if let Some(packages) = lock.get("package").and_then(|p| p.as_array()) {
        for pkg in packages {
            if let (Some(n), Some(v)) = (
                pkg.get("name").and_then(|n| n.as_str()),
                pkg.get("version").and_then(|v| v.as_str()),
            ) {
                deps.push(Dependency {
                    name: n.to_string(),
                    version: v.to_string(),
                    ecosystem: Ecosystem::Cargo,
                });
            }
        }
    }
    Ok(deps)
}

pub fn parse_package_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("package-lock.json") {
        return Ok(deps);
    }
    let content = scanner.read_file("package-lock.json").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "package-lock.json".to_string(),
            source: e,
        })
    })?;
    let lock: serde_json::Value = serde_json::from_str(&content)?;
    if let Some(packages) = lock.get("packages").and_then(|p| p.as_object()) {
        for (path, info) in packages {
            if path.is_empty() {
                continue;
            }
            let name = path.strip_prefix("node_modules/").unwrap_or(path);
            let name = if name.contains("/node_modules/") {
                name.split("/node_modules/").last().unwrap_or(name)
            } else {
                name
            };
            if let Some(v) = info.get("version").and_then(|v| v.as_str()) {
                deps.push(Dependency {
                    name: name.to_string(),
                    version: v.to_string(),
                    ecosystem: Ecosystem::Npm,
                });
            }
        }
    } else if let Some(d) = lock.get("dependencies").and_then(|d| d.as_object()) {
        parse_npm_deps(d, &mut deps);
    }
    Ok(deps)
}

fn parse_npm_deps(d: &serde_json::Map<String, serde_json::Value>, deps: &mut Vec<Dependency>) {
    for (n, i) in d {
        if let Some(v) = i.get("version").and_then(|v| v.as_str()) {
            deps.push(Dependency {
                name: n.clone(),
                version: v.to_string(),
                ecosystem: Ecosystem::Npm,
            });
        }
        if let Some(nested) = i.get("dependencies").and_then(|d| d.as_object()) {
            parse_npm_deps(nested, deps);
        }
    }
}

pub fn parse_requirements_txt(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    for f in [
        "requirements.txt",
        "requirements-dev.txt",
        "requirements/base.txt",
        "requirements/production.txt",
    ] {
        if !scanner.file_exists(f) {
            continue;
        }
        let content = scanner.read_file(f).map_err(|e| {
            RepoLensError::Scan(crate::error::ScanError::FileRead {
                path: f.to_string(),
                source: e,
            })
        })?;
        for line in content.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') || l.starts_with('-') {
                continue;
            }
            if let Some((n, v)) = parse_pip_req(l) {
                deps.push(Dependency {
                    name: n,
                    version: v,
                    ecosystem: Ecosystem::PyPI,
                });
            }
        }
    }
    Ok(deps)
}

fn parse_pip_req(line: &str) -> Option<(String, String)> {
    let l = line.split(';').next()?.trim().split('#').next()?.trim();
    let l = if let Some(p) = l.find('[') {
        let e = l.find(']')?;
        format!("{}{}", &l[..p], &l[e + 1..])
    } else {
        l.to_string()
    };
    for sep in ["==", ">=", "<=", "~=", "!=", ">", "<"] {
        if let Some(p) = l.find(sep) {
            let n = l[..p].trim().to_lowercase();
            let v = l[p + sep.len()..]
                .trim()
                .split(',')
                .next()?
                .trim()
                .to_string();
            if !n.is_empty() && !v.is_empty() {
                return Some((n, v));
            }
        }
    }
    None
}

pub fn parse_go_sum(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    let mut seen: HashMap<String, bool> = HashMap::new();
    if !scanner.file_exists("go.sum") {
        return Ok(deps);
    }
    let content = scanner.read_file("go.sum").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "go.sum".to_string(),
            source: e,
        })
    })?;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let module = parts[0];
        let version = parts[1].trim_end_matches("/go.mod");
        let key = format!("{}@{}", module, version);
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key, true);
        let v = version.strip_prefix('v').unwrap_or(version);
        let v = if v.contains('-') {
            v.split('-').next().unwrap_or(v)
        } else {
            v
        };
        deps.push(Dependency {
            name: module.to_string(),
            version: v.to_string(),
            ecosystem: Ecosystem::Go,
        });
    }
    Ok(deps)
}

pub fn parse_pom_xml(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("pom.xml") {
        return Ok(deps);
    }
    let content = scanner.read_file("pom.xml").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "pom.xml".to_string(),
            source: e,
        })
    })?;

    // Remove <dependencyManagement> sections to avoid picking up managed (non-direct) deps
    let mgmt_re = Regex::new(r"(?s)<dependencyManagement>.*?</dependencyManagement>").unwrap();
    let content = mgmt_re.replace_all(&content, "");

    // Extract the <dependencies> block(s)
    let deps_block_re = Regex::new(r"(?s)<dependencies>(.*?)</dependencies>").unwrap();
    let dep_re = Regex::new(r"(?s)<dependency>(.*?)</dependency>").unwrap();
    let group_re = Regex::new(r"<groupId>\s*([^<]+?)\s*</groupId>").unwrap();
    let artifact_re = Regex::new(r"<artifactId>\s*([^<]+?)\s*</artifactId>").unwrap();
    let version_re = Regex::new(r"<version>\s*([^<]+?)\s*</version>").unwrap();

    for block_cap in deps_block_re.captures_iter(&content) {
        let block = &block_cap[1];
        for dep_cap in dep_re.captures_iter(block) {
            let dep_content = &dep_cap[1];
            let group = group_re.captures(dep_content).map(|c| c[1].to_string());
            let artifact = artifact_re.captures(dep_content).map(|c| c[1].to_string());
            let version = version_re.captures(dep_content).map(|c| c[1].to_string());

            if let (Some(g), Some(a)) = (group, artifact) {
                let ver = match version {
                    Some(v) if !v.starts_with("${") => v,
                    _ => continue,
                };
                deps.push(Dependency {
                    name: format!("{}:{}", g, a),
                    version: ver,
                    ecosystem: Ecosystem::Maven,
                });
            }
        }
    }
    Ok(deps)
}

pub fn parse_gradle_build(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    let gradle_file = if scanner.file_exists("build.gradle.kts") {
        "build.gradle.kts"
    } else if scanner.file_exists("build.gradle") {
        "build.gradle"
    } else {
        return Ok(deps);
    };
    let content = scanner.read_file(gradle_file).map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: gradle_file.to_string(),
            source: e,
        })
    })?;

    let re = Regex::new(
        r#"(?:implementation|api|compile|testImplementation|runtimeOnly|compileOnly)\s*[\(]?\s*['"]([^'"]+)['"]"#,
    )
    .unwrap();

    for cap in re.captures_iter(&content) {
        let coord = &cap[1];
        let parts: Vec<&str> = coord.split(':').collect();
        if parts.len() >= 3 {
            let name = format!("{}:{}", parts[0], parts[1]);
            let version = parts[2].to_string();
            deps.push(Dependency {
                name,
                version,
                ecosystem: Ecosystem::Maven,
            });
        }
    }
    Ok(deps)
}

pub fn parse_composer_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("composer.lock") {
        return Ok(deps);
    }
    let content = scanner.read_file("composer.lock").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "composer.lock".to_string(),
            source: e,
        })
    })?;
    let lock: serde_json::Value = serde_json::from_str(&content)?;

    for key in &["packages", "packages-dev"] {
        if let Some(packages) = lock.get(*key).and_then(|p| p.as_array()) {
            for pkg in packages {
                if let (Some(name), Some(version)) = (
                    pkg.get("name").and_then(|n| n.as_str()),
                    pkg.get("version").and_then(|v| v.as_str()),
                ) {
                    let version = version.strip_prefix('v').unwrap_or(version);
                    deps.push(Dependency {
                        name: name.to_string(),
                        version: version.to_string(),
                        ecosystem: Ecosystem::Packagist,
                    });
                }
            }
        }
    }
    Ok(deps)
}

pub fn parse_composer_json(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("composer.json") {
        return Ok(deps);
    }
    let content = scanner.read_file("composer.json").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "composer.json".to_string(),
            source: e,
        })
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&content)?;

    for section in &["require", "require-dev"] {
        if let Some(reqs) = parsed.get(*section).and_then(|r| r.as_object()) {
            for (name, version_val) in reqs {
                // Skip php itself and extension requirements
                if name == "php" || name.starts_with("ext-") {
                    continue;
                }
                if let Some(version_str) = version_val.as_str() {
                    let version = version_str
                        .trim_start_matches('^')
                        .trim_start_matches('~')
                        .trim_start_matches(">=")
                        .trim_start_matches("<=")
                        .trim_start_matches('>')
                        .trim_start_matches('<')
                        .trim_start_matches('v')
                        .split(',')
                        .next()
                        .unwrap_or(version_str)
                        .trim()
                        .to_string();
                    deps.push(Dependency {
                        name: name.clone(),
                        version,
                        ecosystem: Ecosystem::Packagist,
                    });
                }
            }
        }
    }
    Ok(deps)
}

/// Parse NuGet packages.lock.json for .NET projects
/// Falls back to parsing *.csproj files if lock file not found
pub fn parse_nuget_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();

    // Try packages.lock.json first (preferred)
    if scanner.file_exists("packages.lock.json") {
        let content = scanner.read_file("packages.lock.json").map_err(|e| {
            RepoLensError::Scan(crate::error::ScanError::FileRead {
                path: "packages.lock.json".to_string(),
                source: e,
            })
        })?;
        let lock: serde_json::Value = serde_json::from_str(&content)?;

        // packages.lock.json format: { "version": 1, "dependencies": { "targetFramework": { "packageName": { "resolved": "version" } } } }
        if let Some(dependencies) = lock.get("dependencies").and_then(|d| d.as_object()) {
            for (_framework, packages) in dependencies {
                if let Some(packages_obj) = packages.as_object() {
                    for (name, info) in packages_obj {
                        if let Some(resolved) = info.get("resolved").and_then(|v| v.as_str()) {
                            deps.push(Dependency {
                                name: name.clone(),
                                version: resolved.to_string(),
                                ecosystem: Ecosystem::NuGet,
                            });
                        }
                    }
                }
            }
        }
        return Ok(deps);
    }

    // Fallback to parsing *.csproj files
    let csproj_files = scanner.files_matching_pattern("*.csproj");
    // Pre-compile regexes outside the loop
    let re = Regex::new(r#"<PackageReference\s+Include\s*=\s*"([^"]+)"\s+Version\s*=\s*"([^"]+)""#)
        .unwrap();
    let re_alt = Regex::new(
        r#"<PackageReference\s+Include\s*=\s*"([^"]+)"[^>]*>\s*<Version>([^<]+)</Version>"#,
    )
    .unwrap();
    for file in csproj_files {
        if let Ok(content) = scanner.read_file(&file.path) {
            // Match PackageReference elements: <PackageReference Include="Name" Version="Version" />
            for cap in re.captures_iter(&content) {
                let name = cap[1].to_string();
                let version = cap[2].to_string();
                // Skip version placeholders
                if !version.starts_with("$(") {
                    deps.push(Dependency {
                        name,
                        version,
                        ecosystem: Ecosystem::NuGet,
                    });
                }
            }

            // Also match the alternative format: <PackageReference Include="Name"><Version>Version</Version></PackageReference>
            for cap in re_alt.captures_iter(&content) {
                let name = cap[1].to_string();
                let version = cap[2].to_string();
                if !version.starts_with("$(") {
                    deps.push(Dependency {
                        name,
                        version,
                        ecosystem: Ecosystem::NuGet,
                    });
                }
            }
        }
    }
    Ok(deps)
}

/// Parse Gemfile.lock for Ruby projects
pub fn parse_gemfile_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("Gemfile.lock") {
        return Ok(deps);
    }
    let content = scanner.read_file("Gemfile.lock").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "Gemfile.lock".to_string(),
            source: e,
        })
    })?;

    // Gemfile.lock format has specs section with indented gem entries:
    // GEM
    //   specs:
    //     gem-name (1.2.3)
    //       dependency1 (~> 1.0)
    let mut in_specs = false;
    let gem_re = Regex::new(r"^\s{4}([a-zA-Z0-9_-]+)\s+\(([^)]+)\)$").unwrap();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "specs:" {
            in_specs = true;
            continue;
        }

        // Exit specs section when we hit a non-indented line (or empty line followed by section header)
        if in_specs && !line.starts_with(' ') && !trimmed.is_empty() {
            in_specs = false;
        }

        if in_specs {
            // Match gem lines (4 spaces indent, gem-name (version))
            if let Some(cap) = gem_re.captures(line) {
                let name = cap[1].to_string();
                let version = cap[2].to_string();
                deps.push(Dependency {
                    name,
                    version,
                    ecosystem: Ecosystem::RubyGems,
                });
            }
        }
    }
    Ok(deps)
}

/// Parse Podfile.lock for iOS/CocoaPods projects
pub fn parse_podfile_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("Podfile.lock") {
        return Ok(deps);
    }
    let content = scanner.read_file("Podfile.lock").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "Podfile.lock".to_string(),
            source: e,
        })
    })?;

    // Parse YAML - Podfile.lock format:
    // PODS:
    //   - PodName (1.2.3):
    //     - Dependency
    //   - AnotherPod (2.0.0)
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "Podfile.lock".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        })
    })?;

    if let Some(pods) = parsed.get("PODS").and_then(|p| p.as_sequence()) {
        for pod in pods {
            // Each pod can be a string "Name (version)" or a mapping with the name as key
            let pod_str = if let Some(s) = pod.as_str() {
                s.to_string()
            } else if let Some(mapping) = pod.as_mapping() {
                // Get the first key which is the pod name with version
                mapping
                    .keys()
                    .next()
                    .and_then(|k| k.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            } else {
                continue;
            };

            // Parse "PodName (1.2.3)" format
            if let Some(paren_pos) = pod_str.find('(') {
                if let Some(end_pos) = pod_str.find(')') {
                    let name = pod_str[..paren_pos].trim().to_string();
                    let version = pod_str[paren_pos + 1..end_pos].trim().to_string();
                    if !name.is_empty() && !version.is_empty() {
                        deps.push(Dependency {
                            name,
                            version,
                            ecosystem: Ecosystem::CocoaPods,
                        });
                    }
                }
            }
        }
    }
    Ok(deps)
}

/// Parse Package.resolved for Swift Package Manager projects
pub fn parse_package_resolved(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();

    // Package.resolved can be at root or in Package.swift directory
    let resolved_paths = ["Package.resolved", ".package.resolved"];
    let mut content = None;
    let mut found_path = "";

    for path in resolved_paths {
        if scanner.file_exists(path) {
            content = scanner.read_file(path).ok();
            found_path = path;
            break;
        }
    }

    let content = match content {
        Some(c) => c,
        None => return Ok(deps),
    };

    let resolved: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: found_path.to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        })
    })?;

    // Package.resolved format v2 (Swift 5.6+):
    // { "pins": [ { "identity": "name", "state": { "version": "1.0.0" } } ] }
    // Format v1:
    // { "object": { "pins": [ { "package": "name", "state": { "version": "1.0.0" } } ] } }

    // Try v2 format first
    if let Some(pins) = resolved.get("pins").and_then(|p| p.as_array()) {
        for pin in pins {
            let name = pin
                .get("identity")
                .and_then(|i| i.as_str())
                .map(|s| s.to_string());
            let version = pin
                .get("state")
                .and_then(|s| s.get("version"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if let (Some(name), Some(version)) = (name, version) {
                deps.push(Dependency {
                    name,
                    version,
                    ecosystem: Ecosystem::SwiftPM,
                });
            }
        }
    }
    // Try v1 format
    else if let Some(pins) = resolved
        .get("object")
        .and_then(|o| o.get("pins"))
        .and_then(|p| p.as_array())
    {
        for pin in pins {
            let name = pin
                .get("package")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());
            let version = pin
                .get("state")
                .and_then(|s| s.get("version"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if let (Some(name), Some(version)) = (name, version) {
                deps.push(Dependency {
                    name,
                    version,
                    ecosystem: Ecosystem::SwiftPM,
                });
            }
        }
    }

    Ok(deps)
}

/// Parse pubspec.lock for Dart/Flutter projects
pub fn parse_pubspec_lock(scanner: &Scanner) -> Result<Vec<Dependency>, RepoLensError> {
    let mut deps = Vec::new();
    if !scanner.file_exists("pubspec.lock") {
        return Ok(deps);
    }
    let content = scanner.read_file("pubspec.lock").map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "pubspec.lock".to_string(),
            source: e,
        })
    })?;

    // Parse YAML - pubspec.lock format:
    // packages:
    //   package_name:
    //     dependency: "direct main"
    //     version: "1.2.3"
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        RepoLensError::Scan(crate::error::ScanError::FileRead {
            path: "pubspec.lock".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        })
    })?;

    if let Some(packages) = parsed.get("packages").and_then(|p| p.as_mapping()) {
        for (name, info) in packages {
            let name = match name.as_str() {
                Some(n) => n.to_string(),
                None => continue,
            };

            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.trim_matches('"').to_string());

            if let Some(version) = version {
                deps.push(Dependency {
                    name,
                    version,
                    ecosystem: Ecosystem::Pub,
                });
            }
        }
    }
    Ok(deps)
}

async fn query_osv_batch(
    deps: &[Dependency],
) -> Result<Vec<(Dependency, Vec<OsvVulnerability>)>, String> {
    if deps.is_empty() {
        return Ok(Vec::new());
    }

    // Filter out dependencies with empty names or versions
    let valid_deps: Vec<&Dependency> = deps
        .iter()
        .filter(|d| !d.name.trim().is_empty() && !d.version.trim().is_empty())
        .collect();

    if valid_deps.is_empty() {
        return Ok(Vec::new());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let queries: Vec<OsvQuery> = valid_deps
        .iter()
        .map(|d| OsvQuery {
            package: OsvPackage {
                name: d.name.clone(),
                ecosystem: d.ecosystem.as_str().to_string(),
            },
            version: d.version.clone(),
        })
        .collect();
    let resp = client
        .post("https://api.osv.dev/v1/querybatch")
        .json(&OsvBatchQuery {
            queries: queries.clone(),
        })
        .send()
        .await
        .map_err(|e| format!("HTTP failed: {}", e))?;
    if !resp.status().is_success() {
        // Log the first few packages for debugging
        let sample: Vec<String> = queries
            .iter()
            .take(3)
            .map(|q| format!("{}@{} ({})", q.package.name, q.version, q.package.ecosystem))
            .collect();
        tracing::debug!(
            "OSV API returned {}: sample packages: {:?}",
            resp.status(),
            sample
        );
        return Err(format!("OSV API status: {}", resp.status()));
    }
    let batch: OsvBatchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    let mut results = Vec::new();
    for (i, r) in batch.results.into_iter().enumerate() {
        if !r.vulns.is_empty() {
            if let Some(d) = valid_deps.get(i) {
                results.push(((*d).clone(), r.vulns));
            }
        }
    }
    Ok(results)
}

async fn query_github_advisories(
    deps: &[Dependency],
) -> Result<Vec<(Dependency, Vec<GitHubAdvisory>)>, String> {
    if deps.is_empty() {
        return Ok(Vec::new());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("repolens/1.0.0")
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    // Query GitHub Security Advisories API
    // Note: This uses a simplified approach. For production, consider using
    // GitHub's GraphQL API with authentication for better rate limits and more data.

    // Query each dependency individually (GitHub doesn't have a batch endpoint like OSV)
    for dep in deps {
        // Map ecosystem to GitHub's ecosystem names
        let ecosystem = match dep.ecosystem {
            Ecosystem::Cargo => "rust",
            Ecosystem::Npm => "npm",
            Ecosystem::PyPI => "pip",
            Ecosystem::Go => "go",
            Ecosystem::Maven => "maven",
            Ecosystem::Packagist => "composer",
            Ecosystem::NuGet => "nuget",
            Ecosystem::RubyGems => "rubygems",
            Ecosystem::Pub => "pub",
            // CocoaPods and SwiftPM are not supported by GitHub Advisory DB either
            Ecosystem::CocoaPods | Ecosystem::SwiftPM => continue,
        };

        // Use GitHub's REST API for security advisories
        // Format: https://api.github.com/advisories?ecosystem={ecosystem}&package={package}
        let url = format!(
            "https://api.github.com/advisories?ecosystem={}&package={}",
            ecosystem, dep.name
        );

        match client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(advisories_json) = resp.json::<Vec<serde_json::Value>>().await {
                        let mut vulns = Vec::new();
                        for adv in advisories_json {
                            if let Some(id) = adv.get("ghsa_id").and_then(|v| v.as_str()) {
                                let summary = adv
                                    .get("summary")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                // Extract CVSS score if available
                                let cvss_score = adv
                                    .get("cvss")
                                    .and_then(|v| v.get("score"))
                                    .and_then(|v| v.as_f64());

                                vulns.push(GitHubAdvisory {
                                    id: id.to_string(),
                                    summary,
                                    cvss_score,
                                    fixed_version: None, // GitHub API doesn't provide this directly
                                });
                            }
                        }
                        if !vulns.is_empty() {
                            results.push((dep.clone(), vulns));
                        }
                    }
                } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
                    // Package not found in GitHub advisories, continue
                    continue;
                }
            }
            Err(e) => {
                tracing::debug!("Failed to query GitHub Advisory for {}: {}", dep.name, e);
                // Continue with other dependencies
            }
        }
    }

    Ok(results)
}

fn extract_cvss_score(vuln: &OsvVulnerability) -> Option<String> {
    for s in &vuln.severity {
        if s.severity_type == "CVSS_V3" || s.severity_type == "CVSS_V2" {
            if let Ok(score) = s.score.parse::<f64>() {
                return Some(format!("{:.1}", score));
            }
            if s.score.starts_with("CVSS:") {
                return Some(s.score.clone());
            }
        }
    }
    None
}

fn determine_severity(vuln: &OsvVulnerability) -> Severity {
    for s in &vuln.severity {
        if s.severity_type == "CVSS_V3" || s.severity_type == "CVSS_V2" {
            if let Ok(score) = s.score.parse::<f64>() {
                if score >= 7.0 {
                    return Severity::Critical;
                } else if score >= 4.0 {
                    return Severity::Warning;
                } else {
                    return Severity::Info;
                }
            }
            if s.score.starts_with("CVSS:") {
                return Severity::Warning;
            }
        }
    }
    if vuln.id.starts_with("GHSA-") || vuln.id.starts_with("CVE-") {
        Severity::Warning
    } else {
        Severity::Info
    }
}

fn determine_severity_from_cvss(cvss_score: Option<f64>) -> Severity {
    if let Some(score) = cvss_score {
        if score >= 7.0 {
            Severity::Critical
        } else if score >= 4.0 {
            Severity::Warning
        } else {
            Severity::Info
        }
    } else {
        Severity::Warning
    }
}

fn get_fixed_version(vuln: &OsvVulnerability, dep: &Dependency) -> Option<String> {
    for a in &vuln.affected {
        if let Some(p) = &a.package {
            if p.name != dep.name {
                continue;
            }
        }
        for r in &a.ranges {
            for e in &r.events {
                if let Some(f) = &e.fixed {
                    return Some(f.clone());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_cargo_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.lock"),
            "[[package]]\nname = \"serde\"\nversion = \"1.0.130\"",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_cargo_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 1);
        assert!(
            deps.iter()
                .any(|d| d.name == "serde" && d.version == "1.0.130")
        );
    }

    #[test]
    fn test_ecosystem_as_str() {
        assert_eq!(Ecosystem::Cargo.as_str(), "crates.io");
        assert_eq!(Ecosystem::Npm.as_str(), "npm");
    }

    #[test]
    fn test_determine_severity() {
        let vuln = OsvVulnerability {
            id: "GHSA-test".to_string(),
            summary: None,
            details: None,
            aliases: vec![],
            severity: vec![OsvSeverity {
                severity_type: "CVSS_V3".to_string(),
                score: "9.8".to_string(),
            }],
            affected: vec![],
            references: vec![],
        };
        assert_eq!(determine_severity(&vuln), Severity::Critical);
    }

    #[tokio::test]
    async fn test_check_vulnerabilities_no_lock_files() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let config = Config::default();
        let findings = check_vulnerabilities(&scanner, &config).await.unwrap();
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_dependency_rules_category_name() {
        let rules = DependencyRules;
        assert_eq!(rules.name(), "dependencies");
    }

    #[test]
    fn test_extract_cvss_score() {
        let vuln = OsvVulnerability {
            id: "CVE-2023-1234".to_string(),
            summary: None,
            details: None,
            aliases: vec![],
            severity: vec![OsvSeverity {
                severity_type: "CVSS_V3".to_string(),
                score: "9.8".to_string(),
            }],
            affected: vec![],
            references: vec![],
        };
        assert_eq!(extract_cvss_score(&vuln), Some("9.8".to_string()));

        let vuln_no_cvss = OsvVulnerability {
            id: "GHSA-xxxx".to_string(),
            summary: None,
            details: None,
            aliases: vec![],
            severity: vec![],
            affected: vec![],
            references: vec![],
        };
        assert_eq!(extract_cvss_score(&vuln_no_cvss), None);
    }

    #[test]
    fn test_determine_severity_from_cvss() {
        use crate::rules::Severity;
        assert_eq!(determine_severity_from_cvss(Some(9.8)), Severity::Critical);
        assert_eq!(determine_severity_from_cvss(Some(5.0)), Severity::Warning);
        assert_eq!(determine_severity_from_cvss(Some(3.0)), Severity::Info);
        assert_eq!(determine_severity_from_cvss(None), Severity::Warning);
    }

    #[test]
    fn test_get_fixed_version() {
        let dep = Dependency {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            ecosystem: Ecosystem::Cargo,
        };

        let vuln_with_fix = OsvVulnerability {
            id: "CVE-2023-1234".to_string(),
            summary: None,
            details: None,
            aliases: vec![],
            severity: vec![],
            affected: vec![OsvAffected {
                package: Some(OsvAffectedPackage {
                    name: "test-package".to_string(),
                    ecosystem: "crates.io".to_string(),
                }),
                ranges: vec![OsvRange {
                    range_type: "SEMVER".to_string(),
                    events: vec![OsvEvent {
                        introduced: Some("0.0.0".to_string()),
                        fixed: Some("1.0.1".to_string()),
                    }],
                }],
            }],
            references: vec![],
        };

        assert_eq!(
            get_fixed_version(&vuln_with_fix, &dep),
            Some("1.0.1".to_string())
        );

        let vuln_no_fix = OsvVulnerability {
            id: "CVE-2023-1234".to_string(),
            summary: None,
            details: None,
            aliases: vec![],
            severity: vec![],
            affected: vec![OsvAffected {
                package: Some(OsvAffectedPackage {
                    name: "test-package".to_string(),
                    ecosystem: "crates.io".to_string(),
                }),
                ranges: vec![OsvRange {
                    range_type: "SEMVER".to_string(),
                    events: vec![OsvEvent {
                        introduced: Some("0.0.0".to_string()),
                        fixed: None,
                    }],
                }],
            }],
            references: vec![],
        };

        assert_eq!(get_fixed_version(&vuln_no_fix, &dep), None);
    }

    #[test]
    fn test_parse_pip_req() {
        use super::parse_pip_req;
        assert_eq!(
            parse_pip_req("requests==2.28.0"),
            Some(("requests".to_string(), "2.28.0".to_string()))
        );
        assert_eq!(
            parse_pip_req("requests>=2.28.0"),
            Some(("requests".to_string(), "2.28.0".to_string()))
        );
        assert_eq!(
            parse_pip_req("requests~=2.28.0"),
            Some(("requests".to_string(), "2.28.0".to_string()))
        );
        assert_eq!(parse_pip_req("requests"), None);
        assert_eq!(parse_pip_req("# comment"), None);
        assert_eq!(parse_pip_req(""), None);
    }

    // ===== Maven (pom.xml) Tests =====

    #[test]
    fn test_parse_pom_xml_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pom.xml"),
            r#"<project>
  <dependencies>
    <dependency>
      <groupId>org.springframework</groupId>
      <artifactId>spring-core</artifactId>
      <version>5.3.21</version>
    </dependency>
  </dependencies>
</project>"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pom_xml(&scanner).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "org.springframework:spring-core");
        assert_eq!(deps[0].version, "5.3.21");
        assert_eq!(deps[0].ecosystem, Ecosystem::Maven);
    }

    #[test]
    fn test_parse_pom_xml_multiple_deps() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pom.xml"),
            r#"<project>
  <dependencies>
    <dependency>
      <groupId>org.springframework</groupId>
      <artifactId>spring-core</artifactId>
      <version>5.3.21</version>
    </dependency>
    <dependency>
      <groupId>com.google.guava</groupId>
      <artifactId>guava</artifactId>
      <version>31.1-jre</version>
    </dependency>
    <dependency>
      <groupId>org.apache.commons</groupId>
      <artifactId>commons-lang3</artifactId>
      <version>${project.version}</version>
    </dependency>
  </dependencies>
</project>"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pom_xml(&scanner).unwrap();
        // 3rd dep has ${project.version}, should be skipped
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "org.springframework:spring-core")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "com.google.guava:guava" && d.version == "31.1-jre")
        );
    }

    #[test]
    fn test_parse_pom_xml_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pom_xml(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_parse_pom_xml_skips_dependency_management() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pom.xml"),
            r#"<project>
  <dependencyManagement>
    <dependencies>
      <dependency>
        <groupId>org.managed</groupId>
        <artifactId>managed-dep</artifactId>
        <version>1.0.0</version>
      </dependency>
    </dependencies>
  </dependencyManagement>
  <dependencies>
    <dependency>
      <groupId>org.direct</groupId>
      <artifactId>direct-dep</artifactId>
      <version>2.0.0</version>
    </dependency>
  </dependencies>
</project>"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pom_xml(&scanner).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "org.direct:direct-dep");
        assert_eq!(deps[0].version, "2.0.0");
    }

    // ===== Gradle Tests =====

    #[test]
    fn test_parse_gradle_build_implementation() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("build.gradle"),
            r#"
dependencies {
    implementation 'org.springframework.boot:spring-boot-starter:2.7.0'
    testImplementation 'junit:junit:4.13.2'
}
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gradle_build(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "org.springframework.boot:spring-boot-starter"
                    && d.version == "2.7.0"
                    && d.ecosystem == Ecosystem::Maven)
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "junit:junit" && d.version == "4.13.2")
        );
    }

    #[test]
    fn test_parse_gradle_build_multiple_formats() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("build.gradle"),
            r#"
dependencies {
    implementation "com.google.guava:guava:31.1-jre"
    api 'io.netty:netty-all:4.1.80.Final'
    runtimeOnly 'org.postgresql:postgresql:42.5.0'
    compileOnly 'org.projectlombok:lombok:1.18.24'
}
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gradle_build(&scanner).unwrap();
        assert_eq!(deps.len(), 4);
        assert!(deps.iter().any(|d| d.name == "com.google.guava:guava"));
        assert!(deps.iter().any(|d| d.name == "io.netty:netty-all"));
        assert!(deps.iter().any(|d| d.name == "org.postgresql:postgresql"));
        assert!(deps.iter().any(|d| d.name == "org.projectlombok:lombok"));
    }

    #[test]
    fn test_parse_gradle_build_kotlin_dsl() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("build.gradle.kts"),
            r#"
dependencies {
    implementation("org.jetbrains.kotlin:kotlin-stdlib:1.7.20")
    testImplementation("org.junit.jupiter:junit-jupiter:5.9.0")
}
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gradle_build(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "org.jetbrains.kotlin:kotlin-stdlib" && d.version == "1.7.20")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "org.junit.jupiter:junit-jupiter" && d.version == "5.9.0")
        );
    }

    #[test]
    fn test_parse_gradle_build_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gradle_build(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== Composer Lock Tests =====

    #[test]
    fn test_parse_composer_lock_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.lock"),
            r#"{
    "packages": [
        {
            "name": "monolog/monolog",
            "version": "2.8.0"
        }
    ],
    "packages-dev": []
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "monolog/monolog");
        assert_eq!(deps[0].version, "2.8.0");
        assert_eq!(deps[0].ecosystem, Ecosystem::Packagist);
    }

    #[test]
    fn test_parse_composer_lock_multiple_packages() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.lock"),
            r#"{
    "packages": [
        {
            "name": "monolog/monolog",
            "version": "v2.8.0"
        },
        {
            "name": "symfony/console",
            "version": "v5.4.12"
        }
    ],
    "packages-dev": [
        {
            "name": "phpunit/phpunit",
            "version": "v9.5.25"
        }
    ]
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 3);
        // v prefix should be stripped
        assert!(
            deps.iter()
                .any(|d| d.name == "monolog/monolog" && d.version == "2.8.0")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "symfony/console" && d.version == "5.4.12")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "phpunit/phpunit" && d.version == "9.5.25")
        );
    }

    #[test]
    fn test_parse_composer_lock_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_lock(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== Composer JSON Tests =====

    #[test]
    fn test_parse_composer_json_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.json"),
            r#"{
    "require": {
        "monolog/monolog": "^2.8"
    }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_json(&scanner).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "monolog/monolog");
        assert_eq!(deps[0].version, "2.8");
        assert_eq!(deps[0].ecosystem, Ecosystem::Packagist);
    }

    #[test]
    fn test_parse_composer_json_skip_php_and_ext() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.json"),
            r#"{
    "require": {
        "php": ">=8.0",
        "ext-json": "*",
        "ext-mbstring": "*",
        "monolog/monolog": "^2.8",
        "symfony/console": "~5.4"
    }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_json(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "monolog/monolog"));
        assert!(deps.iter().any(|d| d.name == "symfony/console"));
        // php and ext-* should be excluded
        assert!(!deps.iter().any(|d| d.name == "php"));
        assert!(!deps.iter().any(|d| d.name.starts_with("ext-")));
    }

    #[test]
    fn test_parse_composer_json_version_constraint_cleanup() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.json"),
            r#"{
    "require": {
        "pkg/a": "^1.2.3",
        "pkg/b": "~2.0",
        "pkg/c": ">=3.1.0",
        "pkg/d": "v4.0.0"
    },
    "require-dev": {
        "pkg/e": "^5.0"
    }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_json(&scanner).unwrap();
        assert_eq!(deps.len(), 5);
        assert!(
            deps.iter()
                .any(|d| d.name == "pkg/a" && d.version == "1.2.3")
        );
        assert!(deps.iter().any(|d| d.name == "pkg/b" && d.version == "2.0"));
        assert!(
            deps.iter()
                .any(|d| d.name == "pkg/c" && d.version == "3.1.0")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "pkg/d" && d.version == "4.0.0")
        );
        assert!(deps.iter().any(|d| d.name == "pkg/e" && d.version == "5.0"));
    }

    #[test]
    fn test_parse_composer_json_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_composer_json(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== Ecosystem Enum Tests =====

    #[test]
    fn test_ecosystem_maven_as_str() {
        assert_eq!(Ecosystem::Maven.as_str(), "Maven");
    }

    #[test]
    fn test_ecosystem_packagist_as_str() {
        assert_eq!(Ecosystem::Packagist.as_str(), "Packagist");
    }

    #[test]
    fn test_ecosystem_equality() {
        assert_eq!(Ecosystem::Maven, Ecosystem::Maven);
        assert_eq!(Ecosystem::Packagist, Ecosystem::Packagist);
        assert_ne!(Ecosystem::Maven, Ecosystem::Packagist);
        assert_ne!(Ecosystem::Maven, Ecosystem::Cargo);
    }

    // ===== Lock Files (DEP003) Tests =====

    #[tokio::test]
    async fn test_check_lock_files_cargo_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        // No Cargo.lock

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Rust"));
        assert!(findings[0].message.contains("Cargo.lock"));
    }

    #[tokio::test]
    async fn test_check_lock_files_cargo_present() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(tmp.path().join("Cargo.lock"), "[[package]]").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_npm_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
        // No lock file

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Node.js"));
    }

    #[tokio::test]
    async fn test_check_lock_files_npm_with_package_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
        fs::write(tmp.path().join("package-lock.json"), "{}").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_npm_with_yarn_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
        fs::write(tmp.path().join("yarn.lock"), "").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_npm_with_pnpm_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
        fs::write(tmp.path().join("pnpm-lock.yaml"), "").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_python_pyproject_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "[tool.poetry]").unwrap();
        // No poetry.lock or uv.lock

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Python"));
    }

    #[tokio::test]
    async fn test_check_lock_files_python_with_poetry_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "[tool.poetry]").unwrap();
        fs::write(tmp.path().join("poetry.lock"), "").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_python_with_uv_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "[tool.poetry]").unwrap();
        fs::write(tmp.path().join("uv.lock"), "").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_pipenv_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Pipfile"), "[packages]").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Pipenv"));
    }

    #[tokio::test]
    async fn test_check_lock_files_go_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Go"));
    }

    #[tokio::test]
    async fn test_check_lock_files_go_present() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();
        fs::write(tmp.path().join("go.sum"), "").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_composer_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("composer.json"), "{}").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("PHP"));
    }

    #[tokio::test]
    async fn test_check_lock_files_gemfile_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Ruby"));
    }

    #[tokio::test]
    async fn test_check_lock_files_pubspec_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pubspec.yaml"), "name: test").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Dart"));
    }

    #[tokio::test]
    async fn test_check_lock_files_swift_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Package.swift"),
            "// swift-tools-version:5.5",
        )
        .unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("Swift"));
    }

    #[tokio::test]
    async fn test_check_lock_files_podfile_missing() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Podfile"), "platform :ios, '13.0'").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "DEP003");
        assert!(findings[0].message.contains("CocoaPods"));
    }

    #[tokio::test]
    async fn test_check_lock_files_no_manifest() {
        let tmp = TempDir::new().unwrap();
        // No manifest files

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_lock_files_multiple_ecosystems() {
        let tmp = TempDir::new().unwrap();
        // Multiple manifests without lock files
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();

        let scanner = Scanner::new(tmp.path().to_path_buf());
        let findings = check_lock_files(&scanner).await.unwrap();

        assert_eq!(findings.len(), 3);
        assert!(findings.iter().all(|f| f.rule_id == "DEP003"));
    }

    // ===== NuGet Tests =====

    #[test]
    fn test_parse_nuget_lock_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("packages.lock.json"),
            r#"{
    "version": 1,
    "dependencies": {
        "net6.0": {
            "Newtonsoft.Json": {
                "resolved": "13.0.1"
            },
            "Serilog": {
                "resolved": "2.12.0"
            }
        }
    }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_nuget_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "Newtonsoft.Json" && d.version == "13.0.1")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "Serilog" && d.version == "2.12.0")
        );
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::NuGet));
    }

    #[test]
    fn test_parse_nuget_lock_multiple_frameworks() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("packages.lock.json"),
            r#"{
    "version": 1,
    "dependencies": {
        "net6.0": {
            "Newtonsoft.Json": {
                "resolved": "13.0.1"
            }
        },
        "net7.0": {
            "System.Text.Json": {
                "resolved": "7.0.0"
            }
        }
    }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_nuget_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "Newtonsoft.Json"));
        assert!(deps.iter().any(|d| d.name == "System.Text.Json"));
    }

    #[test]
    fn test_parse_nuget_lock_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_nuget_lock(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_parse_nuget_csproj_fallback() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("MyProject.csproj"),
            r#"<Project>
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
    <PackageReference Include="Microsoft.Extensions.Logging" Version="7.0.0" />
  </ItemGroup>
</Project>"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_nuget_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "Newtonsoft.Json" && d.version == "13.0.1")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "Microsoft.Extensions.Logging" && d.version == "7.0.0")
        );
    }

    // ===== Ruby Gemfile.lock Tests =====

    #[test]
    fn test_parse_gemfile_lock_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Gemfile.lock"),
            r#"GEM
  remote: https://rubygems.org/
  specs:
    rails (7.0.4)
    nokogiri (1.14.0)

PLATFORMS
  ruby

DEPENDENCIES
  rails
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gemfile_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "rails" && d.version == "7.0.4")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "nokogiri" && d.version == "1.14.0")
        );
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::RubyGems));
    }

    #[test]
    fn test_parse_gemfile_lock_with_dependencies() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Gemfile.lock"),
            r#"GEM
  remote: https://rubygems.org/
  specs:
    rack (3.0.2)
    rails (7.0.4)
      actionpack (= 7.0.4)
    actionpack (7.0.4)
      rack (>= 2.2.0)

PLATFORMS
  ruby
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gemfile_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "rack"));
        assert!(deps.iter().any(|d| d.name == "rails"));
        assert!(deps.iter().any(|d| d.name == "actionpack"));
    }

    #[test]
    fn test_parse_gemfile_lock_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_gemfile_lock(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== CocoaPods Podfile.lock Tests =====

    #[test]
    fn test_parse_podfile_lock_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Podfile.lock"),
            r#"PODS:
  - Alamofire (5.6.4)
  - SwiftyJSON (5.0.1)

DEPENDENCIES:
  - Alamofire
  - SwiftyJSON

SPEC CHECKSUMS:
  Alamofire: abc123
  SwiftyJSON: def456
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_podfile_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "Alamofire" && d.version == "5.6.4")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "SwiftyJSON" && d.version == "5.0.1")
        );
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::CocoaPods));
    }

    #[test]
    fn test_parse_podfile_lock_with_dependencies() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Podfile.lock"),
            r#"PODS:
  - Firebase/Core (10.3.0):
    - FirebaseAnalytics
  - FirebaseAnalytics (10.3.0)

DEPENDENCIES:
  - Firebase/Core
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_podfile_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "Firebase/Core" && d.version == "10.3.0")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "FirebaseAnalytics" && d.version == "10.3.0")
        );
    }

    #[test]
    fn test_parse_podfile_lock_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_podfile_lock(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== Swift Package.resolved Tests =====

    #[test]
    fn test_parse_package_resolved_v2() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Package.resolved"),
            r#"{
  "pins": [
    {
      "identity": "swift-argument-parser",
      "kind": "remoteSourceControl",
      "location": "https://github.com/apple/swift-argument-parser.git",
      "state": {
        "revision": "abc123",
        "version": "1.2.0"
      }
    },
    {
      "identity": "swift-log",
      "kind": "remoteSourceControl",
      "location": "https://github.com/apple/swift-log.git",
      "state": {
        "revision": "def456",
        "version": "1.5.2"
      }
    }
  ],
  "version": 2
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_resolved(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "swift-argument-parser" && d.version == "1.2.0")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "swift-log" && d.version == "1.5.2")
        );
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::SwiftPM));
    }

    #[test]
    fn test_parse_package_resolved_v1() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Package.resolved"),
            r#"{
  "object": {
    "pins": [
      {
        "package": "Alamofire",
        "repositoryURL": "https://github.com/Alamofire/Alamofire.git",
        "state": {
          "branch": null,
          "revision": "abc123",
          "version": "5.6.4"
        }
      }
    ]
  },
  "version": 1
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_resolved(&scanner).unwrap();
        assert_eq!(deps.len(), 1);
        assert!(
            deps.iter()
                .any(|d| d.name == "Alamofire" && d.version == "5.6.4")
        );
    }

    #[test]
    fn test_parse_package_resolved_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_resolved(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== Dart/Flutter pubspec.lock Tests =====

    #[test]
    fn test_parse_pubspec_lock_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pubspec.lock"),
            r#"packages:
  http:
    dependency: "direct main"
    description:
      name: http
      url: "https://pub.dev"
    source: hosted
    version: "0.13.5"
  json_annotation:
    dependency: transitive
    description:
      name: json_annotation
      url: "https://pub.dev"
    source: hosted
    version: "4.8.0"
sdks:
  dart: ">=2.18.0 <4.0.0"
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pubspec_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "http" && d.version == "0.13.5")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "json_annotation" && d.version == "4.8.0")
        );
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::Pub));
    }

    #[test]
    fn test_parse_pubspec_lock_flutter() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pubspec.lock"),
            r#"packages:
  flutter:
    dependency: "direct main"
    description: flutter
    source: sdk
    version: "0.0.0"
  provider:
    dependency: "direct main"
    description:
      name: provider
      url: "https://pub.dev"
    source: hosted
    version: "6.0.5"
sdks:
  dart: ">=3.0.0 <4.0.0"
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pubspec_lock(&scanner).unwrap();
        assert!(
            deps.iter()
                .any(|d| d.name == "provider" && d.version == "6.0.5")
        );
    }

    #[test]
    fn test_parse_pubspec_lock_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_pubspec_lock(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== Ecosystem Tests =====

    #[test]
    fn test_ecosystem_is_osv_supported() {
        assert!(Ecosystem::Cargo.is_osv_supported());
        assert!(Ecosystem::Npm.is_osv_supported());
        assert!(Ecosystem::PyPI.is_osv_supported());
        assert!(Ecosystem::Go.is_osv_supported());
        assert!(Ecosystem::Maven.is_osv_supported());
        assert!(Ecosystem::Packagist.is_osv_supported());
        assert!(Ecosystem::NuGet.is_osv_supported());
        assert!(Ecosystem::RubyGems.is_osv_supported());
        assert!(Ecosystem::Pub.is_osv_supported());
        // Not supported
        assert!(!Ecosystem::CocoaPods.is_osv_supported());
        assert!(!Ecosystem::SwiftPM.is_osv_supported());
    }

    #[test]
    fn test_get_ecosystem_lock_file() {
        assert_eq!(get_ecosystem_lock_file(Ecosystem::Cargo), "Cargo.lock");
        assert_eq!(get_ecosystem_lock_file(Ecosystem::Npm), "package-lock.json");
        assert_eq!(
            get_ecosystem_lock_file(Ecosystem::NuGet),
            "packages.lock.json"
        );
        assert_eq!(get_ecosystem_lock_file(Ecosystem::RubyGems), "Gemfile.lock");
        assert_eq!(
            get_ecosystem_lock_file(Ecosystem::CocoaPods),
            "Podfile.lock"
        );
        assert_eq!(
            get_ecosystem_lock_file(Ecosystem::SwiftPM),
            "Package.resolved"
        );
        assert_eq!(get_ecosystem_lock_file(Ecosystem::Pub), "pubspec.lock");
    }

    // ===== parse_package_lock Tests =====

    #[test]
    fn test_parse_package_lock_v2_packages() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package-lock.json"),
            r#"{
  "name": "test-project",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "": {
      "name": "test-project",
      "version": "1.0.0"
    },
    "node_modules/lodash": {
      "version": "4.17.21"
    },
    "node_modules/express": {
      "version": "4.18.2"
    }
  }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "lodash" && d.version == "4.17.21")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "express" && d.version == "4.18.2")
        );
    }

    #[test]
    fn test_parse_package_lock_v1_dependencies() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package-lock.json"),
            r#"{
  "name": "test-project",
  "version": "1.0.0",
  "lockfileVersion": 1,
  "dependencies": {
    "lodash": {
      "version": "4.17.21"
    },
    "express": {
      "version": "4.18.2",
      "dependencies": {
        "body-parser": {
          "version": "1.20.1"
        }
      }
    }
  }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "lodash"));
        assert!(deps.iter().any(|d| d.name == "express"));
        assert!(deps.iter().any(|d| d.name == "body-parser"));
    }

    #[test]
    fn test_parse_package_lock_scoped_packages() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package-lock.json"),
            r#"{
  "lockfileVersion": 2,
  "packages": {
    "": {},
    "node_modules/@types/node": {
      "version": "18.11.18"
    },
    "node_modules/@babel/core": {
      "version": "7.20.12"
    }
  }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_lock(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "@types/node"));
        assert!(deps.iter().any(|d| d.name == "@babel/core"));
    }

    #[test]
    fn test_parse_package_lock_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_package_lock(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== parse_requirements_txt Tests =====

    #[test]
    fn test_parse_requirements_txt_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements.txt"),
            "requests==2.28.0\nflask>=2.0.0\ndjango~=4.1.0",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_requirements_txt(&scanner).unwrap();
        assert_eq!(deps.len(), 3);
        assert!(
            deps.iter()
                .any(|d| d.name == "requests" && d.version == "2.28.0")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "flask" && d.version == "2.0.0")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "django" && d.version == "4.1.0")
        );
    }

    #[test]
    fn test_parse_requirements_txt_with_comments() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements.txt"),
            "# This is a comment\nrequests==2.28.0\n# Another comment\nflask>=2.0.0",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_requirements_txt(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_parse_requirements_txt_with_extras() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements.txt"),
            "requests[security]==2.28.0\ncelery[redis,msgpack]>=5.2.0",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_requirements_txt(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "requests"));
        assert!(deps.iter().any(|d| d.name == "celery"));
    }

    #[test]
    fn test_parse_requirements_txt_dev_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements-dev.txt"),
            "pytest==7.2.0\nmypy>=0.991",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_requirements_txt(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "pytest"));
        assert!(deps.iter().any(|d| d.name == "mypy"));
    }

    #[test]
    fn test_parse_requirements_txt_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_requirements_txt(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_parse_requirements_txt_with_environment_markers() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements.txt"),
            "pywin32==305; sys_platform == 'win32'\nrequests==2.28.0",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_requirements_txt(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "pywin32"));
        assert!(deps.iter().any(|d| d.name == "requests"));
    }

    // ===== parse_go_sum Tests =====

    #[test]
    fn test_parse_go_sum_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("go.sum"),
            "github.com/gin-gonic/gin v1.8.2 h1:hashA\n\
             github.com/gin-gonic/gin v1.8.2/go.mod h1:hashB\n\
             golang.org/x/net v0.4.0 h1:hashC",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_go_sum(&scanner).unwrap();
        // Should deduplicate the gin entries
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "github.com/gin-gonic/gin" && d.version == "1.8.2")
        );
        assert!(
            deps.iter()
                .any(|d| d.name == "golang.org/x/net" && d.version == "0.4.0")
        );
    }

    #[test]
    fn test_parse_go_sum_multiple_versions() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("go.sum"),
            "github.com/pkg/errors v0.8.0 h1:hashA\n\
             github.com/pkg/errors v0.9.1 h1:hashB",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_go_sum(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.version == "0.8.0"));
        assert!(deps.iter().any(|d| d.version == "0.9.1"));
    }

    #[test]
    fn test_parse_go_sum_prerelease_versions() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("go.sum"),
            "github.com/test/pkg v1.0.0-beta.1 h1:hashA\n\
             github.com/test/pkg2 v2.0.0-rc1+meta h1:hashB",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_go_sum(&scanner).unwrap();
        assert_eq!(deps.len(), 2);
        // Pre-release suffix should be stripped
        assert!(deps.iter().any(|d| d.version == "1.0.0"));
        assert!(deps.iter().any(|d| d.version == "2.0.0"));
    }

    #[test]
    fn test_parse_go_sum_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let deps = parse_go_sum(&scanner).unwrap();
        assert!(deps.is_empty());
    }

    // ===== get_lock_file_command Tests =====

    #[test]
    fn test_get_lock_file_command() {
        assert!(get_lock_file_command("Cargo.toml").contains("cargo"));
        assert!(get_lock_file_command("package.json").contains("npm"));
        assert!(get_lock_file_command("pyproject.toml").contains("poetry"));
        assert!(get_lock_file_command("Pipfile").contains("pipenv"));
        assert!(get_lock_file_command("go.mod").contains("go mod"));
        assert!(get_lock_file_command("composer.json").contains("composer"));
        assert!(get_lock_file_command("Gemfile").contains("bundle"));
        assert!(get_lock_file_command("pubspec.yaml").contains("pub get"));
        assert!(get_lock_file_command("Package.swift").contains("swift"));
        assert!(get_lock_file_command("Podfile").contains("pod"));
        assert!(get_lock_file_command("unknown.file").contains("package manager"));
    }
}
