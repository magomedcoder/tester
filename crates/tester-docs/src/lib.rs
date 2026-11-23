mod cache;
mod cli;
mod convert;
mod detect;
mod extract;
mod ocr;

pub use convert::{ConvertTo, convert_document};
pub use detect::{DocKind, prefers_outline};

use anyhow::Result;
use cache::CacheMeta;
use detect::{DocKind as Kind, detect_kind};
use std::path::{Path, PathBuf};
use tester_core::config::DocumentsConfig;
use tester_core::i18n::{self, Msg};
use tester_core::security::path_escapes_root;

#[derive(Debug, Clone)]
pub struct ShowResult {
    pub text: String,
    pub kind: DocKind,
    pub source: String,
    pub engine: String,
    pub cache_hit: bool,
    pub path: PathBuf,
}

fn resolve_path(project_root: &Path, path: &Path) -> Result<PathBuf> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    if path_escapes_root(project_root, &abs) {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrPathEscapes,
                &[("path", &abs.display().to_string())]
            )
        );
    }
    if !abs.is_file() {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsNotFound,
                &[("path", &abs.display().to_string())]
            )
        );
    }
    Ok(abs)
}

pub fn show_document(
    project_root: &Path,
    path: &Path,
    cfg: &DocumentsConfig,
) -> Result<ShowResult> {
    let abs = resolve_path(project_root, path)?;
    let kind = detect_kind(&abs);
    let (mtime, size) = cache::file_stamp(&abs)?;

    if matches!(kind, Kind::Text) {
        let text = std::fs::read_to_string(&abs).map_err(|e| {
            anyhow::anyhow!(
                "{}",
                i18n::tr(
                    Msg::ErrDocsBinFailed,
                    &[("bin", "read"), ("err", &e.to_string())]
                )
            )
        })?;
        return Ok(ShowResult {
            text,
            kind,
            source: "text".into(),
            engine: "read".into(),
            cache_hit: false,
            path: abs,
        });
    }

    let engine_tag = format!("v1:{}", kind.as_str());
    let key = cache::cache_key(&abs, mtime, size, &engine_tag);

    if cfg.cache
        && let Some((meta, text)) = cache::read_cached(project_root, &key)?
    {
        return Ok(ShowResult {
            text,
            kind,
            source: meta.source,
            engine: meta.engine,
            cache_hit: true,
            path: abs,
        });
    }

    let outcome = extract::extract_text(&abs, kind, cfg)?;
    if cfg.cache {
        let _ = cache::ensure_docs_dir(project_root);
        let meta = CacheMeta {
            path: abs.display().to_string(),
            kind: kind.as_str().into(),
            source: outcome.source.into(),
            engine: outcome.engine.clone(),
            mtime_secs: mtime,
            size,
        };
        let _ = cache::write_cached(project_root, &key, &meta, &outcome.text);
    }

    Ok(ShowResult {
        text: outcome.text,
        kind,
        source: outcome.source.into(),
        engine: outcome.engine,
        cache_hit: false,
        path: abs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tester_core::config::DocumentsConfig;

    #[test]
    fn show_plain_text() {
        let home = tempfile::tempdir().unwrap();
        let f = home.path().join("notes.md");
        std::fs::write(&f, "# Hello\nworld\n").unwrap();
        let r = show_document(home.path(), Path::new("notes.md"), &DocumentsConfig::default())
            .unwrap();
        assert!(r.text.contains("Hello"));
        assert_eq!(r.kind, DocKind::Text);
        assert!(!r.cache_hit);
    }

    #[test]
    fn missing_pdf_tool_errors_clearly() {
        let home = tempfile::tempdir().unwrap();
        let f = home.path().join("scan.pdf");
        std::fs::write(&f, b"%PDF-1.4 fake").unwrap();
        let cfg = DocumentsConfig {
            pdftotext: Some("/nonexistent/pdftotext-xyz".into()),
            pdftoppm: Some("/nonexistent/pdftoppm-xyz".into()),
            tesseract: Some("/nonexistent/tesseract-xyz".into()),
            ..Default::default()
        };
        let err = show_document(home.path(), Path::new("scan.pdf"), &cfg).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("pdftotext") || msg.contains("PATH") || msg.contains("bin"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn convert_txt_copy() {
        let home = tempfile::tempdir().unwrap();
        let f = home.path().join("a.txt");
        std::fs::write(&f, "hi").unwrap();
        let r = convert_document(
            home.path(),
            Path::new("a.txt"),
            ConvertTo::Txt,
            None,
            &DocumentsConfig::default(),
        )
        .unwrap();
        assert!(r.out_path.is_file());
        assert_eq!(std::fs::read_to_string(&r.out_path).unwrap(), "hi");
    }
}
