//! Integration tests for the PDF report renderer.
//!
//! Validates: structural validity (qpdf), color operators reaching the
//! page stream, optional logo image embedding (XObject), text extraction
//! (pdftotext), and a 1 000-finding fixture. External binaries
//! (qpdf, pdftotext) are skipped at runtime when absent so CI can opt-in
//! by installing them.

use std::path::Path;
use std::process::Command;

use lopdf::{Document as LopdfDocument, Object};
use repolens::cli::output::PdfReport;
use repolens::config::BrandingConfig;
use repolens::rules::results::{AuditResults, Finding, Severity};
use tempfile::TempDir;

/// Write a minimal valid 1×1 grayscale PNG to `path`.
///
/// Hand-built (RFC 2083) to avoid pulling the `image` crate into tests.
fn write_red_png(path: &Path) {
    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB88320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            table[n as usize] = c;
        }
        let mut crc = 0xFFFF_FFFFu32;
        for b in data {
            crc = table[((crc ^ u32::from(*b)) & 0xFF) as usize] ^ (crc >> 8);
        }
        crc ^ 0xFFFF_FFFF
    }
    fn adler32(data: &[u8]) -> u32 {
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &x in data {
            a = (a + u32::from(x)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut crc_data = Vec::with_capacity(4 + data.len());
        crc_data.extend_from_slice(&kind);
        crc_data.extend_from_slice(data);
        out.extend_from_slice(&kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_data).to_be_bytes());
    }

    let mut png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    // IHDR: 1×1, 8-bit greyscale (color type 0).
    let ihdr: Vec<u8> = {
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_be_bytes()); // width
        v.extend_from_slice(&1u32.to_be_bytes()); // height
        v.push(8); // bit depth
        v.push(0); // color type: greyscale
        v.push(0); // compression
        v.push(0); // filter
        v.push(0); // interlace
        v
    };
    chunk(&mut png, *b"IHDR", &ihdr);

    // IDAT: zlib(data) where data = filter byte 0 + 1 grey pixel byte 0xFF.
    // zlib wrapper: 0x78 0x01 + stored deflate block + adler32.
    let raw = [0u8, 0xFFu8];
    let stored: Vec<u8> = {
        let mut v = Vec::new();
        v.push(0x01); // BFINAL=1, BTYPE=00 stored
        let len: u16 = raw.len() as u16;
        v.extend_from_slice(&len.to_le_bytes());
        v.extend_from_slice(&(!len).to_le_bytes());
        v.extend_from_slice(&raw);
        v
    };
    let mut zlib = vec![0x78u8, 0x01];
    zlib.extend_from_slice(&stored);
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    chunk(&mut png, *b"IDAT", &zlib);

    chunk(&mut png, *b"IEND", &[]);

    std::fs::write(path, &png).expect("write png");
}

fn make_results(repo: &str, size: usize) -> AuditResults {
    let mut r = AuditResults::new(repo, "opensource");
    let categories = [
        "secrets",
        "files",
        "docs",
        "security",
        "workflows",
        "quality",
    ];
    for i in 0..size {
        let cat = categories[i % categories.len()];
        let sev = match i % 3 {
            0 => Severity::Critical,
            1 => Severity::Warning,
            _ => Severity::Info,
        };
        r.add_finding(
            Finding::new(
                format!("RULE{:04}", i),
                cat,
                sev,
                format!("Finding number {i} for {cat}"),
            )
            .with_location(format!("path/to/file{i}.rs:{}", i + 1))
            .with_description("description text")
            .with_remediation("remediation text"),
        );
    }
    r
}

fn binary_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_qpdf_check(path: &Path) -> bool {
    let output = Command::new("qpdf")
        .arg("--check")
        .arg(path)
        .output()
        .expect("qpdf invocation");
    output.status.success()
}

#[test]
fn empty_findings_produces_valid_pdf() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("empty.pdf");
    let results = AuditResults::new("empty-repo", "opensource");
    PdfReport::new(false)
        .render_to_file(&results, &out)
        .expect("render");
    assert!(out.exists());
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    if binary_available("qpdf") {
        assert!(run_qpdf_check(&out), "qpdf --check rejected the PDF");
    } else {
        eprintln!("qpdf not installed; structural check skipped");
    }
}

