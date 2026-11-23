use crate::cli::{self, run_capture};
use crate::detect::{DocKind, detect_kind};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use tester_core::config::DocumentsConfig;
use tester_core::i18n::{self, Msg};
use tester_core::security::path_escapes_root;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTo {
    Txt,
    Md,
    Pdf,
}

impl ConvertTo {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "txt" | "text" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Md),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub out_path: PathBuf,
    pub engine: String,
}

pub fn convert_document(
    project_root: &Path,
    path: &Path,
    to: ConvertTo,
    out: Option<&Path>,
    cfg: &DocumentsConfig,
) -> Result<ConvertResult> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    if !abs.is_file() {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsNotFound,
                &[("path", &abs.display().to_string())]
            )
        );
    }
    if path_escapes_root(project_root, &abs) {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrPathEscapes,
                &[("path", &abs.display().to_string())]
            )
        );
    }

    let out_path = match out {
        Some(p) => {
            let o = if p.is_absolute() {
                p.to_path_buf()
            } else {
                project_root.join(p)
            };
            if path_escapes_root(project_root, &o) {
                anyhow::bail!(
                    "{}",
                    i18n::tr(
                        Msg::ErrPathEscapes,
                        &[("path", &o.display().to_string())]
                    )
                );
            }
            o
        }
        None => {
            crate::cache::ensure_docs_dir(project_root)?;
            let stem = abs
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");
            crate::cache::docs_dir(project_root)
                .join("converted")
                .join(format!("{stem}.{}", to.ext()))
        }
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).with_context(|| i18n::io_ctx(Msg::IoCreate, parent))?;
    }

    let kind = detect_kind(&abs);
    match (kind, to) {
        (DocKind::Text, ConvertTo::Txt) => {
            fs::copy(&abs, &out_path).with_context(|| i18n::io_ctx(Msg::IoWrite, &out_path))?;
            Ok(ConvertResult {
                out_path,
                engine: "copy".into(),
            })
        }
        (DocKind::Text, ConvertTo::Md) if abs.extension().and_then(|e| e.to_str()) == Some("md") => {
            fs::copy(&abs, &out_path).with_context(|| i18n::io_ctx(Msg::IoWrite, &out_path))?;
            Ok(ConvertResult {
                out_path,
                engine: "copy".into(),
            })
        }
        (DocKind::Text, ConvertTo::Md) => convert_via_pandoc(&abs, &out_path, "markdown", cfg)
            .or_else(|_| {
                fs::copy(&abs, &out_path)
                    .with_context(|| i18n::io_ctx(Msg::IoWrite, &out_path))?;
                Ok(ConvertResult {
                    out_path: out_path.clone(),
                    engine: "copy".into(),
                })
            }),
        (DocKind::Image, _) => anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsUnsupportedConvert,
                &[
                    ("kind", kind.as_str()),
                    ("to", to.ext())
                ]
            )
        ),
        _ => convert_via_soffice(&abs, &out_path, to, cfg),
    }
}

fn convert_via_pandoc(
    input: &Path,
    out: &Path,
    to_fmt: &str,
    cfg: &DocumentsConfig,
) -> Result<ConvertResult> {
    let bin = cli::pandoc_bin(cfg)?;
    let (ok, _stdout, stderr) = run_capture(
        &bin,
        &[
            &input.to_string_lossy(),
            "-t",
            to_fmt,
            "-o",
            &out.to_string_lossy(),
        ],
    )?;
    if !ok || !out.is_file() {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsConvert,
                &[
                    ("path", &input.display().to_string()),
                    ("err", &stderr)
                ]
            )
        );
    }
    Ok(ConvertResult {
        out_path: out.to_path_buf(),
        engine: bin,
    })
}

fn convert_via_soffice(
    input: &Path,
    out: &Path,
    to: ConvertTo,
    cfg: &DocumentsConfig,
) -> Result<ConvertResult> {
    let bin = cli::soffice_bin(cfg)?;
    let filter = match to {
        ConvertTo::Txt => "txt:Text",
        ConvertTo::Md => {
            if cli::pandoc_bin(cfg).is_ok() {
                return convert_via_pandoc(input, out, "markdown", cfg);
            }
            "txt:Text"
        }
        ConvertTo::Pdf => "pdf:writer_pdf_Export",
    };
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
            filter,
            "--outdir",
            &tmp.path().to_string_lossy(),
            &input.to_string_lossy(),
        ],
    )?;
    if !ok {
        anyhow::bail!(
            "{}",
            i18n::tr(
                Msg::ErrDocsConvert,
                &[
                    ("path", &input.display().to_string()),
                    ("err", &stderr)
                ]
            )
        );
    }
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let produced_ext = if to == ConvertTo::Md {
        "txt"
    } else {
        to.ext()
    };
    let produced = tmp.path().join(format!("{stem}.{produced_ext}"));
    if !produced.is_file() {
        let mut found = None;
        if let Ok(rd) = fs::read_dir(tmp.path()) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some(produced_ext) {
                    found = Some(p);
                    break;
                }
            }
        }
        let Some(p) = found else {
            anyhow::bail!(
                "{}",
                i18n::tr(
                    Msg::ErrDocsConvert,
                    &[
                        ("path", &input.display().to_string()),
                        ("err", "soffice produced no output file")
                    ]
                )
            );
        };
        fs::copy(&p, out).with_context(|| i18n::io_ctx(Msg::IoWrite, out))?;
    } else {
        fs::copy(&produced, out).with_context(|| i18n::io_ctx(Msg::IoWrite, out))?;
    }

    Ok(ConvertResult {
        out_path: out.to_path_buf(),
        engine: bin,
    })
}
