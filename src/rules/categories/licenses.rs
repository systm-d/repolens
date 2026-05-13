//! License compliance rules
//!
//! This module provides rules for checking license compliance, including:
//! - Detecting the project license
//! - Parsing dependency licenses from manifest files
//! - Checking compatibility between project and dependency licenses
//! - Alerting on unknown or missing licenses

use crate::config::Config;
use crate::error::RepoLensError;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;
use regex::Regex;
use std::collections::HashMap;

/// Rules for checking license compliance
pub struct LicenseRules;

#[async_trait::async_trait]
impl RuleCategory for LicenseRules {
    /// Get the category name
    fn name(&self) -> &'static str {
        "licenses"
    }

    /// Run all license compliance rules
    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        if !config.is_rule_enabled("licenses/compliance") {
            return Ok(findings);
        }

        let license_config = &config.license_compliance;

        if !license_config.enabled {
            return Ok(findings);
        }

        // LIC001: Detect project license
        let project_license = detect_project_license(scanner);
        if project_license.is_none() {
            findings.push(
                Finding::new(
                    "LIC001",
                    "licenses",
                    Severity::Warning,
                    "No project license detected",
                )
                .with_description(
                    "No LICENSE file or license field found in the project manifest. \
                     A license is required for others to legally use your code.",
                )
                .with_remediation(
                    "Add a LICENSE file to the repository root or specify a license \
                     in your project manifest (e.g., Cargo.toml, package.json).",
                ),
            );
        }

        // Parse dependency licenses from all supported manifest files
        let dep_licenses = collect_dependency_licenses(scanner);

        for dep_license in &dep_licenses {
            // LIC004: Dependency without license
            if dep_license.license.is_none() {
                findings.push(
                    Finding::new(
                        "LIC004",
                        "licenses",
                        Severity::Warning,
                        format!("Dependency '{}' has no license specified", dep_license.name),
                    )
                    .with_location(&dep_license.source_file)
                    .with_description(
                        "Using dependencies without a license may expose your project \
                         to legal risk, as all code is copyrighted by default.",
                    )
                    .with_remediation(format!(
                        "Check the '{}' project repository for license information \
                         and consider replacing it if no license is available.",
                        dep_license.name
                    )),
                );
                continue;
            }

            let dep_lic = dep_license.license.as_ref().unwrap();

            // LIC003: Unknown/unrecognized license
            if !is_known_license(dep_lic) {
                findings.push(
                    Finding::new(
                        "LIC003",
                        "licenses",
                        Severity::Info,
                        format!(
                            "Dependency '{}' uses unknown license: {}",
                            dep_license.name, dep_lic
                        ),
                    )
                    .with_location(&dep_license.source_file)
                    .with_description(
                        "The dependency uses a license that is not recognized. \
                         Manual review may be needed to ensure compatibility.",
                    )
                    .with_remediation(format!(
                        "Review the license of '{}' ({}) to verify it is \
                         compatible with your project.",
                        dep_license.name, dep_lic
                    )),
                );
                continue;
            }

            // Check against denied list
            if is_license_denied(dep_lic, &license_config.denied_licenses) {
                findings.push(
                    Finding::new(
                        "LIC002",
                        "licenses",
                        Severity::Critical,
                        format!(
                            "Dependency '{}' uses denied license: {}",
                            dep_license.name, dep_lic
                        ),
                    )
                    .with_location(&dep_license.source_file)
                    .with_description(format!(
                        "The dependency '{}' uses the '{}' license which is on \
                         the denied list for this project.",
                        dep_license.name, dep_lic
                    ))
                    .with_remediation(format!(
                        "Replace '{}' with an alternative that uses a permitted license, \
                         or update the denied_licenses configuration if this license is acceptable.",
                        dep_license.name
                    )),
                );
                continue;
            }

            // Check against allowed list (if configured)
            if !license_config.allowed_licenses.is_empty()
                && !is_license_allowed(dep_lic, &license_config.allowed_licenses)
            {
                findings.push(
                    Finding::new(
                        "LIC002",
                        "licenses",
                        Severity::Warning,
                        format!(
                            "Dependency '{}' uses license '{}' not in the allowed list",
                            dep_license.name, dep_lic
                        ),
                    )
                    .with_location(&dep_license.source_file)
                    .with_description(format!(
                        "The dependency '{}' uses the '{}' license which is not in \
                         the project's allowed license list.",
                        dep_license.name, dep_lic
                    ))
                    .with_remediation(format!(
                        "Add '{}' to the allowed_licenses list in .repolens.toml, \
                         or replace '{}' with a dependency that uses an allowed license.",
                        dep_lic, dep_license.name
                    )),
                );
                continue;
            }

            // LIC002: Check compatibility with project license
            if let Some(ref proj_lic) = project_license {
                if !is_compatible(proj_lic, dep_lic) {
                    findings.push(
                        Finding::new(
                            "LIC002",
                            "licenses",
                            Severity::Critical,
                            format!(
                                "Dependency '{}' license '{}' is incompatible with project license '{}'",
                                dep_license.name, dep_lic, proj_lic
                            ),
                        )
                        .with_location(&dep_license.source_file)
                        .with_description(format!(
                            "The '{}' license used by '{}' is not compatible with \
                             the project's '{}' license. This could create legal issues.",
                            dep_lic, dep_license.name, proj_lic
                        ))
                        .with_remediation(format!(
                            "Replace '{}' with an alternative that uses a license \
                             compatible with '{}', or change the project license.",
                            dep_license.name, proj_lic
                        )),
                    );
                }
            }
        }

        Ok(findings)
    }
}

/// Information about a dependency's license
#[derive(Debug, Clone)]
pub struct DependencyLicense {
    /// Name of the dependency
    pub name: String,
    /// License identifier (SPDX), if detected
    pub license: Option<String>,
    /// Source file where the dependency was found
    pub source_file: String,
}

