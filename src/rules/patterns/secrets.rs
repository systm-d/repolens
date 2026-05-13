//! Secret detection patterns

use lazy_static::lazy_static;
use regex::Regex;

/// A pattern for detecting secrets
pub struct SecretPattern {
    pub name: &'static str,
    pub description: &'static str,
    pub regex: Regex,
}

lazy_static! {
    /// Collection of secret patterns to detect
    pub static ref SECRET_PATTERNS: Vec<SecretPattern> = vec![
        // GitHub tokens
        SecretPattern {
            name: "GitHub Personal Access Token",
            description: "GitHub personal access tokens start with 'ghp_'",
            regex: Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap(),
        },
        SecretPattern {
            name: "GitHub OAuth Token",
            description: "GitHub OAuth tokens start with 'gho_'",
            regex: Regex::new(r"gho_[A-Za-z0-9]{36}").unwrap(),
        },
        SecretPattern {
            name: "GitHub User-to-Server Token",
            description: "GitHub user-to-server tokens start with 'ghu_'",
            regex: Regex::new(r"ghu_[A-Za-z0-9]{36}").unwrap(),
        },
        SecretPattern {
            name: "GitHub Server-to-Server Token",
            description: "GitHub server-to-server tokens start with 'ghs_'",
            regex: Regex::new(r"ghs_[A-Za-z0-9]{36}").unwrap(),
        },
        SecretPattern {
            name: "GitHub Refresh Token",
            description: "GitHub refresh tokens start with 'ghr_'",
            regex: Regex::new(r"ghr_[A-Za-z0-9]{36}").unwrap(),
        },

        // AWS
        SecretPattern {
            name: "AWS Access Key ID",
            description: "AWS access keys start with 'AKIA'",
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        },
        SecretPattern {
            name: "AWS Secret Access Key",
            description: "AWS secret access keys are 40 character strings",
            regex: Regex::new(r#"(?i)(aws_secret_access_key|aws_secret_key)\s*[=:]\s*['"]?[A-Za-z0-9/+=]{40}['"]?"#).unwrap(),
        },

        // Stripe
        SecretPattern {
            name: "Stripe Live Secret Key",
            description: "Stripe live secret keys start with 'sk_live_'",
            regex: Regex::new(r"sk_live_[0-9a-zA-Z]{24,}").unwrap(),
        },
        SecretPattern {
            name: "Stripe Test Secret Key",
            description: "Stripe test secret keys start with 'sk_test_'",
            regex: Regex::new(r"sk_test_[0-9a-zA-Z]{24,}").unwrap(),
        },
        SecretPattern {
            name: "Stripe Restricted Key",
            description: "Stripe restricted keys start with 'rk_live_' or 'rk_test_'",
            regex: Regex::new(r"rk_(live|test)_[0-9a-zA-Z]{24,}").unwrap(),
        },

        // Slack
        SecretPattern {
            name: "Slack Token",
            description: "Slack tokens start with 'xox'",
            regex: Regex::new(r"xox[baprs]-[0-9a-zA-Z-]{10,48}").unwrap(),
        },

        // Google
        SecretPattern {
            name: "Google API Key",
            description: "Google API keys start with 'AIza'",
            regex: Regex::new(r"AIza[0-9A-Za-z\-_]{35}").unwrap(),
        },
        SecretPattern {
            name: "Google OAuth Token",
            description: "Google OAuth tokens start with 'ya29.'",
            regex: Regex::new(r"ya29\.[0-9A-Za-z\-_]+").unwrap(),
        },

        // Firebase
        SecretPattern {
            name: "Firebase Cloud Messaging",
            description: "Firebase server keys",
            regex: Regex::new(r"AAAA[A-Za-z0-9_-]{7}:[A-Za-z0-9_-]{140}").unwrap(),
        },

        // Twilio
        SecretPattern {
            name: "Twilio API Key",
            description: "Twilio API keys start with 'SK'",
            regex: Regex::new(r"SK[0-9a-fA-F]{32}").unwrap(),
        },

        // SendGrid
        SecretPattern {
            name: "SendGrid API Key",
            description: "SendGrid API keys start with 'SG.'",
            regex: Regex::new(r"SG\.[0-9A-Za-z\-_]{22}\.[0-9A-Za-z\-_]{43}").unwrap(),
        },

        // Mailgun
        SecretPattern {
            name: "Mailgun API Key",
            description: "Mailgun API keys start with 'key-'",
            regex: Regex::new(r"key-[0-9a-zA-Z]{32}").unwrap(),
        },

        // npm
        SecretPattern {
            name: "npm Token",
            description: "npm tokens start with 'npm_'",
            regex: Regex::new(r"npm_[A-Za-z0-9]{36}").unwrap(),
        },

        // Discord
        SecretPattern {
            name: "Discord Token",
            description: "Discord bot tokens",
            regex: Regex::new(r"[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}").unwrap(),
        },

        // Generic patterns
        SecretPattern {
            name: "Private Key",
            description: "PEM encoded private key",
            regex: Regex::new(r"-----BEGIN (RSA|DSA|EC|OPENSSH) PRIVATE KEY-----").unwrap(),
        },
        SecretPattern {
            name: "JWT Token",
            description: "JSON Web Token",
            regex: Regex::new(r"eyJ[A-Za-z0-9-_=]+\.eyJ[A-Za-z0-9-_=]+\.[A-Za-z0-9-_.+/=]+").unwrap(),
        },

        // Database connection strings
        SecretPattern {
            name: "MongoDB Connection String",
            description: "MongoDB connection string with credentials",
            regex: Regex::new(r"mongodb(\+srv)?://[^:]+:[^@]+@").unwrap(),
        },
        SecretPattern {
            name: "PostgreSQL Connection String",
            description: "PostgreSQL connection string with credentials",
            regex: Regex::new(r"postgres(ql)?://[^:]+:[^@]+@").unwrap(),
        },
        SecretPattern {
            name: "MySQL Connection String",
            description: "MySQL connection string with credentials",
            regex: Regex::new(r"mysql://[^:]+:[^@]+@").unwrap(),
        },
        SecretPattern {
            name: "Redis Connection String",
            description: "Redis connection string with credentials",
            regex: Regex::new(r"redis://[^:]+:[^@]+@").unwrap(),
        },

        // Generic credential patterns
        SecretPattern {
            name: "Generic Password Assignment",
            description: "Password assigned in code",
            regex: Regex::new(r#"(?i)(password|passwd|pwd)\s*[=:]\s*['"][^'"]{8,}['"]"#).unwrap(),
        },
        SecretPattern {
            name: "Generic API Key Assignment",
            description: "API key assigned in code",
            regex: Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[=:]\s*['"][^'"]{16,}['"]"#).unwrap(),
        },
        SecretPattern {
            name: "Generic Secret Assignment",
            description: "Secret assigned in code",
            regex: Regex::new(r#"(?i)(secret[_-]?key|secretkey)\s*[=:]\s*['"][^'"]{16,}['"]"#).unwrap(),
        },
        SecretPattern {
            name: "Generic Token Assignment",
            description: "Token assigned in code",
            regex: Regex::new(r#"(?i)(access[_-]?token|auth[_-]?token)\s*[=:]\s*['"][^'"]{16,}['"]"#).unwrap(),
        },

        // URL with credentials
        SecretPattern {
            name: "URL with Embedded Credentials",
            description: "URL containing username:password",
            regex: Regex::new(r"https?://[^:]+:[^@]+@[^/]+").unwrap(),
        },
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_token_detection() {
        let pattern = &SECRET_PATTERNS[0]; // GitHub PAT
        assert!(
            pattern
                .regex
                .is_match("ghp_abcdefghijklmnopqrstuvwxyz1234567890")
        );
        assert!(!pattern.regex.is_match("ghp_short"));
    }

    #[test]
    fn test_aws_key_detection() {
        let pattern = &SECRET_PATTERNS[5]; // AWS Access Key
        assert!(pattern.regex.is_match("AKIAIOSFODNN7EXAMPLE"));
        assert!(!pattern.regex.is_match("NOTANAWSKEY12345678"));
    }

    #[test]
    fn test_stripe_key_detection() {
        let pattern = &SECRET_PATTERNS[7]; // Stripe Live Key
        assert!(pattern.regex.is_match("sk_live_abcdefghijklmnopqrstuvwx"));
        assert!(!pattern.regex.is_match("sk_live_short"));
    }
}
