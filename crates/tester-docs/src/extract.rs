use crate::cli::{self, run_capture};
use crate::detect::DocKind;
use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tester_core::config::DocumentsConfig;
use tester_core::i18n::{self, Msg};

pub struct ExtractOutcome {
    pub text: String,
    pub engine: String,
    pub source: &'static str,
}

pub fn extract_text(path: &Path, kind: DocKind, cfg: &DocumentsConfig) -> Result<ExtractOutcome> {
    match kind {
        DocKind::Text => {
            let text = fs::read_to_string(path)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "{}",
                        i18n::tr(
                            Msg::ErrDocsBinFailed,
                            &[("bin", "read"), ("err", &e.to_string())]
                        )
                    )
                })?;
            Ok(ExtractOutcome {
                text,
                engine: "read".into(),
                source: "text",
            })
        }
        DocKind::Pdf => extract_pdf(path, cfg),
        DocKind::Office => extract_office(path, cfg),
        DocKind::Image => crate::ocr::ocr_image(path, cfg),
    }
}

fn extract_pdf(path: &Path, cfg: &DocumentsConfig) -> Result<ExtractOutcome> {
    let bin = cli::pdftotext_bin(cfg)?;
    let (ok, stdout, stderr) = run_capture(&bin, &["-layout", "-nopgbrk", &path.to_string_lossy(), "-"])?;
    if ok {
        let text = stdout.trim().to_string();
        if text.chars().filter(|c| !c.is_whitespace()).count() >= 40 {
            return Ok(ExtractOutcome {
                text,
                engine: bin,
                source: "extract",
            });
        }
    }

    match crate::ocr::ocr_pdf(path, cfg) {
        Ok(o) => Ok(o),
        Err(ocr_err) => {
            if !stdout.trim().is_empty() {
                return Ok(ExtractOutcome {
                    text: stdout,
                    engine: bin,
                    source: "extract",
                });
            }
            anyhow::bail!(
                "{}",
                i18n::tr(
                    Msg::ErrDocsExtract,
                    &[
                        ("path", &path.display().to_string()),
                        (
                            "err",
                            &format!("pdftotext: {stderr}; ocr: {ocr_err}")
                        )
                    ]
                )
            )
        }
    }
}

fn extract_office(path: &Path, cfg: &DocumentsConfig) -> Result<ExtractOutcome> {
    let bin = cli::soffice_bin(cfg)?;
    let tmp = tempdir().map_err(|e| {
        anyhow::anyhow!(
            "{}",
            i18n::tr(Msg::ErrDocsBinFailed, &[("bin", "tempdir"), ("err", &e.to_string())])
        )
    })?;
    let (ok, _stdout, stderr) = run_capture(
        &bin,
        &[
            "--headless",
            "--convert-to",
            "txt:Text",
            "--outdir",
            &tmp.path().to_string_lossy(),
            &path.to_string_lossy(),
        ],
    )?;
    if !ok {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsExtract,
                &[
                    ("path", &path.display().to_string()),
                    ("err", &stderr)
                ]
            )
        );
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let out = tmp.path().join(format!("{stem}.txt"));
    let text = fs::read_to_string(&out).unwrap_or_default();
    if text.trim().is_empty() {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsExtract,
                &[
                    ("path", &path.display().to_string()),
                    ("err", "empty soffice txt output")
                ]
            )
        );
    }
    Ok(ExtractOutcome {
        text,
        engine: bin,
        source: "extract",
    })
}