/// Detect the project's license from common locations
pub fn detect_project_license(scanner: &Scanner) -> Option<String> {
    // Check LICENSE file content
    for license_file in &[
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "LICENCE",
        "LICENCE.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        if scanner.file_exists(license_file) {
            if let Ok(content) = scanner.read_file(license_file) {
                if let Some(lic) = detect_license_from_content(&content) {
                    return Some(lic);
                }
            }
        }
    }

    // Check Cargo.toml license field
    if scanner.file_exists("Cargo.toml") {
        if let Ok(content) = scanner.read_file("Cargo.toml") {
            if let Ok(parsed) = content.parse::<toml::Value>() {
                if let Some(lic) = parsed
                    .get("package")
                    .and_then(|p| p.get("license"))
                    .and_then(|l| l.as_str())
                {
                    return Some(normalize_license(lic));
                }
            }
        }
    }

    // Check package.json license field
    if scanner.file_exists("package.json") {
        if let Ok(content) = scanner.read_file("package.json") {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(lic) = parsed.get("license").and_then(|l| l.as_str()) {
                    return Some(normalize_license(lic));
                }
            }
        }
    }

    // Check setup.py / setup.cfg for Python
    if scanner.file_exists("setup.cfg") {
        if let Ok(content) = scanner.read_file("setup.cfg") {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("license") {
                    let rest = rest.trim();
                    if let Some(value) = rest.strip_prefix('=') {
                        let lic = value.trim();
                        if !lic.is_empty() {
                            return Some(normalize_license(lic));
                        }
                    }
                }
            }
        }
    }

    // Check pyproject.toml
    if scanner.file_exists("pyproject.toml") {
        if let Ok(content) = scanner.read_file("pyproject.toml") {
            if let Ok(parsed) = content.parse::<toml::Value>() {
                // Check [project] table first (PEP 621)
                if let Some(lic) = parsed
                    .get("project")
                    .and_then(|p| p.get("license"))
                    .and_then(|l| {
                        // Can be a string or a table with "text" key
                        l.as_str().map(|s| s.to_string()).or_else(|| {
                            l.get("text")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string())
                        })
                    })
                {
                    return Some(normalize_license(&lic));
                }
                // Check [tool.poetry] table
                if let Some(lic) = parsed
                    .get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("license"))
                    .and_then(|l| l.as_str())
                {
                    return Some(normalize_license(lic));
                }
            }
        }
    }

    None
}

/// Detect license type from file content
fn detect_license_from_content(content: &str) -> Option<String> {
    let lower = content.to_lowercase();

    if lower.contains("mit license")
        || lower.contains("permission is hereby granted, free of charge")
    {
        return Some("MIT".to_string());
    }
    if lower.contains("apache license") && lower.contains("version 2.0") {
        return Some("Apache-2.0".to_string());
    }
    if lower.contains("gnu general public license") {
        if lower.contains("version 3") {
            return Some("GPL-3.0".to_string());
        }
        if lower.contains("version 2") {
            return Some("GPL-2.0".to_string());
        }
    }
    if lower.contains("gnu lesser general public license") {
        if lower.contains("version 3") {
            return Some("LGPL-3.0".to_string());
        }
        if lower.contains("version 2.1") {
            return Some("LGPL-2.1".to_string());
        }
    }
    if lower.contains("gnu affero general public license") {
        return Some("AGPL-3.0".to_string());
    }
    if lower.contains("bsd 3-clause")
        || lower.contains("redistribution and use in source and binary forms")
            && lower.contains("neither the name")
    {
        return Some("BSD-3-Clause".to_string());
    }
    if lower.contains("bsd 2-clause")
        || lower.contains("redistribution and use in source and binary forms")
            && !lower.contains("neither the name")
            && lower.contains("this list of conditions")
    {
        return Some("BSD-2-Clause".to_string());
    }
    if lower.contains("isc license")
        || lower.contains("permission to use, copy, modify, and/or distribute")
    {
        return Some("ISC".to_string());
    }
    if lower.contains("mozilla public license") && lower.contains("version 2.0") {
        return Some("MPL-2.0".to_string());
    }
    if lower.contains("the unlicense") || lower.contains("this is free and unencumbered software") {
        return Some("Unlicense".to_string());
    }

    None
}

/// Collect dependency licenses from all supported manifest files
pub fn collect_dependency_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    licenses.extend(parse_cargo_toml_licenses(scanner));
    licenses.extend(parse_package_json_licenses(scanner));
    licenses.extend(parse_requirements_txt_licenses(scanner));
    licenses.extend(parse_go_mod_licenses(scanner));
    licenses.extend(parse_pom_xml_licenses(scanner));
    licenses.extend(parse_composer_json_licenses(scanner));
    // New ecosystems
    licenses.extend(parse_nuget_licenses(scanner));
    licenses.extend(parse_gemspec_licenses(scanner));
    licenses.extend(parse_podspec_licenses(scanner));
    licenses.extend(parse_pubspec_licenses(scanner));

    licenses
}

/// Parse dependency licenses from Cargo.toml
fn parse_cargo_toml_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    if !scanner.file_exists("Cargo.toml") {
        return licenses;
    }

    let content = match scanner.read_file("Cargo.toml") {
        Ok(c) => c,
        Err(_) => return licenses,
    };

    let parsed: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return licenses,
    };

    if let Some(deps) = parsed.get("dependencies") {
        extract_cargo_deps(deps, &mut licenses, "Cargo.toml");
    }
    if let Some(deps) = parsed.get("dev-dependencies") {
        extract_cargo_deps(deps, &mut licenses, "Cargo.toml");
    }
    if let Some(deps) = parsed.get("build-dependencies") {
        extract_cargo_deps(deps, &mut licenses, "Cargo.toml");
    }

    licenses
}

/// Extract dependencies from a Cargo.toml dependencies table
fn extract_cargo_deps(
    deps: &toml::Value,
    licenses: &mut Vec<DependencyLicense>,
    source_file: &str,
) {
    if let Some(table) = deps.as_table() {
        for (name, _value) in table {
            // Cargo.toml doesn't include license information directly.
            // We record the dependency; the license would need to be resolved
            // from a Cargo.lock or registry lookup. For now, we mark them
            // as having no license info unless Cargo.lock provides it.
            licenses.push(DependencyLicense {
                name: name.clone(),
                license: None,
                source_file: source_file.to_string(),
            });
        }
    }
}

/// Parse dependency licenses from package.json
fn parse_package_json_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    if !scanner.file_exists("package.json") {
        return licenses;
    }

    let content = match scanner.read_file("package.json") {
        Ok(c) => c,
        Err(_) => return licenses,
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return licenses,
    };

    // Check dependencies
    if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_object()) {
        for (name, _) in deps {
            licenses.push(DependencyLicense {
                name: name.clone(),
                license: None,
                source_file: "package.json".to_string(),
            });
        }
    }

    // Check devDependencies
    if let Some(deps) = parsed.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, _) in deps {
            licenses.push(DependencyLicense {
                name: name.clone(),
                license: None,
                source_file: "package.json".to_string(),
            });
        }
    }

    // Also try to read node_modules for actual license info
    if scanner.directory_exists("node_modules") {
        for dep in &mut licenses {
            let pkg_path = format!("node_modules/{}/package.json", dep.name);
            if scanner.file_exists(&pkg_path) {
                if let Ok(pkg_content) = scanner.read_file(&pkg_path) {
                    if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                        if let Some(lic) = pkg_json.get("license").and_then(|l| l.as_str()) {
                            dep.license = Some(normalize_license(lic));
                        }
                    }
                }
            }
        }
    }

    licenses
}

