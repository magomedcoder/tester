use crate::types::{
    WikiAnalysis, WikiIndex, WikiPage, WikiPageMeta, WikiScenarios, WikiSource,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn wiki_dir(project_root: &Path) -> PathBuf {
    project_root.join(".tester").join("wiki")
}

pub(crate) fn ensure_wiki_dir(project_root: &Path) -> Result<PathBuf> {
    let dir = wiki_dir(project_root);
    fs::create_dir_all(dir.join("pages")).with_context(|| format!("create {}", dir.display()))?;
    Ok(dir)
}

pub(crate) fn write_source(project_root: &Path, source: &WikiSource) -> Result<()> {
    let dir = ensure_wiki_dir(project_root)?;
    write_json(&dir.join("source.json"), source)
}

pub fn load_source(project_root: &Path) -> Result<Option<WikiSource>> {
    read_json_opt(&wiki_dir(project_root).join("source.json"))
}

pub(crate) fn write_index(project_root: &Path, index: &WikiIndex) -> Result<()> {
    let dir = ensure_wiki_dir(project_root)?;
    write_json(&dir.join("index.json"), index)
}

pub fn load_index(project_root: &Path) -> Result<WikiIndex> {
    Ok(read_json_opt(&wiki_dir(project_root).join("index.json"))?.unwrap_or_default())
}

pub(crate) fn write_page(project_root: &Path, page: &WikiPage) -> Result<WikiPageMeta> {
    let dir = ensure_wiki_dir(project_root)?;
    let file = format!("{}.md", sanitize_id(&page.id));
    let path = dir.join("pages").join(&file);
    let mut body = String::new();
    body.push_str(&format!("# {}\n\n", page.title));
    if let Some(url) = &page.url {
        body.push_str(&format!("<!-- url: {url} -->\n"));
    }
    body.push_str(&format!("<!-- version: {} -->\n\n", page.version));
    body.push_str(&page.body);
    if !body.ends_with('\n') {
        body.push('\n');
    }
    fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(WikiPageMeta {
        id: page.id.clone(),
        title: page.title.clone(),
        version: page.version.clone(),
        url: page.url.clone(),
        links: page.links.clone(),
        file: format!("pages/{file}"),
    })
}

pub fn load_page_body(project_root: &Path, meta: &WikiPageMeta) -> Result<String> {
    let path = wiki_dir(project_root).join(&meta.file);
    fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}

pub(crate) fn write_analysis(project_root: &Path, analysis: &WikiAnalysis) -> Result<()> {
    let dir = ensure_wiki_dir(project_root)?;
    write_json(&dir.join("analysis.json"), analysis)
}

pub(crate) fn write_scenarios(project_root: &Path, scenarios: &WikiScenarios) -> Result<()> {
    let dir = ensure_wiki_dir(project_root)?;
    write_json(&dir.join("scenarios.json"), scenarios)
}

pub fn load_scenarios(project_root: &Path) -> Result<WikiScenarios> {
    Ok(read_json_opt(&wiki_dir(project_root).join("scenarios.json"))?.unwrap_or_default())
}

pub(crate) fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(120)
        .collect()
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn read_json_opt<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value = serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(value))
}
