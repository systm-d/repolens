//! # Branding Configuration
//!
//! Loads and validates the optional branding TOML used by the PDF report
//! renderer (logo, colors, fonts, header/footer text). Invalid values do not
//! prevent PDF generation: `validate_and_apply_defaults` emits a `WARN` and
//! falls back to the default value so a report is always produced.

use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use tracing::warn;

use crate::error::{ConfigError, RepoLensError};

/// Default primary color (Atlassian-blue).
pub const DEFAULT_PRIMARY_COLOR: &str = "#0052CC";
/// Default secondary color (Atlassian-navy).
pub const DEFAULT_SECONDARY_COLOR: &str = "#172B4D";
/// Default body text color.
pub const DEFAULT_TEXT_COLOR: &str = "#000000";
/// Default font family (one of printpdf's built-in Type1 fonts).
pub const DEFAULT_FONT_FAMILY: &str = "Helvetica";

/// Maximum logo size on disk (5 MB).
pub const MAX_LOGO_BYTES: u64 = 5 * 1024 * 1024;
/// Maximum length for header/footer text.
pub const MAX_HEADER_FOOTER_CHARS: usize = 200;
/// Maximum length for the cover subtitle.
pub const MAX_COVER_SUBTITLE_CHARS: usize = 100;

lazy_static! {
    static ref HEX_COLOR_RE: Regex =
        Regex::new(r"^#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?$").expect("valid regex");
}

/// User-supplied branding configuration for PDF reports.
///
/// All fields are optional. After construction, callers must invoke
/// [`BrandingConfig::validate_and_apply_defaults`] before consuming the
/// values; that step replaces any invalid input with safe defaults and
/// logs a `WARN`.
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct BrandingConfig {
    /// Path to a PNG/JPG logo file (max 5 MB).
    pub logo_path: Option<PathBuf>,
    /// Primary brand color in `#RRGGBB` or `#RRGGBBAA` form.
    pub primary_color: Option<String>,
    /// Secondary brand color in `#RRGGBB` or `#RRGGBBAA` form.
    pub secondary_color: Option<String>,
    /// Body text color in `#RRGGBB` or `#RRGGBBAA` form.
    pub text_color: Option<String>,
    /// Preferred font family. Falls back to Helvetica if not bundled.
    pub font_family: Option<String>,
    /// Footer text rendered on every content page (max 200 chars).
    pub footer_text: Option<String>,
    /// Header text rendered on every content page (max 200 chars; empty = omitted).
    pub header_text: Option<String>,
    /// Subtitle line on the cover page (max 100 chars).
    pub cover_subtitle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BrandingFile {
    branding: Option<BrandingConfig>,
}

impl BrandingConfig {
    /// Load a [`BrandingConfig`] from a TOML file.
    ///
    /// Accepts either a top-level `[branding]` table or a flat table
    /// containing the branding keys directly.
    pub fn load_from_file(path: &Path) -> Result<Self, RepoLensError> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            RepoLensError::Config(ConfigError::FileRead {
                path: path.display().to_string(),
                source: e,
            })
        })?;

        if let Ok(wrapper) = toml::from_str::<BrandingFile>(&raw) {
            if let Some(cfg) = wrapper.branding {
                return Ok(cfg);
            }
        }

        let cfg: Self = toml::from_str(&raw).map_err(|e| {
            RepoLensError::Config(ConfigError::Parse {
                message: format!("branding config '{}': {}", path.display(), e),
            })
        })?;

        Ok(cfg)
    }

    /// Validate fields and replace invalid values with safe defaults.
    ///
    /// Emits `tracing::warn!` for each correction so users notice issues
    /// without losing the PDF output.
    pub fn validate_and_apply_defaults(&mut self) {
        validate_color_field(
            &mut self.primary_color,
            "primary_color",
            DEFAULT_PRIMARY_COLOR,
        );
        validate_color_field(
            &mut self.secondary_color,
            "secondary_color",
            DEFAULT_SECONDARY_COLOR,
        );
        validate_color_field(&mut self.text_color, "text_color", DEFAULT_TEXT_COLOR);

        if let Some(ref mut family) = self.font_family {
            if family.trim().is_empty() {
                warn!("branding: empty font_family, falling back to {DEFAULT_FONT_FAMILY}");
                *family = DEFAULT_FONT_FAMILY.to_string();
            }
        }

        if let Some(ref logo) = self.logo_path.clone() {
            match std::fs::metadata(logo) {
                Ok(meta) if meta.len() > MAX_LOGO_BYTES => {
                    warn!(
                        "branding: logo '{}' is {} bytes (> 5 MB limit), ignoring",
                        logo.display(),
                        meta.len()
                    );
                    self.logo_path = None;
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "branding: logo '{}' not readable ({}), ignoring",
                        logo.display(),
                        e
                    );
                    self.logo_path = None;
                }
            }
        }

        if let Some(ref mut text) = self.header_text {
            truncate_with_warn(text, MAX_HEADER_FOOTER_CHARS, "header_text");
        }
        if let Some(ref mut text) = self.footer_text {
            truncate_with_warn(text, MAX_HEADER_FOOTER_CHARS, "footer_text");
        }
        if let Some(ref mut text) = self.cover_subtitle {
            truncate_with_warn(text, MAX_COVER_SUBTITLE_CHARS, "cover_subtitle");
        }

        // Apply defaults for any field still missing after validation.
        self.primary_color
            .get_or_insert_with(|| DEFAULT_PRIMARY_COLOR.to_string());
        self.secondary_color
            .get_or_insert_with(|| DEFAULT_SECONDARY_COLOR.to_string());
        self.text_color
            .get_or_insert_with(|| DEFAULT_TEXT_COLOR.to_string());
        self.font_family
            .get_or_insert_with(|| DEFAULT_FONT_FAMILY.to_string());
    }

    /// Convenience: build a fully-defaulted instance ready for rendering.
    pub fn defaults() -> Self {
        let mut cfg = Self::default();
        cfg.validate_and_apply_defaults();
        cfg
    }
}