/// Parse dependency licenses from requirements.txt
fn parse_requirements_txt_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    for req_file in &[
        "requirements.txt",
        "requirements-dev.txt",
        "requirements/base.txt",
    ] {
        if !scanner.file_exists(req_file) {
            continue;
        }

        let content = match scanner.read_file(req_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
                continue;
            }

            // Parse package name (before version specifier)
            if let Some(name) = extract_python_package_name(trimmed) {
                licenses.push(DependencyLicense {
                    name,
                    license: None, // Python packages don't embed license in requirements.txt
                    source_file: req_file.to_string(),
                });
            }
        }
    }

    licenses
}

/// Extract Python package name from a requirements line
fn extract_python_package_name(line: &str) -> Option<String> {
    let line = line.split(';').next()?.trim().split('#').next()?.trim();

    // Remove extras like [security]
    let line = if let Some(bracket_pos) = line.find('[') {
        if let Some(end_pos) = line.find(']') {
            format!("{}{}", &line[..bracket_pos], &line[end_pos + 1..])
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    };

    // Find version separator
    for sep in &["==", ">=", "<=", "~=", "!=", ">", "<"] {
        if let Some(pos) = line.find(sep) {
            let name = line[..pos].trim();
            if !name.is_empty() {
                return Some(name.to_lowercase());
            }
        }
    }

    // No version specifier, just the package name
    let name = line.trim();
    if !name.is_empty() && name.chars().next().is_some_and(|c| c.is_alphabetic()) {
        Some(name.to_lowercase())
    } else {
        None
    }
}

/// Parse dependency licenses from go.mod
fn parse_go_mod_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    if !scanner.file_exists("go.mod") {
        return licenses;
    }

    let content = match scanner.read_file("go.mod") {
        Ok(c) => c,
        Err(_) => return licenses,
    };

    let mut in_require_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("require (") || trimmed == "require (" {
            in_require_block = true;
            continue;
        }

        if in_require_block && trimmed == ")" {
            in_require_block = false;
            continue;
        }

        if in_require_block {
            // Parse: module/path v1.2.3
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && !trimmed.starts_with("//") {
                let module = parts[0];
                // Indirect dependencies are marked with // indirect
                licenses.push(DependencyLicense {
                    name: module.to_string(),
                    license: None, // Go doesn't embed license in go.mod
                    source_file: "go.mod".to_string(),
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("require ") {
            // Single-line require
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                licenses.push(DependencyLicense {
                    name: parts[0].to_string(),
                    license: None,
                    source_file: "go.mod".to_string(),
                });
            }
        }
    }

    licenses
}

/// Parse dependency licenses from pom.xml (Maven)
fn parse_pom_xml_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    if !scanner.file_exists("pom.xml") {
        return licenses;
    }

    let content = match scanner.read_file("pom.xml") {
        Ok(c) => c,
        Err(_) => return licenses,
    };

    // Try to extract project-level license from <licenses> block
    let project_license = {
        let lic_block_re = Regex::new(r"(?s)<licenses>(.*?)</licenses>").unwrap();
        let lic_name_re = Regex::new(r"<name>\s*([^<]+?)\s*</name>").unwrap();
        lic_block_re.captures(&content).and_then(|cap| {
            lic_name_re
                .captures(&cap[1])
                .map(|c| normalize_license(&c[1]))
        })
    };

    // Remove <dependencyManagement> sections
    let mgmt_re = Regex::new(r"(?s)<dependencyManagement>.*?</dependencyManagement>").unwrap();
    let content = mgmt_re.replace_all(&content, "");

    // Extract dependencies
    let deps_block_re = Regex::new(r"(?s)<dependencies>(.*?)</dependencies>").unwrap();
    let dep_re = Regex::new(r"(?s)<dependency>(.*?)</dependency>").unwrap();
    let group_re = Regex::new(r"<groupId>\s*([^<]+?)\s*</groupId>").unwrap();
    let artifact_re = Regex::new(r"<artifactId>\s*([^<]+?)\s*</artifactId>").unwrap();

    for block_cap in deps_block_re.captures_iter(&content) {
        let block = &block_cap[1];
        for dep_cap in dep_re.captures_iter(block) {
            let dep_content = &dep_cap[1];
            let group = group_re.captures(dep_content).map(|c| c[1].to_string());
            let artifact = artifact_re.captures(dep_content).map(|c| c[1].to_string());

            if let (Some(g), Some(a)) = (group, artifact) {
                licenses.push(DependencyLicense {
                    name: format!("{}:{}", g, a),
                    license: project_license.clone(),
                    source_file: "pom.xml".to_string(),
                });
            }
        }
    }

    licenses
}

/// Parse dependency licenses from composer.json / composer.lock (PHP)
fn parse_composer_json_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();

    // Try composer.lock first for richer license data
    if scanner.file_exists("composer.lock") {
        let content = match scanner.read_file("composer.lock") {
            Ok(c) => c,
            Err(_) => return licenses,
        };
        let lock: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return licenses,
        };

        for key in &["packages", "packages-dev"] {
            if let Some(packages) = lock.get(*key).and_then(|p| p.as_array()) {
                for pkg in packages {
                    let name = match pkg.get("name").and_then(|n| n.as_str()) {
                        Some(n) => n.to_string(),
                        None => continue,
                    };
                    let license = pkg
                        .get("license")
                        .and_then(|l| l.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .map(normalize_license);
                    licenses.push(DependencyLicense {
                        name,
                        license,
                        source_file: "composer.lock".to_string(),
                    });
                }
            }
        }
        return licenses;
    }

    // Fallback to composer.json
    if !scanner.file_exists("composer.json") {
        return licenses;
    }

    let content = match scanner.read_file("composer.json") {
        Ok(c) => c,
        Err(_) => return licenses,
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return licenses,
    };

    // Extract project license
    let project_license = parsed.get("license").and_then(|l| {
        if let Some(s) = l.as_str() {
            Some(normalize_license(s))
        } else if let Some(arr) = l.as_array() {
            arr.first().and_then(|v| v.as_str()).map(normalize_license)
        } else {
            None
        }
    });

    for section in &["require", "require-dev"] {
        if let Some(reqs) = parsed.get(*section).and_then(|r| r.as_object()) {
            for (name, _) in reqs {
                if name == "php" || name.starts_with("ext-") {
                    continue;
                }
                licenses.push(DependencyLicense {
                    name: name.clone(),
                    license: project_license.clone(),
                    source_file: "composer.json".to_string(),
                });
            }
        }
    }

    licenses
}

