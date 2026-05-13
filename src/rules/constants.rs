//! Rule category constants and validation

use colored::Colorize;

/// Valid category names for --only and --skip options.
/// Must match the categories registered in `src/rules/engine.rs`.
pub const VALID_CATEGORIES: &[&str] = &[
    "secrets",
    "files",
    "docs",
    "security",
    "workflows",
    "quality",
    "dependencies",
    "licenses",
    "docker",
    "git",
    "custom",
    "codeowners",
    "history",
    "issues",
    "metadata",
];

/// Check if a category name is valid
pub fn is_valid_category(name: &str) -> bool {
    VALID_CATEGORIES.contains(&name)
}

/// Filter a list of categories, returning only valid ones and printing warnings for invalid ones
pub fn filter_valid_categories(categories: Vec<String>) -> Vec<String> {
    let mut valid = Vec::new();
    for category in categories {
        if is_valid_category(&category) {
            valid.push(category);
        } else {
            eprintln!(
                "{} Unknown category '{}' ignored. Valid categories: {}",
                "Warning:".yellow(),
                category.cyan(),
                VALID_CATEGORIES.join(", ").dimmed()
            );
        }
    }
    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_categories_list() {
        assert_eq!(VALID_CATEGORIES.len(), 15);
        for expected in [
            "secrets",
            "files",
            "docs",
            "security",
            "workflows",
            "quality",
            "dependencies",
            "licenses",
            "docker",
            "git",
            "custom",
            "codeowners",
            "history",
            "issues",
            "metadata",
        ] {
            assert!(
                VALID_CATEGORIES.contains(&expected),
                "missing category: {expected}"
            );
        }
    }

    #[test]
    fn test_is_valid_category_returns_true_for_valid() {
        for valid in VALID_CATEGORIES {
            assert!(is_valid_category(valid), "expected '{valid}' to be valid");
        }
    }

    #[test]
    fn test_is_valid_category_returns_false_for_invalid() {
        assert!(!is_valid_category("invalid"));
        assert!(!is_valid_category("unknown"));
        assert!(!is_valid_category(""));
        assert!(!is_valid_category("SECRETS")); // case-sensitive
        assert!(!is_valid_category("Files"));
    }

    #[test]
    fn test_filter_valid_categories_keeps_valid() {
        let input = vec![
            "secrets".to_string(),
            "files".to_string(),
            "docs".to_string(),
        ];
        let result = filter_valid_categories(input);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"secrets".to_string()));
        assert!(result.contains(&"files".to_string()));
        assert!(result.contains(&"docs".to_string()));
    }

    #[test]
    fn test_filter_valid_categories_removes_invalid() {
        let input = vec![
            "secrets".to_string(),
            "invalid".to_string(),
            "docs".to_string(),
        ];
        let result = filter_valid_categories(input);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"secrets".to_string()));
        assert!(result.contains(&"docs".to_string()));
        assert!(!result.contains(&"invalid".to_string()));
    }

    #[test]
    fn test_filter_valid_categories_empty_input() {
        let input: Vec<String> = vec![];
        let result = filter_valid_categories(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_valid_categories_all_invalid() {
        let input = vec!["invalid".to_string(), "unknown".to_string()];
        let result = filter_valid_categories(input);
        assert!(result.is_empty());
    }
}
