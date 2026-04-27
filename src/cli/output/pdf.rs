//! PDF report renderer.
//!
//! Generates a fully-featured audit report (cover, table of contents,
//! summary, per-category findings, annexes) using `printpdf`. The output is a
//! pure-Rust PDF — no external binary, no system font dependency — so the
//! CLI works inside CI containers and inside the published Docker image.
//!
//! Branding is optional and degrades gracefully: invalid inputs become
//! defaults via [`crate::config::BrandingConfig::validate_and_apply_defaults`].

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use chrono::Utc;
use printpdf::{
    BuiltinFont, Color, Image, ImageTransform, ImageXObject, IndirectFontRef, Mm, PdfDocument,
    PdfDocumentReference, PdfLayerIndex, PdfLayerReference, PdfPageIndex, Rect, Rgb,
};
use sha2::{Digest, Sha256};
use tracing::warn;

use crate::config::branding::{hex_to_pdf_rgb, BrandingConfig, DEFAULT_FONT_FAMILY};
use crate::error::{ActionError, RepoLensError};
use crate::rules::results::{AuditResults, Finding, Severity};

/// A4 page width in millimeters.
const PAGE_WIDTH_MM: f32 = 210.0;
/// A4 page height in millimeters.
const PAGE_HEIGHT_MM: f32 = 297.0;
/// Left/right margin in millimeters.
const MARGIN_LEFT_MM: f32 = 18.0;
const MARGIN_RIGHT_MM: f32 = 18.0;
/// Top margin in millimeters.
const MARGIN_TOP_MM: f32 = 22.0;
/// Bottom margin in millimeters.
const MARGIN_BOTTOM_MM: f32 = 22.0;

/// Critical-severity color: GitHub red.
const COLOR_CRITICAL: &str = "#D73A49";
/// Warning-severity color: orange.
const COLOR_WARNING: &str = "#FB8500";
/// Info-severity color: GitHub blue.
const COLOR_INFO: &str = "#0366D6";

/// Threshold above which Info findings are aggregated rather than detailed.
const LARGE_REPORT_THRESHOLD: usize = 5_000;
/// Maximum findings rendered fully inside a single category before overflow
/// goes to the annex.
const MAX_CATEGORY_BODY_FINDINGS: usize = 200;
/// Truncation length for cells > 250 chars (leaves room for "…").
const CELL_TRUNCATE_LEN: usize = 247;
/// Wrap target for cells > 80 chars.
const CELL_WRAP_AT: usize = 80;

/// PDF report renderer.
///
/// Construct via [`PdfReport::new`] and optionally chain
/// [`PdfReport::with_branding`] to apply a `[branding]` TOML.
pub struct PdfReport {
    detailed: bool,
    branding: BrandingConfig,
}

impl PdfReport {
    /// Create a new renderer using default branding values.
    pub fn new(detailed: bool) -> Self {
        Self {
            detailed,
            branding: BrandingConfig::defaults(),
        }
    }

    /// Apply a user-supplied [`BrandingConfig`].
    ///
    /// The configuration is validated immediately so any rendering call
    /// works against the corrected values.
    pub fn with_branding(mut self, mut branding: BrandingConfig) -> Self {
        branding.validate_and_apply_defaults();
        self.branding = branding;
        self
    }

    /// Render the audit results to a PDF file at `output`.
    pub fn render_to_file(
        &self,
        results: &AuditResults,
        output: &Path,
    ) -> Result<(), RepoLensError> {
        let bytes = self.render_to_bytes(results)?;
        let mut file = BufWriter::new(File::create(output).map_err(|e| {
            RepoLensError::Action(ActionError::FileWrite {
                path: output.display().to_string(),
                source: e,
            })
        })?);
        file.write_all(&bytes).map_err(|e| {
            RepoLensError::Action(ActionError::FileWrite {
                path: output.display().to_string(),
                source: e,
            })
        })?;
        file.flush().map_err(|e| {
            RepoLensError::Action(ActionError::FileWrite {
                path: output.display().to_string(),
                source: e,
            })
        })?;
        Ok(())
    }

