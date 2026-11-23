use crate::cli::{self, run_capture};
use crate::extract::ExtractOutcome;
use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tester_core::config::DocumentsConfig;
use tester_core::i18n::{self, Msg};

pub fn ocr_image(path: &Path, cfg: &DocumentsConfig) -> Result<ExtractOutcome> {
    let bin = cli::tesseract_bin(cfg)?;
    let lang = cfg.ocr_lang.trim();
    let lang = if lang.is_empty() { "eng" } else { lang };
    let (ok, stdout, stderr) = run_capture(
        &bin,
        &[&path.to_string_lossy(), "stdout", "-l", lang],
    )?;
    if !ok {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsOcr,
                &[
                    ("path", &path.display().to_string()),
                    ("err", &stderr)
                ]
            )
        );
    }
    Ok(ExtractOutcome {
        text: stdout,
        engine: format!("{bin}:{lang}"),
        source: "ocr",
    })
}

pub fn ocr_pdf(path: &Path, cfg: &DocumentsConfig) -> Result<ExtractOutcome> {
    let ppm = cli::pdftoppm_bin(cfg)?;
    let tess = cli::tesseract_bin(cfg)?;
    let lang = cfg.ocr_lang.trim();
    let lang = if lang.is_empty() { "eng" } else { lang };
    let tmp = tempdir().map_err(|e| {
        anyhow::anyhow!(
            "{}",
            i18n::tr(Msg::ErrDocsBinFailed, &[("bin", "tempdir"), ("err", &e.to_string())])
        )
    })?;
    let prefix = tmp.path().join("page");
    let (ok, _stdout, stderr) = run_capture(
        &ppm,
        &[
            "-png",
            "-r",
            "200",
            "-f",
            "1",
            "-l",
            "5",
            &path.to_string_lossy(),
            &prefix.to_string_lossy(),
        ],
    )?;
    if !ok {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsOcr,
                &[
                    ("path", &path.display().to_string()),
                    ("err", &format!("pdftoppm: {stderr}"))
                ]
            )
        );
    }
    let mut texts = Vec::new();
    let mut pages = fs::read_dir(tmp.path())
        .map_err(|e| {
            anyhow::anyhow!(
                "{}",
                i18n::tr(Msg::ErrDocsBinFailed, &[("bin", "readdir"), ("err", &e.to_string())])
            )
        })?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    pages.sort();
    if pages.is_empty() {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsOcr,
                &[
                    ("path", &path.display().to_string()),
                    ("err", "pdftoppm produced no pages")
                ]
            )
        );
    }
    for page in pages {
        let (ok, stdout, stderr) =
            run_capture(&tess, &[&page.to_string_lossy(), "stdout", "-l", lang])?;
        if !ok {
            tracing::warn!(%stderr, page = %page.display(), "tesseract page failed");
            continue;
        }
        texts.push(stdout);
    }
    let text = texts.join("\n\n");
    if text.trim().is_empty() {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsOcr,
                &[
                    ("path", &path.display().to_string()),
                    ("err", "empty OCR output")
                ]
            )
        );
    }
    Ok(ExtractOutcome {
        text,
        engine: format!("{ppm}+{tess}:{lang}"),
        source: "ocr",
    })
}