/// Normalize a license string to a standard SPDX identifier
pub fn normalize_license(license: &str) -> String {
    let normalized = license.trim();

    // Build a map of common aliases
    let aliases: HashMap<&str, &str> = [
        ("mit", "MIT"),
        ("apache-2.0", "Apache-2.0"),
        ("apache 2.0", "Apache-2.0"),
        ("apache license 2.0", "Apache-2.0"),
        ("apache2", "Apache-2.0"),
        ("gpl-2.0", "GPL-2.0"),
        ("gpl-2.0-only", "GPL-2.0"),
        ("gpl-2.0-or-later", "GPL-2.0"),
        ("gpl2", "GPL-2.0"),
        ("gpl-3.0", "GPL-3.0"),
        ("gpl-3.0-only", "GPL-3.0"),
        ("gpl-3.0-or-later", "GPL-3.0"),
        ("gpl3", "GPL-3.0"),
        ("lgpl-2.1", "LGPL-2.1"),
        ("lgpl-2.1-only", "LGPL-2.1"),
        ("lgpl-2.1-or-later", "LGPL-2.1"),
        ("lgpl-3.0", "LGPL-3.0"),
        ("lgpl-3.0-only", "LGPL-3.0"),
        ("lgpl-3.0-or-later", "LGPL-3.0"),
        ("agpl-3.0", "AGPL-3.0"),
        ("agpl-3.0-only", "AGPL-3.0"),
        ("agpl-3.0-or-later", "AGPL-3.0"),
        ("bsd-2-clause", "BSD-2-Clause"),
        ("bsd 2-clause", "BSD-2-Clause"),
        ("bsd-3-clause", "BSD-3-Clause"),
        ("bsd 3-clause", "BSD-3-Clause"),
        ("isc", "ISC"),
        ("mpl-2.0", "MPL-2.0"),
        ("mozilla public license 2.0", "MPL-2.0"),
        ("unlicense", "Unlicense"),
        ("public domain", "Unlicense"),
        ("0bsd", "0BSD"),
        ("cc0-1.0", "CC0-1.0"),
        ("zlib", "Zlib"),
        ("artistic-2.0", "Artistic-2.0"),
        ("bsl-1.0", "BSL-1.0"),
    ]
    .iter()
    .copied()
    .collect();

    let lower = normalized.to_lowercase();
    if let Some(&canonical) = aliases.get(lower.as_str()) {
        return canonical.to_string();
    }

    // Return as-is if no alias found
    normalized.to_string()
}

/// Known SPDX license identifiers
const KNOWN_LICENSES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "GPL-2.0",
    "GPL-3.0",
    "LGPL-2.1",
    "LGPL-3.0",
    "AGPL-3.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
    "Unlicense",
    "0BSD",
    "CC0-1.0",
    "Zlib",
    "Artistic-2.0",
    "BSL-1.0",
];

/// Check if a license is a known SPDX identifier
pub fn is_known_license(license: &str) -> bool {
    let normalized = normalize_license(license);
    KNOWN_LICENSES.contains(&normalized.as_str())
}

/// Check if a license is in the denied list
pub fn is_license_denied(license: &str, denied: &[String]) -> bool {
    if denied.is_empty() {
        return false;
    }

    let normalized = normalize_license(license);
    denied.iter().any(|d| normalize_license(d) == normalized)
}

/// Check if a license is in the allowed list
pub fn is_license_allowed(license: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true; // No allowlist means everything is allowed
    }

    let normalized = normalize_license(license);
    allowed.iter().any(|a| normalize_license(a) == normalized)
}

/// License compatibility matrix
///
/// Returns true if `dependency_license` is compatible with `project_license`.
/// Compatibility means the dependency can be used in a project with the given license.
pub fn is_compatible(project_license: &str, dependency_license: &str) -> bool {
    let proj = normalize_license(project_license);
    let dep = normalize_license(dependency_license);

    // Same license is always compatible
    if proj == dep {
        return true;
    }

    // Permissive licenses are compatible with everything
    let permissive = [
        "MIT",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Unlicense",
        "0BSD",
        "CC0-1.0",
        "Zlib",
        "BSL-1.0",
    ];
    if permissive.contains(&dep.as_str()) {
        return true;
    }

    // Apache-2.0 is compatible with most licenses except GPL-2.0
    if dep == "Apache-2.0" {
        return proj != "GPL-2.0";
    }

    // MPL-2.0 is compatible with most licenses (weak copyleft)
    if dep == "MPL-2.0" {
        return !matches!(
            proj.as_str(),
            "GPL-2.0" | "MIT" | "BSD-2-Clause" | "BSD-3-Clause" | "ISC"
        );
    }

    // Artistic-2.0 is relatively permissive
    if dep == "Artistic-2.0" {
        return true;
    }

    // LGPL allows linking from any license
    if dep == "LGPL-2.1" || dep == "LGPL-3.0" {
        // LGPL is compatible when the dependency is used as a library (linking)
        // For simplicity, we consider it compatible with GPL and LGPL variants
        return matches!(
            proj.as_str(),
            "GPL-2.0" | "GPL-3.0" | "LGPL-2.1" | "LGPL-3.0" | "AGPL-3.0"
        );
    }

    // GPL-2.0 is only compatible with GPL-2.0 and LGPL
    if dep == "GPL-2.0" {
        return matches!(proj.as_str(), "GPL-2.0" | "GPL-3.0");
    }

    // GPL-3.0 is only compatible with GPL-3.0 and AGPL-3.0
    if dep == "GPL-3.0" {
        return matches!(proj.as_str(), "GPL-3.0" | "AGPL-3.0");
    }

    // AGPL-3.0 is the most restrictive
    if dep == "AGPL-3.0" {
        return proj == "AGPL-3.0";
    }

    // Unknown combination: consider incompatible
    false
}

/// Parse NuGet licenses - returns empty vec as NuGet doesn't embed licenses
/// in the lock file. License information would need to be fetched from NuGet API.
fn parse_nuget_licenses(_scanner: &Scanner) -> Vec<DependencyLicense> {
    // NuGet doesn't embed licenses in packages.lock.json or .csproj files
    // License info would require fetching from nuget.org API
    Vec::new()
}

