use crate::cache;
use crate::types::{WikiIndex, WikiPage};
use crate::version::{infer_version_from_path, version_or_default};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::Path;
use tester_core::i18n::{self, Msg};
use walkdir::WalkDir;

pub fn ingest_markdown(project_root: &Path, docs_root: &Path) -> Result<WikiIndex> {
    if !docs_root.is_dir() {
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrWikiMdNotDir,
                &[("path", &docs_root.display().to_string())]
            )
        );
    }
    cache::ensure_wiki_dir(project_root)?;
    let link_re = Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").expect("regex");
    let mut index = WikiIndex::default();

    for entry in WalkDir::new(docs_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "md" && ext != "mdx" {
            continue;
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let (fm_title, fm_version, body) = split_frontmatter(&text);
        let rel = path.strip_prefix(docs_root).unwrap_or(path);
        let title = fm_title.unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| rel.display().to_string())
        });
        let version = version_or_default(
            fm_version.or_else(|| infer_version_from_path(rel)),
        );
        let id = format!("md:{}", rel.display()).replace('\\', "/");
        let mut links = Vec::new();
        for cap in link_re.captures_iter(&body) {
            let href = cap.get(2).map(|m| m.as_str()).unwrap_or("").trim();
            if href.starts_with("http") || href.starts_with('#') || href.is_empty() {
                continue;
            }
            links.push(href.to_string());
        }
        let page = WikiPage {
            id,
            title,
            version,
            url: Some(path.display().to_string()),
            body,
            links,
        };
        let meta = cache::write_page(project_root, &page)?;
        index.pages.push(meta);
    }

    if index.pages.is_empty() {
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrWikiMdZeroPages,
                &[("path", &docs_root.display().to_string())]
            )
        );
    }
    cache::write_index(project_root, &index)?;
    Ok(index)
}

fn split_frontmatter(text: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return (None, None, text.to_string());
    }
    let rest = &trimmed[3..];
    let Some(end) = rest.find("\n---") else {
        return (None, None, text.to_string());
    };
    let fm = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    let mut title = None;
    let mut version = None;
    for line in fm.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("title:") {
            title = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(v) = line.strip_prefix("version:") {
            version = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    (title, version, body)
}