fn validate_color_field(field: &mut Option<String>, name: &str, default: &str) {
    if let Some(value) = field.as_ref() {
        if !HEX_COLOR_RE.is_match(value) {
            warn!("branding: invalid hex color for {name} ({value:?}), using default {default}");
            *field = Some(default.to_string());
        }
    }
}

fn truncate_with_warn(text: &mut String, max: usize, name: &str) {
    if text.chars().count() > max {
        warn!(
            "branding: {name} exceeds {max} chars ({} given), truncating",
            text.chars().count()
        );
        let truncated: String = text.chars().take(max).collect();
        *text = truncated;
    }
}

/// Convert a `#RRGGBB` or `#RRGGBBAA` color string into the `r g b` triple
/// expected by the PDF `rg`/`RG` operators (each channel scaled to `[0, 1]`,
/// rounded to three decimal places).
///
/// Returns `None` if the color is not a valid hex string.
pub fn hex_to_pdf_rgb(hex: &str) -> Option<(f32, f32, f32)> {
    if !HEX_COLOR_RE.is_match(hex) {
        return None;
    }
    let bytes = hex.as_bytes();
    let r = u8::from_str_radix(std::str::from_utf8(&bytes[1..3]).ok()?, 16).ok()?;
    let g = u8::from_str_radix(std::str::from_utf8(&bytes[3..5]).ok()?, 16).ok()?;
    let b = u8::from_str_radix(std::str::from_utf8(&bytes[5..7]).ok()?, 16).ok()?;
    Some((
        round3(f32::from(r) / 255.0),
        round3(f32::from(g) / 255.0),
        round3(f32::from(b) / 255.0),
    ))
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_toml(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("create tempfile");
        file.write_all(content.as_bytes()).expect("write tempfile");
        file
    }

    #[test]
    fn parses_fully_populated_table() {
        let toml = r##"
            [branding]
            logo_path = "assets/logo.png"
            primary_color = "#0052CC"
            secondary_color = "#172B4D"
            text_color = "#111111"
            font_family = "Inter"
            footer_text = "Confidential — Acme"
            header_text = "Acme Corp"
            cover_subtitle = "Q2 2026 Compliance"
        "##;
        let file = write_toml(toml);
        let cfg = BrandingConfig::load_from_file(file.path()).expect("load");
        assert_eq!(cfg.logo_path, Some(PathBuf::from("assets/logo.png")));
        assert_eq!(cfg.primary_color.as_deref(), Some("#0052CC"));
        assert_eq!(cfg.secondary_color.as_deref(), Some("#172B4D"));
        assert_eq!(cfg.text_color.as_deref(), Some("#111111"));
        assert_eq!(cfg.font_family.as_deref(), Some("Inter"));
        assert_eq!(cfg.footer_text.as_deref(), Some("Confidential — Acme"));
        assert_eq!(cfg.header_text.as_deref(), Some("Acme Corp"));
        assert_eq!(cfg.cover_subtitle.as_deref(), Some("Q2 2026 Compliance"));
    }

    #[test]
    fn parses_flat_table_without_branding_header() {
        let toml = r##"
            primary_color = "#112233"
            footer_text = "Hi"
        "##;
        let file = write_toml(toml);
        let cfg = BrandingConfig::load_from_file(file.path()).expect("load");
        assert_eq!(cfg.primary_color.as_deref(), Some("#112233"));
        assert_eq!(cfg.footer_text.as_deref(), Some("Hi"));
    }

    #[test]
    fn defaults_fill_missing_optional_fields() {
        let mut cfg = BrandingConfig::default();
        cfg.validate_and_apply_defaults();
        assert_eq!(cfg.primary_color.as_deref(), Some(DEFAULT_PRIMARY_COLOR));
        assert_eq!(
            cfg.secondary_color.as_deref(),
            Some(DEFAULT_SECONDARY_COLOR)
        );
        assert_eq!(cfg.text_color.as_deref(), Some(DEFAULT_TEXT_COLOR));
        assert_eq!(cfg.font_family.as_deref(), Some(DEFAULT_FONT_FAMILY));
    }

    #[test]
    fn invalid_hex_resets_to_default() {
        let mut cfg = BrandingConfig {
            primary_color: Some("#ZZZZZZ".to_string()),
            secondary_color: Some("not-a-color".to_string()),
            text_color: Some("#GGGGGG".to_string()),
            ..Default::default()
        };
        cfg.validate_and_apply_defaults();
        assert_eq!(cfg.primary_color.as_deref(), Some(DEFAULT_PRIMARY_COLOR));
        assert_eq!(
            cfg.secondary_color.as_deref(),
            Some(DEFAULT_SECONDARY_COLOR)
        );
        assert_eq!(cfg.text_color.as_deref(), Some(DEFAULT_TEXT_COLOR));
    }

    #[test]
    fn valid_alpha_channel_hex_is_preserved() {
        let mut cfg = BrandingConfig {
            primary_color: Some("#0052CCFF".to_string()),
            ..Default::default()
        };
        cfg.validate_and_apply_defaults();
        assert_eq!(cfg.primary_color.as_deref(), Some("#0052CCFF"));
    }

    #[test]
    fn missing_logo_file_is_dropped() {
        let mut cfg = BrandingConfig {
            logo_path: Some(PathBuf::from("/nonexistent/logo.png")),
            ..Default::default()
        };
        cfg.validate_and_apply_defaults();
        assert!(cfg.logo_path.is_none());
    }

    #[test]
    fn header_footer_truncated_to_max_chars() {
        let long = "x".repeat(MAX_HEADER_FOOTER_CHARS + 50);
        let mut cfg = BrandingConfig {
            header_text: Some(long.clone()),
            footer_text: Some(long.clone()),
            cover_subtitle: Some("y".repeat(MAX_COVER_SUBTITLE_CHARS + 10)),
            ..Default::default()
        };
        cfg.validate_and_apply_defaults();
        assert_eq!(
            cfg.header_text.as_deref().unwrap().chars().count(),
            MAX_HEADER_FOOTER_CHARS
        );
        assert_eq!(
            cfg.footer_text.as_deref().unwrap().chars().count(),
            MAX_HEADER_FOOTER_CHARS
        );
        assert_eq!(
            cfg.cover_subtitle.as_deref().unwrap().chars().count(),
            MAX_COVER_SUBTITLE_CHARS
        );
    }

    #[test]
    fn load_from_nonexistent_path_errors() {
        let path = PathBuf::from("/nonexistent/branding-does-not-exist.toml");
        let err = BrandingConfig::load_from_file(&path).unwrap_err();
        match err {
            RepoLensError::Config(ConfigError::FileRead { .. }) => {}
            other => panic!("expected FileRead error, got {other:?}"),
        }
    }

    #[test]
    fn hex_to_pdf_rgb_known_values() {
        let (r, g, b) = hex_to_pdf_rgb("#0052CC").expect("valid");
        // 0x52 = 82, 82/255 = 0.32156..., round to 0.322
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.322);
        assert_eq!(b, 0.8);
    }

    #[test]
    fn hex_to_pdf_rgb_rejects_invalid() {
        assert!(hex_to_pdf_rgb("0052CC").is_none());
        assert!(hex_to_pdf_rgb("#XYZ").is_none());
        assert!(hex_to_pdf_rgb("#1234").is_none());
    }
}
