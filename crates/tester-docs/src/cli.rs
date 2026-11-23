use std::path::Path;
use std::process::Command;
use tester_core::config::DocumentsConfig;
use tester_core::i18n::{self, Msg};

pub fn resolve_bin(configured: &Option<String>, default_name: &str) -> String {
    configured
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default_name)
        .to_string()
}

pub fn which(bin: &str) -> Option<String> {
    if bin.contains('/') || bin.contains('\\') {
        return Path::new(bin).is_file().then(|| bin.to_string());
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{bin}.exe"));
            if exe.is_file() {
                return Some(exe.to_string_lossy().into_owned());
            }
        }
    }
    None
}

pub fn require_bin(configured: &Option<String>, default_name: &str) -> anyhow::Result<String> {
    let name = resolve_bin(configured, default_name);
    if which(&name).is_some() {
        return Ok(name);
    }
    anyhow::bail!(
        "{}",
        i18n::tr(Msg::ErrDocsMissingBin, &[("bin", &name)])
    )
}

pub fn run_capture(bin: &str, args: &[&str]) -> anyhow::Result<(bool, String, String)> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| {
            anyhow::anyhow!(
                "{}",
                i18n::tr(
                    Msg::ErrDocsBinFailed,
                    &[("bin", bin), ("err", &e.to_string())]
                )
            )
        })?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    Ok((out.status.success(), stdout, stderr))
}

pub fn pdftotext_bin(cfg: &DocumentsConfig) -> anyhow::Result<String> {
    require_bin(&cfg.pdftotext, "pdftotext")
}

pub fn pdftoppm_bin(cfg: &DocumentsConfig) -> anyhow::Result<String> {
    require_bin(&cfg.pdftoppm, "pdftoppm")
}

pub fn tesseract_bin(cfg: &DocumentsConfig) -> anyhow::Result<String> {
    require_bin(&cfg.tesseract, "tesseract")
}

pub fn soffice_bin(cfg: &DocumentsConfig) -> anyhow::Result<String> {
    if let Some(c) = cfg.soffice.as_deref().map(str::trim).filter(|s| !s.is_empty())
        && which(c).is_some()
    {
        return Ok(c.to_string());
    }
    for name in ["soffice", "libreoffice"] {
        if which(name).is_some() {
            return Ok(name.to_string());
        }
    }
    anyhow::bail!(
        "{}",
        i18n::tr(Msg::ErrDocsMissingBin, &[("bin", "soffice")])
    )
}

pub fn pandoc_bin(cfg: &DocumentsConfig) -> anyhow::Result<String> {
    require_bin(&cfg.pandoc, "pandoc")
}
