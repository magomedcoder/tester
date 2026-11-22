use crate::html_crawl;
use crate::markdown;
use crate::mediawiki::{self, IngestStats};
use crate::types::{WikiIndex, WikiKind, WikiSource};
use crate::cache;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};

const DEFAULT_MAX_PAGES: usize = 500;

#[derive(Debug, Clone)]
pub struct SyncOpts {
    pub url: Option<String>,
    pub path: Option<PathBuf>,
    pub max_pages: usize,
}

impl Default for SyncOpts {
    fn default() -> Self {
        Self {
            url: None,
            path: None,
            max_pages: DEFAULT_MAX_PAGES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub source: WikiSource,
    pub index: WikiIndex,
    pub empty_skipped: usize,
}

pub fn sync(project_root: &Path, opts: SyncOpts) -> Result<(WikiSource, WikiIndex)> {
    let r = sync_ex(project_root, opts)?;
    Ok((r.source, r.index))
}

pub fn sync_ex(project_root: &Path, opts: SyncOpts) -> Result<SyncResult> {
    let max_pages = opts.max_pages.max(1);

    let (source, index, stats) = if let Some(path) = opts.path {
        let index = markdown::ingest_markdown(project_root, &path)?;
        let source = WikiSource {
            kind: WikiKind::Markdown,
            url: None,
            path: Some(path),
            synced_at: now_rfc3339(),
        };
        (source, index, IngestStats::default())
    } else if let Some(url) = opts.url {
        if let Some(api) = mediawiki::probe_api(&url)? {
            let (index, stats) =
                mediawiki::ingest_mediawiki(project_root, &api, &url, max_pages)?;
            let source = WikiSource {
                kind: WikiKind::Mediawiki,
                url: Some(url),
                path: None,
                synced_at: now_rfc3339(),
            };
            (source, index, stats)
        } else {
            let (index, stats) = html_crawl::ingest_html_crawl(project_root, &url, max_pages)?;
            let source = WikiSource {
                kind: WikiKind::HtmlCrawl,
                url: Some(url),
                path: None,
                synced_at: now_rfc3339(),
            };
            (source, index, stats)
        }
    } else if let Some(prev) = cache::load_source(project_root)? {
        return sync_ex(
            project_root,
            SyncOpts {
                url: prev.url,
                path: prev.path,
                max_pages,
            },
        );
    } else {
        bail!("{}", i18n::t(Msg::ErrWikiNeedUrlOrPath));
    };

    cache::write_source(project_root, &source)?;
    Ok(SyncResult {
        empty_skipped: stats.empty_skipped,
        source,
        index,
    })
}

fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