/// Parse *.gemspec files for license information
fn parse_gemspec_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();
    let gemspec_files = scanner.files_matching_pattern("*.gemspec");

    // Pre-compile regexes outside the loop
    let license_re = Regex::new(r#"(?:spec|s)\.license\s*=\s*['"]([^'"]+)['"]"#).unwrap();
    let name_re = Regex::new(r#"(?:spec|s)\.name\s*=\s*['"]([^'"]+)['"]"#).unwrap();

    for file in gemspec_files {
        if let Ok(content) = scanner.read_file(&file.path) {
            // Match spec.license = 'MIT' or s.license = "MIT"

            let license = license_re.captures(&content).map(|c| c[1].to_string());
            let name = name_re
                .captures(&content)
                .map(|c| c[1].to_string())
                .unwrap_or_else(|| {
                    file.path
                        .trim_end_matches(".gemspec")
                        .rsplit('/')
                        .next()
                        .unwrap_or("unknown")
                        .to_string()
                });

            licenses.push(DependencyLicense {
                name,
                license: license.map(|l| normalize_license(&l)),
                source_file: file.path.clone(),
            });
        }
    }
    licenses
}

/// Parse *.podspec files for license information
fn parse_podspec_licenses(scanner: &Scanner) -> Vec<DependencyLicense> {
    let mut licenses = Vec::new();
    let podspec_files = scanner.files_matching_pattern("*.podspec");

    // Pre-compile regexes outside the loop
    let license_simple_re = Regex::new(r#"(?:spec|s)\.license\s*=\s*['"]([^'"]+)['"]"#).unwrap();
    let license_hash_re =
        Regex::new(r#"(?:spec|s)\.license\s*=\s*\{[^}]*:type\s*=>\s*['"]([^'"]+)['"]"#).unwrap();
    let name_re = Regex::new(r#"(?:spec|s)\.name\s*=\s*['"]([^'"]+)['"]"#).unwrap();

    for file in podspec_files {
        if let Ok(content) = scanner.read_file(&file.path) {
            // Match s.license = 'MIT' or s.license = { :type => 'MIT' }

            let license = license_simple_re
                .captures(&content)
                .or_else(|| license_hash_re.captures(&content))
                .map(|c| c[1].to_string());
            let name = name_re
                .captures(&content)
                .map(|c| c[1].to_string())
                .unwrap_or_else(|| {
                    file.path
                        .trim_end_matches(".podspec")
                        .rsplit('/')
                        .next()
                        .unwrap_or("unknown")
                        .to_string()
                });

            licenses.push(DependencyLicense {
                name,
                license: license.map(|l| normalize_license(&l)),
                source_file: file.path.clone(),
            });
        }
    }
    licenses
}

/// Parse pubspec.yaml/pubspec.lock for Dart/Flutter - returns empty vec
/// as Pub doesn't embed license information in lock files
fn parse_pubspec_licenses(_scanner: &Scanner) -> Vec<DependencyLicense> {
    // Pub doesn't embed license information in pubspec.lock or pubspec.yaml
    // License info would require fetching from pub.dev API
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::scanner::Scanner;
    use std::fs;
    use tempfile::TempDir;

    // ===== License Detection Tests =====

    #[test]
    fn test_detect_project_license_from_license_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted, free of charge...",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), Some("MIT".to_string()));
    }

    #[test]
    fn test_detect_project_license_apache() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "Apache License\nVersion 2.0, January 2004",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(
            detect_project_license(&scanner),
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_detect_project_license_gpl3() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "GNU General Public License\nVersion 3, 29 June 2007",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(
            detect_project_license(&scanner),
            Some("GPL-3.0".to_string())
        );
    }

    #[test]
    fn test_detect_project_license_gpl2() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "GNU General Public License\nVersion 2, June 1991",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(
            detect_project_license(&scanner),
            Some("GPL-2.0".to_string())
        );
    }

    #[test]
    fn test_detect_project_license_from_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nlicense = \"MIT\"",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), Some("MIT".to_string()));
    }

    #[test]
    fn test_detect_project_license_from_package_json() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","version":"1.0.0","license":"Apache-2.0"}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(
            detect_project_license(&scanner),
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_detect_project_license_from_setup_cfg() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("setup.cfg"),
            "[metadata]\nname = test\nlicense = MIT\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), Some("MIT".to_string()));
    }

    #[test]
    fn test_detect_project_license_from_pyproject_toml() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"test\"\nlicense = \"MIT\"\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), Some("MIT".to_string()));
    }

    #[test]
    fn test_detect_project_license_from_pyproject_toml_poetry() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[tool.poetry]\nname = \"test\"\nlicense = \"Apache-2.0\"\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(
            detect_project_license(&scanner),
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_detect_project_license_none() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), None);
    }

    #[test]
    fn test_detect_license_from_content_isc() {
        let content = "ISC License\n\nCopyright (c) 2024...";
        assert_eq!(
            detect_license_from_content(content),
            Some("ISC".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_mpl() {
        let content = "Mozilla Public License Version 2.0";
        assert_eq!(
            detect_license_from_content(content),
            Some("MPL-2.0".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_unlicense() {
        let content = "This is free and unencumbered software released into the public domain.";
        assert_eq!(
            detect_license_from_content(content),
            Some("Unlicense".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_bsd3() {
        let content = "Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met: neither the name of the copyright holder nor the names of its contributors";
        assert_eq!(
            detect_license_from_content(content),
            Some("BSD-3-Clause".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_lgpl3() {
        let content = "GNU Lesser General Public License\nVersion 3, 29 June 2007";
        assert_eq!(
            detect_license_from_content(content),
            Some("LGPL-3.0".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_lgpl21() {
        let content = "GNU Lesser General Public License\nVersion 2.1, February 1999";
        assert_eq!(
            detect_license_from_content(content),
            Some("LGPL-2.1".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_agpl() {
        let content = "GNU Affero General Public License";
        assert_eq!(
            detect_license_from_content(content),
            Some("AGPL-3.0".to_string())
        );
    }

    #[test]
    fn test_detect_license_from_content_unknown() {
        let content = "Some custom license text that doesn't match anything";
        assert_eq!(detect_license_from_content(content), None);
    }

    // ===== Normalize License Tests =====

    #[test]
    fn test_normalize_license() {
        assert_eq!(normalize_license("mit"), "MIT");
        assert_eq!(normalize_license("MIT"), "MIT");
        assert_eq!(normalize_license("apache-2.0"), "Apache-2.0");
        assert_eq!(normalize_license("Apache 2.0"), "Apache-2.0");
        assert_eq!(normalize_license("gpl-3.0"), "GPL-3.0");
        assert_eq!(normalize_license("gpl-3.0-only"), "GPL-3.0");
        assert_eq!(normalize_license("bsd-2-clause"), "BSD-2-Clause");
        assert_eq!(normalize_license("unlicense"), "Unlicense");
        assert_eq!(normalize_license("SomeCustomLicense"), "SomeCustomLicense");
    }

    // ===== Known License Tests =====

    #[test]
    fn test_is_known_license() {
        assert!(is_known_license("MIT"));
        assert!(is_known_license("mit"));
        assert!(is_known_license("Apache-2.0"));
        assert!(is_known_license("GPL-3.0"));
        assert!(is_known_license("BSD-2-Clause"));
        assert!(is_known_license("ISC"));
        assert!(is_known_license("MPL-2.0"));
        assert!(is_known_license("Unlicense"));
        assert!(!is_known_license("SomeUnknownLicense"));
        assert!(!is_known_license("WTFPL"));
    }

    // ===== Denied/Allowed License Tests =====

    #[test]
    fn test_is_license_denied() {
        let denied = vec!["GPL-3.0".to_string(), "AGPL-3.0".to_string()];
        assert!(is_license_denied("GPL-3.0", &denied));
        assert!(is_license_denied("gpl-3.0", &denied));
        assert!(is_license_denied("AGPL-3.0", &denied));
        assert!(!is_license_denied("MIT", &denied));
        assert!(!is_license_denied("Apache-2.0", &denied));
    }

    #[test]
    fn test_is_license_denied_empty() {
        let denied: Vec<String> = vec![];
        assert!(!is_license_denied("GPL-3.0", &denied));
    }

    #[test]
    fn test_is_license_allowed() {
        let allowed = vec![
            "MIT".to_string(),
            "Apache-2.0".to_string(),
            "BSD-3-Clause".to_string(),
        ];
        assert!(is_license_allowed("MIT", &allowed));
        assert!(is_license_allowed("mit", &allowed));
        assert!(is_license_allowed("Apache-2.0", &allowed));
        assert!(!is_license_allowed("GPL-3.0", &allowed));
    }

    #[test]
    fn test_is_license_allowed_empty() {
        let allowed: Vec<String> = vec![];
        assert!(is_license_allowed("anything", &allowed));
    }

    // ===== Compatibility Tests =====

    #[test]
    fn test_is_compatible_same_license() {
        assert!(is_compatible("MIT", "MIT"));
        assert!(is_compatible("GPL-3.0", "GPL-3.0"));
    }

    #[test]
    fn test_is_compatible_permissive_with_all() {
        // Permissive licenses should be compatible with any project license
        for proj in &["MIT", "Apache-2.0", "GPL-3.0", "AGPL-3.0"] {
            assert!(
                is_compatible(proj, "MIT"),
                "MIT should be compatible with {}",
                proj
            );
            assert!(
                is_compatible(proj, "BSD-2-Clause"),
                "BSD-2-Clause should be compatible with {}",
                proj
            );
            assert!(
                is_compatible(proj, "BSD-3-Clause"),
                "BSD-3-Clause should be compatible with {}",
                proj
            );
            assert!(
                is_compatible(proj, "ISC"),
                "ISC should be compatible with {}",
                proj
            );
            assert!(
                is_compatible(proj, "Unlicense"),
                "Unlicense should be compatible with {}",
                proj
            );
        }
    }

    #[test]
    fn test_is_compatible_apache_with_gpl2() {
        // Apache-2.0 is NOT compatible with GPL-2.0
        assert!(!is_compatible("GPL-2.0", "Apache-2.0"));
        // But compatible with other licenses
        assert!(is_compatible("MIT", "Apache-2.0"));
        assert!(is_compatible("GPL-3.0", "Apache-2.0"));
    }

    #[test]
    fn test_is_compatible_gpl_restrictions() {
        // GPL-2.0 dep can only be used with GPL-2.0 or GPL-3.0 projects
        assert!(is_compatible("GPL-2.0", "GPL-2.0"));
        assert!(is_compatible("GPL-3.0", "GPL-2.0"));
        assert!(!is_compatible("MIT", "GPL-2.0"));
        assert!(!is_compatible("Apache-2.0", "GPL-2.0"));

        // GPL-3.0 dep can only be used with GPL-3.0 or AGPL-3.0 projects
        assert!(is_compatible("GPL-3.0", "GPL-3.0"));
        assert!(is_compatible("AGPL-3.0", "GPL-3.0"));
        assert!(!is_compatible("MIT", "GPL-3.0"));
        assert!(!is_compatible("GPL-2.0", "GPL-3.0"));
    }

    #[test]
    fn test_is_compatible_agpl_restrictions() {
        // AGPL-3.0 is only compatible with AGPL-3.0 projects
        assert!(is_compatible("AGPL-3.0", "AGPL-3.0"));
        assert!(!is_compatible("MIT", "AGPL-3.0"));
        assert!(!is_compatible("GPL-3.0", "AGPL-3.0"));
    }

    #[test]
    fn test_is_compatible_lgpl() {
        // LGPL allows linking from GPL/LGPL/AGPL
        assert!(is_compatible("GPL-3.0", "LGPL-3.0"));
        assert!(is_compatible("AGPL-3.0", "LGPL-3.0"));
        assert!(!is_compatible("MIT", "LGPL-3.0"));
    }

    #[test]
    fn test_is_compatible_mpl() {
        // MPL-2.0 is weak copyleft
        assert!(is_compatible("GPL-3.0", "MPL-2.0"));
        assert!(is_compatible("AGPL-3.0", "MPL-2.0"));
        assert!(!is_compatible("MIT", "MPL-2.0"));
    }

    // ===== Parsing Tests =====

    #[test]
    fn test_parse_cargo_toml_licenses() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tempfile = "3"
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_cargo_toml_licenses(&scanner);
        assert_eq!(licenses.len(), 3);
        assert!(licenses.iter().any(|l| l.name == "serde"));
        assert!(licenses.iter().any(|l| l.name == "tokio"));
        assert!(licenses.iter().any(|l| l.name == "tempfile"));
    }

    #[test]
    fn test_parse_cargo_toml_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_cargo_toml_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_package_json_licenses() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{
  "name": "test",
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_package_json_licenses(&scanner);
        assert_eq!(licenses.len(), 3);
        assert!(licenses.iter().any(|l| l.name == "express"));
        assert!(licenses.iter().any(|l| l.name == "lodash"));
        assert!(licenses.iter().any(|l| l.name == "jest"));
    }

    #[test]
    fn test_parse_package_json_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_package_json_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_requirements_txt_licenses() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements.txt"),
            "requests==2.28.0\nflask>=2.0\n# comment\n\n-r other.txt\nnumpy~=1.24\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_requirements_txt_licenses(&scanner);
        assert_eq!(licenses.len(), 3);
        assert!(licenses.iter().any(|l| l.name == "requests"));
        assert!(licenses.iter().any(|l| l.name == "flask"));
        assert!(licenses.iter().any(|l| l.name == "numpy"));
    }

    #[test]
    fn test_parse_requirements_txt_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_requirements_txt_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_go_mod_licenses() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("go.mod"),
            r#"module example.com/myproject

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
    github.com/go-sql-driver/mysql v1.7.0
    golang.org/x/net v0.17.0 // indirect
)

require github.com/stretchr/testify v1.8.4
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_go_mod_licenses(&scanner);
        assert_eq!(licenses.len(), 4);
        assert!(
            licenses
                .iter()
                .any(|l| l.name == "github.com/gin-gonic/gin")
        );
        assert!(
            licenses
                .iter()
                .any(|l| l.name == "github.com/go-sql-driver/mysql")
        );
        assert!(licenses.iter().any(|l| l.name == "golang.org/x/net"));
        assert!(
            licenses
                .iter()
                .any(|l| l.name == "github.com/stretchr/testify")
        );
    }

    #[test]
    fn test_parse_go_mod_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_go_mod_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_extract_python_package_name() {
        assert_eq!(
            extract_python_package_name("requests==2.28.0"),
            Some("requests".to_string())
        );
        assert_eq!(
            extract_python_package_name("Flask>=2.0"),
            Some("flask".to_string())
        );
        assert_eq!(
            extract_python_package_name("numpy~=1.24"),
            Some("numpy".to_string())
        );
        assert_eq!(
            extract_python_package_name("urllib3[socks]>=1.26"),
            Some("urllib3".to_string())
        );
        assert_eq!(
            extract_python_package_name("simplepkg"),
            Some("simplepkg".to_string())
        );
        assert_eq!(extract_python_package_name("# comment"), None);
        assert_eq!(extract_python_package_name(""), None);
    }

    // ===== Collect Dependency Licenses =====

    #[test]
    fn test_collect_dependency_licenses_multiple_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","dependencies":{"express":"^4.0"}}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = collect_dependency_licenses(&scanner);
        assert!(licenses.len() >= 2);
        assert!(licenses.iter().any(|l| l.name == "serde"));
        assert!(licenses.iter().any(|l| l.name == "express"));
    }

    // ===== Integration / Rule Run Tests =====

    #[tokio::test]
    async fn test_license_rules_no_license_detected() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let config = Config::default();
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(findings.iter().any(|f| f.rule_id == "LIC001"));
    }

    #[tokio::test]
    async fn test_license_rules_with_license_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted...",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let config = Config::default();
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(!findings.iter().any(|f| f.rule_id == "LIC001"));
    }

    #[tokio::test]
    async fn test_license_rules_dependency_no_license() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted...",
        )
        .unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let config = Config::default();
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        // serde doesn't have license info from Cargo.toml alone
        assert!(findings.iter().any(|f| f.rule_id == "LIC004"));
    }

    #[tokio::test]
    async fn test_license_rules_denied_license() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted...",
        )
        .unwrap();
        // Create a package.json with a node_modules dep that has a denied license
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","license":"MIT","dependencies":{"badpkg":"^1.0"}}"#,
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("node_modules/badpkg")).unwrap();
        fs::write(
            tmp.path().join("node_modules/badpkg/package.json"),
            r#"{"name":"badpkg","license":"GPL-3.0"}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let mut config = Config::default();
        config.license_compliance.denied_licenses = vec!["GPL-3.0".to_string()];
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(findings.iter().any(|f| f.rule_id == "LIC002"
            && f.severity == Severity::Critical
            && f.message.contains("denied")));
    }

    #[tokio::test]
    async fn test_license_rules_not_in_allowed_list() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted...",
        )
        .unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","license":"MIT","dependencies":{"mplpkg":"^1.0"}}"#,
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("node_modules/mplpkg")).unwrap();
        fs::write(
            tmp.path().join("node_modules/mplpkg/package.json"),
            r#"{"name":"mplpkg","license":"MPL-2.0"}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let mut config = Config::default();
        config.license_compliance.allowed_licenses =
            vec!["MIT".to_string(), "Apache-2.0".to_string()];
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "LIC002" && f.message.contains("not in the allowed list"))
        );
    }

    #[tokio::test]
    async fn test_license_rules_unknown_license() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted...",
        )
        .unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","license":"MIT","dependencies":{"custpkg":"^1.0"}}"#,
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("node_modules/custpkg")).unwrap();
        fs::write(
            tmp.path().join("node_modules/custpkg/package.json"),
            r#"{"name":"custpkg","license":"WTFPL"}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let config = Config::default();
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(findings.iter().any(|f| f.rule_id == "LIC003"));
    }

    #[tokio::test]
    async fn test_license_rules_incompatible_license() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE"),
            "MIT License\n\nPermission is hereby granted...",
        )
        .unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","license":"MIT","dependencies":{"gplpkg":"^1.0"}}"#,
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("node_modules/gplpkg")).unwrap();
        fs::write(
            tmp.path().join("node_modules/gplpkg/package.json"),
            r#"{"name":"gplpkg","license":"GPL-3.0"}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let config = Config::default();
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(findings.iter().any(|f| f.rule_id == "LIC002"
            && f.severity == Severity::Critical
            && f.message.contains("incompatible")));
    }

    #[tokio::test]
    async fn test_license_rules_disabled() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let mut config = Config::default();
        config.license_compliance.enabled = false;
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_license_rules_rule_disabled_in_config() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let mut config = Config::default();
        config.rules.insert(
            "licenses/compliance".to_string(),
            crate::config::RuleConfig {
                enabled: false,
                severity: None,
            },
        );
        let rules = LicenseRules;
        let findings = rules.run(&scanner, &config).await.unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn test_license_rules_category_name() {
        let rules = LicenseRules;
        assert_eq!(rules.name(), "licenses");
    }

    #[test]
    fn test_parse_cargo_toml_with_build_deps() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"