#[test]
fn primary_color_appears_in_page_stream() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("colors.pdf");
    let results = make_results("color-repo", 10);
    let mut branding = BrandingConfig {
        primary_color: Some("#0052CC".to_string()),
        ..Default::default()
    };
    branding.validate_and_apply_defaults();
    PdfReport::new(false)
        .with_branding(branding)
        .render_to_file(&results, &out)
        .expect("render");

    let doc = LopdfDocument::load(&out).expect("lopdf load");
    let mut found = false;
    for (_id, object) in doc.objects.iter() {
        if let Ok(stream) = object.as_stream() {
            // Decompress if compressed.
            let content = stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone());
            if let Ok(text) = std::str::from_utf8(&content) {
                if text.contains("0.000 0.322 0.800 rg")
                    || text.contains("0 0.322 0.8 rg")
                    || text.contains("0.0 0.322 0.8 rg")
                {
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(
        found,
        "expected primary_color #0052CC (rg 0.000 0.322 0.800) in PDF page stream"
    );
}

#[test]
fn logo_is_embedded_as_xobject_image() {
    let dir = TempDir::new().unwrap();
    let logo = dir.path().join("logo.png");
    write_red_png(&logo);

    let mut branding = BrandingConfig {
        logo_path: Some(logo.clone()),
        ..Default::default()
    };
    branding.validate_and_apply_defaults();
    assert!(
        branding.logo_path.is_some(),
        "logo should pass branding validation"
    );

    let out = dir.path().join("with-logo.pdf");
    let results = make_results("logo-repo", 5);
    PdfReport::new(false)
        .with_branding(branding)
        .render_to_file(&results, &out)
        .expect("render");

    let doc = LopdfDocument::load(&out).expect("lopdf load");
    let has_image_xobject = doc.objects.values().any(|object| {
        object
            .as_stream()
            .ok()
            .and_then(|s| s.dict.get(b"Subtype").ok())
            .and_then(|sub| match sub {
                Object::Name(n) => Some(n.as_slice() == b"Image"),
                _ => None,
            })
            .unwrap_or(false)
    });
    assert!(
        has_image_xobject,
        "expected an XObject image stream representing the logo"
    );
}

#[test]
fn pdftotext_contains_repository_name() {
    if !binary_available("pdftotext") {
        eprintln!("pdftotext not installed; skipping text extraction check");
        return;
    }
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("text.pdf");
    let txt = dir.path().join("text.txt");
    let results = make_results("snapshot-repo", 12);
    PdfReport::new(false)
        .render_to_file(&results, &out)
        .expect("render");

    let status = Command::new("pdftotext")
        .arg(&out)
        .arg(&txt)
        .status()
        .expect("pdftotext");
    assert!(status.success(), "pdftotext failed");
    let extracted = std::fs::read_to_string(&txt).unwrap();
    assert!(
        extracted.contains("snapshot-repo"),
        "expected repository name 'snapshot-repo' in extracted text:\n{extracted}"
    );
}

#[test]
fn toc_page_has_internal_goto_link_annotations() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("toc-links.pdf");
    let results = make_results("toc-link-repo", 6);
    PdfReport::new(false)
        .render_to_file(&results, &out)
        .expect("render");

    let doc = LopdfDocument::load(&out).expect("lopdf load");
    let pages = doc.get_pages();
    // TOC is the second page (cover is page 1).
    let toc_id = pages.get(&2).copied().expect("toc page");
    let annots = doc.get_page_annotations(toc_id);
    assert!(
        !annots.is_empty(),
        "expected at least one annotation on the TOC page"
    );

    let mut goto_links = 0usize;
    for annot in &annots {
        let subtype_is_link = annot
            .get(b"Subtype")
            .ok()
            .and_then(|o| match o {
                Object::Name(n) => Some(n.as_slice() == b"Link"),
                _ => None,
            })
            .unwrap_or(false);
        if !subtype_is_link {
            continue;
        }
        let action = annot.get(b"A").and_then(|o| o.as_dict()).ok();
        let is_goto = action
            .and_then(|d| d.get(b"S").ok())
            .and_then(|o| match o {
                Object::Name(n) => Some(n.as_slice() == b"GoTo"),
                _ => None,
            })
            .unwrap_or(false);
        if is_goto {
            goto_links += 1;
        }
    }
    assert!(
        goto_links >= 2,
        "expected at least 2 internal /GoTo link annotations on the TOC page, found {goto_links}"
    );
}

#[test]
fn one_thousand_findings_pass_qpdf_check() {
    if !binary_available("qpdf") {
        eprintln!("qpdf not installed; skipping 1k-finding qpdf check");
        return;
    }
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("large.pdf");
    let results = make_results("large-repo", 1_000);
    PdfReport::new(false)
        .render_to_file(&results, &out)
        .expect("render");
    assert!(run_qpdf_check(&out), "qpdf --check rejected the 1k PDF");
}