    /// Render the audit to an in-memory PDF byte buffer (used by tests and
    /// benchmarks; the CLI uses [`PdfReport::render_to_file`]).
    pub fn render_to_bytes(&self, results: &AuditResults) -> Result<Vec<u8>, RepoLensError> {
        let title = format!("RepoLens Audit — {}", results.repository_name);
        let (doc, page0, layer0) =
            PdfDocument::new(&title, Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Cover");
        let doc = doc
            .with_author("RepoLens")
            .with_creator(format!("RepoLens v{}", env!("CARGO_PKG_VERSION")))
            .with_producer("RepoLens PDF renderer (printpdf)")
            .with_subject(format!("Audit report for {}", results.repository_name));

        let fonts = Fonts::load(
            &doc,
            self.branding
                .font_family
                .as_deref()
                .unwrap_or(DEFAULT_FONT_FAMILY),
        )?;
        let palette = Palette::from_branding(&self.branding);
        let config_hash = compute_config_hash(results, &self.branding);

        let layout = Layout::new();

        // Plan all pages up-front so we can build a TOC with real page numbers.
        let categories = collect_categories(results);
        let plan = ReportPlan::build(results, &categories, self.detailed);

        let mut pages = Pages::new(doc, page0, layer0);

        // Cover page (page 0, already created).
        self.draw_cover(&pages, &fonts, &palette, &layout, results, &config_hash);

        // Table of contents.
        let toc_index = pages.add_page("Table of contents");
        self.draw_toc(&pages.layer(toc_index), &fonts, &palette, &layout, &plan);

        // Per-section pages — record actual page indices for the TOC.
        let mut toc_pages: Vec<(String, usize)> = Vec::new();

        let summary_idx = pages.add_page("Summary");
        toc_pages.push(("Summary".to_string(), human_page(summary_idx)));
        self.draw_summary(
            &pages.layer(summary_idx),
            &fonts,
            &palette,
            &layout,
            results,
        );
        self.draw_header_footer(&pages.layer(summary_idx), &fonts, &palette, &layout);

        for category in &categories {
            let entry_idx = pages.add_page(&format!("Category: {}", category));
            toc_pages.push((format!("Category: {category}"), human_page(entry_idx)));
            self.draw_category_section(
                &mut pages, entry_idx, &fonts, &palette, &layout, results, category, &plan,
            );
        }

        let annex_idx = pages.add_page("Annexes");
        toc_pages.push(("Annexes".to_string(), human_page(annex_idx)));
        self.draw_annexes(
            &mut pages,
            annex_idx,
            &fonts,
            &palette,
            &layout,
            results,
            &config_hash,
            &plan,
        );

        // Re-render the TOC now that we know the real page numbers.
        self.draw_toc_entries(
            &pages.layer(toc_index),
            &fonts,
            &palette,
            &layout,
            &toc_pages,
        );

        let bytes = pages.into_bytes()?;
        Ok(bytes)
    }

    fn draw_cover(
        &self,
        pages: &Pages,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
        results: &AuditResults,
        config_hash: &str,
    ) {
        let layer = pages.layer(pages.first());

        // Optional logo: centered, 60pt from top, max 200×80pt.
        if let Some(ref logo) = self.branding.logo_path {
            match decode_logo(logo) {
                Ok(image) => {
                    let (w_pt, h_pt) = clamp_logo_dimensions(
                        image.image.width.0 as f32,
                        image.image.height.0 as f32,
                    );
                    let center_x_mm = PAGE_WIDTH_MM / 2.0 - pt_to_mm(w_pt) / 2.0;
                    let top_y_pt = mm_to_pt(PAGE_HEIGHT_MM) - 60.0 - h_pt;
                    let scale_x = w_pt / image.image.width.0 as f32;
                    let scale_y = h_pt / image.image.height.0 as f32;
                    image.add_to_layer(
                        layer.clone(),
                        ImageTransform {
                            translate_x: Some(Mm(center_x_mm)),
                            translate_y: Some(Mm(pt_to_mm(top_y_pt))),
                            scale_x: Some(scale_x),
                            scale_y: Some(scale_y),
                            ..Default::default()
                        },
                    );
                }
                Err(e) => {
                    warn!("branding: failed to decode logo: {e}");
                }
            }
        }

        // Repository title in primary_color, 24pt bold, centered.
        layer.set_fill_color(palette.primary.clone());
        let title = results.repository_name.clone();
        let title_y_mm = PAGE_HEIGHT_MM - 110.0;
        layer.use_text(
            title.clone(),
            24.0,
            Mm(layout.center_x(estimate_text_width_mm(&title, 24.0))),
            Mm(title_y_mm),
            &fonts.bold,
        );

        // Subtitle in secondary_color, 14pt.
        layer.set_fill_color(palette.secondary.clone());
        let subtitle = self
            .branding
            .cover_subtitle
            .clone()
            .unwrap_or_else(|| format!("Preset: {}", results.preset));
        layer.use_text(
            subtitle.clone(),
            14.0,
            Mm(layout.center_x(estimate_text_width_mm(&subtitle, 14.0))),
            Mm(title_y_mm - 12.0),
            &fonts.regular,
        );

        // Metadata block.
        layer.set_fill_color(palette.text.clone());
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let lines = [
            format!("Generated: {date}"),
            format!("RepoLens version: {}", env!("CARGO_PKG_VERSION")),
            format!("Config hash: {}", &config_hash[..16]),
        ];
        for (i, line) in lines.iter().enumerate() {
            layer.use_text(
                line.as_str(),
                10.0,
                Mm(layout.left_margin),
                Mm(title_y_mm - 36.0 - (i as f32) * 6.0),
                &fonts.regular,
            );
        }

        // Bottom band: primary_color, full width, 18mm tall.
        layer.set_fill_color(palette.primary.clone());
        layer.add_rect(
            Rect::new(Mm(0.0), Mm(0.0), Mm(PAGE_WIDTH_MM), Mm(18.0))
                .with_mode(printpdf::path::PaintMode::Fill),
        );
        // White text on bottom band.
        layer.set_fill_color(Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
        let band_text = format!("RepoLens v{}  •  {}", env!("CARGO_PKG_VERSION"), date);
        layer.use_text(
            band_text,
            10.0,
            Mm(layout.left_margin),
            Mm(7.0),
            &fonts.bold,
        );
    }

    fn draw_toc(
        &self,
        layer: &PdfLayerReference,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
        _plan: &ReportPlan,
    ) {
        layer.set_fill_color(palette.primary.clone());
        layer.use_text(
            "Table of Contents",
            18.0,
            Mm(layout.left_margin),
            Mm(layout.content_top_y),
            &fonts.bold,
        );
        // Header/footer added by the per-page decorator after entries are filled.
        self.draw_header_footer(layer, fonts, palette, layout);
    }

    fn draw_toc_entries(
        &self,
        layer: &PdfLayerReference,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
        entries: &[(String, usize)],
    ) {
        layer.set_fill_color(palette.text.clone());
        let start_y = layout.content_top_y - 14.0;
        for (i, (label, page)) in entries.iter().enumerate() {
            let y = start_y - (i as f32) * 7.0;
            layer.use_text(
                format!("{}. {}", i + 1, label),
                11.0,
                Mm(layout.left_margin),
                Mm(y),
                &fonts.regular,
            );
            layer.use_text(
                page.to_string(),
                11.0,
                Mm(PAGE_WIDTH_MM - layout.right_margin - 10.0),
                Mm(y),
                &fonts.regular,
            );
        }
    }

    fn draw_summary(
        &self,
        layer: &PdfLayerReference,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
        results: &AuditResults,
    ) {
        layer.set_fill_color(palette.primary.clone());
        layer.use_text(
            "Summary",
            18.0,
            Mm(layout.left_margin),
            Mm(layout.content_top_y),
            &fonts.bold,
        );

        let counts = [
            (
                "Critical",
                results.count_by_severity(Severity::Critical),
                palette.critical.clone(),
            ),
            (
                "Warning",
                results.count_by_severity(Severity::Warning),
                palette.warning.clone(),
            ),
            (
                "Info",
                results.count_by_severity(Severity::Info),
                palette.info.clone(),
            ),
        ];

        let mut y = layout.content_top_y - 16.0;
        for (label, count, color) in &counts {
            layer.set_fill_color(color.clone());
            layer.use_text(
                format!("{label}: {count}"),
                14.0,
                Mm(layout.left_margin),
                Mm(y),
                &fonts.bold,
            );
            y -= 9.0;
        }

        // Top-10 critical findings.
        layer.set_fill_color(palette.text.clone());
        layer.use_text(
            "Top 10 Critical Findings",
            13.0,
            Mm(layout.left_margin),
            Mm(y - 8.0),
            &fonts.bold,
        );
        y -= 16.0;

        let critical: Vec<&Finding> = results
            .findings_by_severity(Severity::Critical)
            .take(10)
            .collect();

        if critical.is_empty() {
            layer.use_text(
                "No critical findings.",
                10.0,
                Mm(layout.left_margin),
                Mm(y),
                &fonts.regular,
            );
        } else {
            for (i, finding) in critical.iter().enumerate() {
                let line = truncate_cell(&format!(
                    "{}. {} — {}",
                    i + 1,
                    finding.rule_id,
                    finding.message
                ));
                layer.use_text(line, 10.0, Mm(layout.left_margin), Mm(y), &fonts.regular);
                y -= 6.0;
                if y < layout.content_bottom_y {
                    break;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_category_section(
        &self,
        pages: &mut Pages,
        first_page: PageRef,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
        results: &AuditResults,
        category: &str,
        plan: &ReportPlan,
    ) {
        let layer = pages.layer(first_page);

        let findings: Vec<&Finding> = results.findings_by_category(category).collect();
        let count = findings.len();

        layer.set_fill_color(palette.primary.clone());
        layer.use_text(
            format!("Category: {category}"),
            18.0,
            Mm(layout.left_margin),
            Mm(layout.content_top_y),
            &fonts.bold,
        );

        layer.set_fill_color(palette.text.clone());
        layer.use_text(
            format!("{count} finding(s)"),
            11.0,
            Mm(layout.left_margin),
            Mm(layout.content_top_y - 7.0),
            &fonts.regular,
        );
        self.draw_header_footer(&layer, fonts, palette, layout);

        // For very large reports, aggregate Info findings.
        let aggregate_info = plan.aggregate_info;

        let body_findings: Vec<&Finding> = findings
            .iter()
            .copied()
            .filter(|f| !(aggregate_info && f.severity == Severity::Info))
            .take(MAX_CATEGORY_BODY_FINDINGS)
            .collect();

        let column_x = [
            layout.left_margin,
            layout.left_margin + 60.0,
            layout.left_margin + 95.0,
            layout.left_margin + 120.0,
        ];
        let header_y = layout.content_top_y - 16.0;
        layer.set_fill_color(palette.secondary.clone());
        for (i, label) in ["Path", "Line", "Severity", "Message"].iter().enumerate() {
            layer.use_text(*label, 10.0, Mm(column_x[i]), Mm(header_y), &fonts.bold);
        }

        let mut current_layer = layer;
        let mut y = header_y - 7.0;
        for finding in body_findings {
            if y < layout.content_bottom_y + 10.0 {
                let next = pages.add_page(&format!("{category} (cont.)"));
                current_layer = pages.layer(next);
                self.draw_header_footer(&current_layer, fonts, palette, layout);
                y = layout.content_top_y;
            }
            let (path, line) = split_location(finding.location.as_deref());
            let severity = severity_label(finding.severity);
            let severity_color = severity_color(finding.severity, palette);
            let message = wrap_long(&truncate_cell(&finding.message));

            current_layer.set_fill_color(palette.text.clone());
            current_layer.use_text(
                truncate_cell(&path),
                9.0,
                Mm(column_x[0]),
                Mm(y),
                &fonts.regular,
            );
            current_layer.use_text(line, 9.0, Mm(column_x[1]), Mm(y), &fonts.regular);
            current_layer.set_fill_color(severity_color);
            current_layer.use_text(severity, 9.0, Mm(column_x[2]), Mm(y), &fonts.bold);
            current_layer.set_fill_color(palette.text.clone());
            for (i, msg_line) in message.lines().enumerate() {
                current_layer.use_text(
                    msg_line,
                    9.0,
                    Mm(column_x[3]),
                    Mm(y - (i as f32) * 4.5),
                    &fonts.regular,
                );
            }
            let consumed = (message.lines().count().max(1) as f32) * 4.5;
            y -= consumed.max(5.5);
        }

        if aggregate_info {
            let info_count = findings
                .iter()
                .filter(|f| f.severity == Severity::Info)
                .count();
            if info_count > 0 {
                if y < layout.content_bottom_y + 10.0 {
                    let next = pages.add_page(&format!("{category} (cont.)"));
                    current_layer = pages.layer(next);
                    self.draw_header_footer(&current_layer, fonts, palette, layout);
                    y = layout.content_top_y;
                }
                current_layer.set_fill_color(palette.info.clone());
                current_layer.use_text(
                    format!("(+ {info_count} info finding(s) aggregated; see Annexes)"),
                    9.0,
                    Mm(layout.left_margin),
                    Mm(y - 4.0),
                    &fonts.regular,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_annexes(
        &self,
        pages: &mut Pages,
        first_page: PageRef,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
        results: &AuditResults,
        config_hash: &str,
        plan: &ReportPlan,
    ) {
        let layer = pages.layer(first_page);
        layer.set_fill_color(palette.primary.clone());
        layer.use_text(
            "Annexes",
            18.0,
            Mm(layout.left_margin),
            Mm(layout.content_top_y),
            &fonts.bold,
        );
        self.draw_header_footer(&layer, fonts, palette, layout);

        layer.set_fill_color(palette.text.clone());
        let mut y = layout.content_top_y - 12.0;

        let info = [
            format!("Repository: {}", results.repository_name),
            format!("Preset: {}", results.preset),
            format!("Total findings: {}", results.findings().len()),
            format!("Generated: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")),
            format!("RepoLens version: {}", env!("CARGO_PKG_VERSION")),
            format!("Config hash: {config_hash}"),
        ];
        for line in info {
            layer.use_text(line, 10.0, Mm(layout.left_margin), Mm(y), &fonts.regular);
            y -= 6.0;
        }

        // Branding TOML dump.
        y -= 4.0;
        layer.set_fill_color(palette.secondary.clone());
        layer.use_text(
            "Applied branding configuration",
            12.0,
            Mm(layout.left_margin),
            Mm(y),
            &fonts.bold,
        );
        layer.set_fill_color(palette.text.clone());
        y -= 6.0;
        let branding_dump = render_branding_dump(&self.branding);
        for line in branding_dump.lines() {
            if y < layout.content_bottom_y + 6.0 {
                let next = pages.add_page("Annexes (cont.)");
                let l2 = pages.layer(next);
                self.draw_header_footer(&l2, fonts, palette, layout);
                l2.set_fill_color(palette.text.clone());
                y = layout.content_top_y;
                l2.use_text(line, 9.0, Mm(layout.left_margin), Mm(y), &fonts.regular);
            } else {
                layer.use_text(line, 9.0, Mm(layout.left_margin), Mm(y), &fonts.regular);
            }
            y -= 4.5;
        }

        // Aggregated info findings, if any.
        if plan.aggregate_info {
            let next = pages.add_page("Annexes — Info findings");
            let l = pages.layer(next);
            self.draw_header_footer(&l, fonts, palette, layout);
            l.set_fill_color(palette.info.clone());
            l.use_text(
                "Info findings (aggregated)",
                14.0,
                Mm(layout.left_margin),
                Mm(layout.content_top_y),
                &fonts.bold,
            );
            l.set_fill_color(palette.text.clone());
            let mut yy = layout.content_top_y - 10.0;
            let info_by_cat = aggregate_info_findings(results);
            for (cat, count) in info_by_cat {
                if yy < layout.content_bottom_y {
                    break;
                }
                l.use_text(
                    format!("{cat}: {count}"),
                    10.0,
                    Mm(layout.left_margin),
                    Mm(yy),
                    &fonts.regular,
                );
                yy -= 5.5;
            }
        }

        // Rule list.
        let next = pages.add_page("Annexes — Rules");
        let l = pages.layer(next);
        self.draw_header_footer(&l, fonts, palette, layout);
        l.set_fill_color(palette.secondary.clone());
        l.use_text(
            "Rules referenced in this report",
            14.0,
            Mm(layout.left_margin),
            Mm(layout.content_top_y),
            &fonts.bold,
        );
        l.set_fill_color(palette.text.clone());
        let mut yy = layout.content_top_y - 10.0;
        let mut rules: Vec<String> = results
            .findings()
            .iter()
            .map(|f| format!("{} ({})", f.rule_id, f.category))
            .collect();
        rules.sort();
        rules.dedup();
        let mut current = l;
        for rule in rules {
            if yy < layout.content_bottom_y {
                let next = pages.add_page("Annexes — Rules (cont.)");
                current = pages.layer(next);
                self.draw_header_footer(&current, fonts, palette, layout);
                current.set_fill_color(palette.text.clone());
                yy = layout.content_top_y;
            }
            current.use_text(rule, 9.0, Mm(layout.left_margin), Mm(yy), &fonts.regular);
            yy -= 4.5;
        }
    }

    fn draw_header_footer(
        &self,
        layer: &PdfLayerReference,
        fonts: &Fonts,
        palette: &Palette,
        layout: &Layout,
    ) {
        if let Some(ref header) = self.branding.header_text {
            if !header.trim().is_empty() {
                layer.set_fill_color(palette.secondary.clone());
                layer.use_text(
                    header.as_str(),
                    9.0,
                    Mm(layout.left_margin),
                    Mm(PAGE_HEIGHT_MM - 10.0),
                    &fonts.regular,
                );
            }
        }
        if let Some(ref footer) = self.branding.footer_text {
            if !footer.trim().is_empty() {
                layer.set_fill_color(palette.secondary.clone());
                layer.use_text(
                    footer.as_str(),
                    9.0,
                    Mm(layout.left_margin),
                    Mm(8.0),
                    &fonts.regular,
                );
            }
        }
    }
}

struct Layout {
    left_margin: f32,
    right_margin: f32,
    content_top_y: f32,
    content_bottom_y: f32,
}

impl Layout {
    fn new() -> Self {
        Self {
            left_margin: MARGIN_LEFT_MM,
            right_margin: MARGIN_RIGHT_MM,
            content_top_y: PAGE_HEIGHT_MM - MARGIN_TOP_MM,
            content_bottom_y: MARGIN_BOTTOM_MM,
        }
    }

    fn center_x(&self, content_width_mm: f32) -> f32 {
        (PAGE_WIDTH_MM - content_width_mm) / 2.0
    }
}

struct Fonts {
    regular: IndirectFontRef,
    bold: IndirectFontRef,
}

impl Fonts {
    fn load(doc: &PdfDocumentReference, family: &str) -> Result<Self, RepoLensError> {
        let (regular_kind, bold_kind) = match resolve_builtin_family(family) {
            Some(pair) => pair,
            None => {
                warn!(
                    "branding: font family {family:?} is not a built-in PDF font, falling back to {DEFAULT_FONT_FAMILY}"
                );
                (BuiltinFont::Helvetica, BuiltinFont::HelveticaBold)
            }
        };
        let regular = doc.add_builtin_font(regular_kind).map_err(font_err)?;
        let bold = doc.add_builtin_font(bold_kind).map_err(font_err)?;
        Ok(Self { regular, bold })
    }
}

fn font_err(e: printpdf::Error) -> RepoLensError {
    RepoLensError::Action(ActionError::ExecutionFailed {
        message: format!("printpdf font: {e}"),
    })
}

fn resolve_builtin_family(family: &str) -> Option<(BuiltinFont, BuiltinFont)> {
    match family.to_ascii_lowercase().as_str() {
        "helvetica" | "" => Some((BuiltinFont::Helvetica, BuiltinFont::HelveticaBold)),
        "times" | "times-roman" | "times new roman" => {
            Some((BuiltinFont::TimesRoman, BuiltinFont::TimesBold))
        }
        "courier" => Some((BuiltinFont::Courier, BuiltinFont::CourierBold)),
        _ => None,
    }
}

#[derive(Clone)]
struct Palette {
    primary: Color,
    secondary: Color,
    text: Color,
    critical: Color,
    warning: Color,
    info: Color,
}

impl Palette {
    fn from_branding(branding: &BrandingConfig) -> Self {
        Self {
            primary: hex_color(branding.primary_color.as_deref().unwrap_or("#0052CC")),
            secondary: hex_color(branding.secondary_color.as_deref().unwrap_or("#172B4D")),
            text: hex_color(branding.text_color.as_deref().unwrap_or("#000000")),
            critical: hex_color(COLOR_CRITICAL),
            warning: hex_color(COLOR_WARNING),
            info: hex_color(COLOR_INFO),
        }
    }
}

fn hex_color(hex: &str) -> Color {
    let (r, g, b) = hex_to_pdf_rgb(hex).unwrap_or((0.0, 0.0, 0.0));
    Color::Rgb(Rgb::new(r, g, b, None))
}

fn severity_color(sev: Severity, palette: &Palette) -> Color {
    match sev {
        Severity::Critical => palette.critical.clone(),
        Severity::Warning => palette.warning.clone(),
        Severity::Info => palette.info.clone(),
    }
}

fn severity_label(sev: Severity) -> &'static str {
    match sev {
        Severity::Critical => "CRITICAL",
        Severity::Warning => "WARNING",
        Severity::Info => "INFO",
    }
}

/// Sequential page handle returned by [`Pages::add_page`].
///
/// Tracks the order in which pages were added so we can compute the human
/// page number for the table of contents (the underlying
/// [`printpdf::PdfPageIndex`] hides its inner offset).
#[derive(Clone, Copy)]
struct PageRef(usize);

struct Pages {
    doc: PdfDocumentReference,
    pages: Vec<(PdfPageIndex, PdfLayerIndex)>,
}

impl Pages {
    fn new(
        doc: PdfDocumentReference,
        first_page: PdfPageIndex,
        first_layer: PdfLayerIndex,
    ) -> Self {
        Self {
            doc,
            pages: vec![(first_page, first_layer)],
        }
    }

    fn first(&self) -> PageRef {
        PageRef(0)
    }

    fn add_page(&mut self, layer_name: &str) -> PageRef {
        let (page, layer) = self
            .doc
            .add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), layer_name);
        self.pages.push((page, layer));
        PageRef(self.pages.len() - 1)
    }

    fn layer(&self, page: PageRef) -> PdfLayerReference {
        let (p, l) = self.pages[page.0];
        self.doc.get_page(p).get_layer(l)
    }

    fn into_bytes(self) -> Result<Vec<u8>, RepoLensError> {
        self.doc.save_to_bytes().map_err(|e| {
            RepoLensError::Action(ActionError::ExecutionFailed {
                message: format!("printpdf save: {e}"),
            })
        })
    }
}

fn human_page(page: PageRef) -> usize {
    page.0 + 1
}

struct ReportPlan {
    aggregate_info: bool,
}

impl ReportPlan {
    fn build(results: &AuditResults, _categories: &[String], _detailed: bool) -> Self {
        Self {
            aggregate_info: results.findings().len() > LARGE_REPORT_THRESHOLD,
        }
    }
}

fn collect_categories(results: &AuditResults) -> Vec<String> {
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for f in results.findings() {
        seen.insert(f.category.clone(), ());
    }
    seen.into_keys().collect()
}

fn aggregate_info_findings(results: &AuditResults) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in results.findings() {
        if f.severity == Severity::Info {
            *counts.entry(f.category.clone()).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
}

fn split_location(loc: Option<&str>) -> (String, String) {
    match loc {
        None => ("-".to_string(), "-".to_string()),
        Some(s) => match s.rsplit_once(':') {
            Some((path, line)) if line.chars().all(|c| c.is_ascii_digit()) => {
                (path.to_string(), line.to_string())
            }
            _ => (s.to_string(), "-".to_string()),
        },
    }
}

fn wrap_long(text: &str) -> String {
    if text.chars().count() <= CELL_WRAP_AT {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut current = 0usize;
    for ch in text.chars() {
        out.push(ch);
        current += 1;
        if current >= CELL_WRAP_AT && matches!(ch, '/' | '_' | '-' | '.') {
            out.push('\n');
            current = 0;
        }
    }
    out
}

fn truncate_cell(text: &str) -> String {
    if text.chars().count() <= CELL_TRUNCATE_LEN + 3 {
        return text.to_string();
    }
    let mut out: String = text.chars().take(CELL_TRUNCATE_LEN).collect();
    out.push('…');
    out
}

fn render_branding_dump(b: &BrandingConfig) -> String {
    fn opt(label: &str, v: Option<&str>) -> String {
        format!("{} = {}", label, v.unwrap_or("(default)"))
    }
    let mut out = String::new();
    out.push_str("[branding]\n");
    out.push_str(&format!(
        "logo_path        = {}\n",
        b.logo_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".to_string())
    ));
    out.push_str(&opt("primary_color   ", b.primary_color.as_deref()));
    out.push('\n');
    out.push_str(&opt("secondary_color ", b.secondary_color.as_deref()));
    out.push('\n');
    out.push_str(&opt("text_color      ", b.text_color.as_deref()));
    out.push('\n');
    out.push_str(&opt("font_family     ", b.font_family.as_deref()));
    out.push('\n');
    out.push_str(&opt("header_text     ", b.header_text.as_deref()));
    out.push('\n');
    out.push_str(&opt("footer_text     ", b.footer_text.as_deref()));
    out.push('\n');
    out.push_str(&opt("cover_subtitle  ", b.cover_subtitle.as_deref()));
    out.push('\n');
    out
}

fn compute_config_hash(results: &AuditResults, branding: &BrandingConfig) -> String {
    let mut hasher = Sha256::new();
    hasher.update(results.preset.as_bytes());
    hasher.update(b"|");
    hasher.update(results.repository_name.as_bytes());
    hasher.update(b"|");
    hasher.update(branding.primary_color.as_deref().unwrap_or("").as_bytes());
    hasher.update(branding.secondary_color.as_deref().unwrap_or("").as_bytes());
    hasher.update(branding.text_color.as_deref().unwrap_or("").as_bytes());
    hasher.update(branding.font_family.as_deref().unwrap_or("").as_bytes());
    format!("{:x}", hasher.finalize())
}

fn estimate_text_width_mm(text: &str, font_size_pt: f32) -> f32 {
    // Conservative average width of a Helvetica glyph at the given size.
    // Used purely for approximate centering of headings on the cover.
    let avg_glyph_pt = font_size_pt * 0.5;
    pt_to_mm(text.chars().count() as f32 * avg_glyph_pt)
}

fn pt_to_mm(pt: f32) -> f32 {
    pt / 2.834_645_7
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 2.834_645_7
}

fn clamp_logo_dimensions(width_pt: f32, height_pt: f32) -> (f32, f32) {
    const MAX_W: f32 = 200.0;
    const MAX_H: f32 = 80.0;
    let scale = (MAX_W / width_pt.max(1.0))
        .min(MAX_H / height_pt.max(1.0))
        .min(1.0);
    (width_pt * scale, height_pt * scale)
}

fn decode_logo(path: &Path) -> Result<Image, String> {
    let raw = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let img = image::load_from_memory(&raw).map_err(|e| format!("decode: {e}"))?;
    Ok(Image::from(ImageXObject::from_dynamic_image(&img)))
}