[build-dependencies]
cc = "1.0"
"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_cargo_toml_licenses(&scanner);
        assert_eq!(licenses.len(), 2);
        assert!(licenses.iter().any(|l| l.name == "serde"));
        assert!(licenses.iter().any(|l| l.name == "cc"));
    }

    #[test]
    fn test_parse_cargo_toml_invalid() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "invalid [[[toml").unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_cargo_toml_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_package_json_invalid() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), "not valid json").unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_package_json_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_package_json_with_node_modules() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","dependencies":{"mypkg":"^1.0"}}"#,
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("node_modules/mypkg")).unwrap();
        fs::write(
            tmp.path().join("node_modules/mypkg/package.json"),
            r#"{"name":"mypkg","license":"MIT"}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_package_json_licenses(&scanner);
        assert_eq!(licenses.len(), 1);
        assert_eq!(licenses[0].license, Some("MIT".to_string()));
    }

    #[test]
    fn test_parse_requirements_txt_with_extras() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("requirements.txt"),
            "requests[security]>=2.28.0\nurllib3!=1.25.0\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_requirements_txt_licenses(&scanner);
        assert_eq!(licenses.len(), 2);
        assert!(licenses.iter().any(|l| l.name == "requests"));
        assert!(licenses.iter().any(|l| l.name == "urllib3"));
    }

    #[test]
    fn test_is_compatible_artistic() {
        assert!(is_compatible("MIT", "Artistic-2.0"));
        assert!(is_compatible("GPL-3.0", "Artistic-2.0"));
    }

    #[test]
    fn test_is_compatible_unknown_combination() {
        // Unknown licenses should be considered incompatible
        assert!(!is_compatible("MIT", "SomeUnknownLicense"));
    }

    #[test]
    fn test_detect_license_from_license_md() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("LICENSE.md"),
            "# MIT License\n\nPermission is hereby granted, free of charge...",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), Some("MIT".to_string()));
    }

    #[test]
    fn test_detect_license_from_licence_uk_spelling() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("LICENCE"), "Apache License\nVersion 2.0").unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(
            detect_project_license(&scanner),
            Some("Apache-2.0".to_string())
        );
    }

    #[test]
    fn test_detect_project_license_from_pyproject_toml_table() {
        // Cover the { text = "..." } variant for PEP 621
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"test\"\n\n[project.license]\ntext = \"MIT\"\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        assert_eq!(detect_project_license(&scanner), Some("MIT".to_string()));
    }

    #[test]
    fn test_detect_license_from_content_bsd2_clause() {
        let content = "Redistribution and use in source and binary forms, with or without modification, are permitted provided that this list of conditions and the following disclaimer.";
        assert_eq!(
            detect_license_from_content(content),
            Some("BSD-2-Clause".to_string())
        );
    }

    #[test]
    fn test_extract_python_package_name_bracket_no_close() {
        // Package name with bracket but no closing bracket - treated as raw name
        assert_eq!(
            extract_python_package_name("broken[extra"),
            Some("broken[extra".to_string())
        );
    }

    #[test]
    fn test_extract_python_package_name_no_version() {
        assert_eq!(
            extract_python_package_name("simplepkg"),
            Some("simplepkg".to_string())
        );
    }

    #[test]
    fn test_parse_cargo_toml_read_error() {
        // Cargo.toml exists but can't parse
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "invalid [[[toml content").unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_cargo_toml_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_package_json_read_error() {
        // package.json exists but is invalid JSON
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), "not valid json {{{").unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_package_json_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_go_mod_empty_require_block() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("go.mod"),
            "module test\n\ngo 1.21\n\nrequire (\n)\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_go_mod_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_parse_go_mod_comment_in_require() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("go.mod"),
            "module test\n\ngo 1.21\n\nrequire (\n// a comment\ngithub.com/pkg/errors v0.9.1\n)\n",
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_go_mod_licenses(&scanner);
        assert_eq!(licenses.len(), 1);
        assert!(licenses.iter().any(|l| l.name == "github.com/pkg/errors"));
    }

    #[test]
    fn test_collect_dep_licenses_empty() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = collect_dependency_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    #[test]
    fn test_is_compatible_0bsd_with_anything() {
        assert!(is_compatible("MIT", "0BSD"));
        assert!(is_compatible("GPL-3.0", "0BSD"));
        assert!(is_compatible("AGPL-3.0", "0BSD"));
    }

    #[test]
    fn test_is_compatible_cc0_with_anything() {
        assert!(is_compatible("MIT", "CC0-1.0"));
        assert!(is_compatible("GPL-3.0", "CC0-1.0"));
    }

    #[test]
    fn test_normalize_license_0bsd() {
        assert_eq!(normalize_license("0bsd"), "0BSD");
    }

    #[test]
    fn test_normalize_license_cc0() {
        assert_eq!(normalize_license("cc0-1.0"), "CC0-1.0");
    }

    #[test]
    fn test_normalize_license_zlib() {
        assert_eq!(normalize_license("zlib"), "Zlib");
    }

    #[test]
    fn test_normalize_license_artistic() {
        assert_eq!(normalize_license("artistic-2.0"), "Artistic-2.0");
    }

    #[test]
    fn test_normalize_license_bsl() {
        assert_eq!(normalize_license("bsl-1.0"), "BSL-1.0");
    }

    // ===== Maven (pom.xml) License Tests =====

    #[test]
    fn test_parse_pom_xml_licenses_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pom.xml"),
            r#"<project>
  <licenses>
    <license>
      <name>Apache-2.0</name>
    </license>
  </licenses>
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
        let licenses = parse_pom_xml_licenses(&scanner);
        assert_eq!(licenses.len(), 1);
        assert_eq!(licenses[0].name, "org.springframework:spring-core");
        assert_eq!(licenses[0].license, Some("Apache-2.0".to_string()));
        assert_eq!(licenses[0].source_file, "pom.xml");
    }

    #[test]
    fn test_parse_pom_xml_licenses_no_file() {
        let tmp = TempDir::new().unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_pom_xml_licenses(&scanner);
        assert!(licenses.is_empty());
    }

    // ===== Composer License Tests =====

    #[test]
    fn test_parse_composer_json_licenses_from_lock() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.lock"),
            r#"{
    "packages": [
        {
            "name": "monolog/monolog",
            "version": "2.8.0",
            "license": ["MIT"]
        }
    ],
    "packages-dev": []
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_composer_json_licenses(&scanner);
        assert_eq!(licenses.len(), 1);
        assert_eq!(licenses[0].name, "monolog/monolog");
        assert_eq!(licenses[0].license, Some("MIT".to_string()));
        assert_eq!(licenses[0].source_file, "composer.lock");
    }

    #[test]
    fn test_parse_composer_json_licenses_from_json_fallback() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("composer.json"),
            r#"{
    "license": "MIT",
    "require": {
        "php": ">=8.0",
        "monolog/monolog": "^2.8"
    }
}"#,
        )
        .unwrap();
        let scanner = Scanner::new(tmp.path().to_path_buf());
        let licenses = parse_composer_json_licenses(&scanner);
        assert_eq!(licenses.len(), 1);
        assert_eq!(licenses[0].name, "monolog/monolog");
        assert_eq!(licenses[0].license, Some("MIT".to_string()));
        assert_eq!(licenses[0].source_file, "composer.json");
    }
}
