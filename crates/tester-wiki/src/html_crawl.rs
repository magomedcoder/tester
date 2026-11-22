use crate::cache;
use crate::http;
use crate::mediawiki::IngestStats;
use crate::types::{WikiIndex, WikiPage};
use crate::version::{infer_version_from_url, version_or_default};
use crate::wikitext::{extract_content_root, html_to_text};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::Duration;
use tester_core::i18n::{self, Msg};
use tracing::debug;
use url::Url;

pub fn ingest_html_crawl(
    project_root: &Path,
    start_url: &str,
    max_pages: usize,
) -> Result<(WikiIndex, IngestStats)> {
    cache::ensure_wiki_dir(project_root)?;
    let start = Url::parse(start_url).with_context(|| format!("parse {start_url}"))?;
    let host = start.host_str().unwrap_or("").to_string();
    if host.is_empty() {
        bail!("{}", i18n::t(Msg::ErrWikiHtmlNoHost));
    }
    let path_prefix = path_prefix_of(&start);

    let client = http::wiki_client()?;
    let href_re = Regex::new(r#"(?i)<a[^>]+href=["']([^"']+)["']"#).expect("regex");
    let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").expect("regex");

    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start.clone());
    seen.insert(normalize_url(&start));

    let mut index = WikiIndex::default();
    let mut stats = IngestStats::default();
    while let Some(url) = queue.pop_front() {
        if index.pages.len() >= max_pages {
            break;
        }
        if skip_url(&url) {
            continue;
        }
        if !prefix_ok(&url, &path_prefix) {
            continue;
        }
        debug!(%url, "crawl");
        let resp = match client.get(url.clone()).send() {
            Ok(r) => r,
            Err(e) => {
                debug!(error = %e, "fetch failed");
                continue;
            }
        };
        if !resp.status().is_success() {
            continue;
        }
        let ctype = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !ctype.contains("html") && !ctype.is_empty() {
            continue;
        }
        let html = match resp.text() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let content_html = extract_content_root(&html).unwrap_or_else(|| html.clone());
        let link_html = if content_html.len() > 80 {
            &content_html
        } else {
            &html
        };
        for cap in href_re.captures_iter(link_html) {
            let href = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Ok(next) = url.join(href) {
                if next.host_str() != Some(host.as_str()) {
                    continue;
                }
                if skip_url(&next) || !prefix_ok(&next, &path_prefix) {
                    continue;
                }
                let key = normalize_url(&next);
                if seen.insert(key) {
                    queue.push_back(next);
                }
            }
        }

        let title = title_re
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| collapse_ws(m.as_str()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| url.path().to_string());
        let text = html_to_text(&content_html);
        if text.len() < 40 {
            stats.empty_skipped += 1;
            continue;
        }

        let mut links = Vec::new();
        for cap in href_re.captures_iter(&content_html) {
            let href = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Ok(next) = url.join(href)
                && next.host_str() == Some(host.as_str())
                && let Some(t) = link_title_from_url(&next)
            {
                links.push(t);
            }
        }

        let version = version_or_default(
            infer_version_from_url(url.as_str()).or_else(|| infer_version_from_url(&title)),
        );
        let id = format!("html:{}", cache::sanitize_id(url.as_str()));
        let page = WikiPage {
            id,
            title,
            version,
            url: Some(url.to_string()),
            body: text.chars().take(50_000).collect(),
            links,
        };
        let meta = cache::write_page(project_root, &page)?;
        index.pages.push(meta);
        std::thread::sleep(Duration::from_millis(60));
    }

    if index.pages.is_empty() {
        bail!(
            "{}",
            i18n::tr(Msg::ErrWikiHtmlZeroPages, &[("url", start_url)])
        );
    }
    cache::write_index(project_root, &index)?;
    Ok((index, stats))
}

fn path_prefix_of(start: &Url) -> String {
    let path = start.path();
    if let Some(idx) = path.find("/index.php") {
        return path[..idx + "/index.php".len()].to_string();
    }
    if let Some(idx) = path.find("/wiki") {
        return path[..=idx + 4].to_string();
    }
    if let Some((dir, _)) = path.rsplit_once('/') {
        if dir.is_empty() {
            "/".into()
        } else {
            dir.to_string()
        }
    } else {
        "/".into()
    }
}

fn prefix_ok(url: &Url, prefix: &str) -> bool {
    if prefix == "/" {
        return true;
    }
    url.path().starts_with(prefix) || url.as_str().contains("title=")
}

fn link_title_from_url(url: &Url) -> Option<String> {
    if let Some((_, title)) = url.query_pairs().find(|(k, _)| k == "title") {
        return Some(title.replace('_', " "));
    }
    let path = url.path();
    if let Some(rest) = path.strip_prefix("/index.php/") {
        return Some(rest.replace('_', " "));
    }
    if let Some(rest) = path.strip_prefix("/wiki/") {
        return Some(rest.replace('_', " "));
    }
    None
}

fn skip_url(url: &Url) -> bool {
    let s = url.as_str().to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();
    let auth_path = matches!(path.as_str(), "/login" | "/logout" | "/signin" | "/signout");
    s.contains("action=edit")
        || s.contains("action=history")
        || s.contains("action=login")
        || s.contains("action=submit")
        || s.contains("action=raw")
        || s.contains("diff=")
        || s.contains("oldid=")
        || path.contains("special:")
        || path.contains("special%3a")
        || auth_path
        || path.ends_with(".png")
        || path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".gif")
        || path.ends_with(".css")
        || path.ends_with(".js")
        || path.ends_with(".svg")
        || path.ends_with(".ico")
        || path.ends_with(".pdf")
        || path.ends_with(".zip")
}

fn normalize_url(url: &Url) -> String {
    let mut u = url.clone();
    u.set_fragment(None);
    u.to_string()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_edit_login_and_binaries() {
        let edit = Url::parse("http://wiki/index.php?title=X&action=edit").unwrap();
        assert!(skip_url(&edit));
        let special = Url::parse("http://wiki/index.php/Special:RecentChanges").unwrap();
        assert!(skip_url(&special));
        let png = Url::parse("http://wiki/a.png").unwrap();
        assert!(skip_url(&png));
        let ok = Url::parse("http://wiki/index.php/Login").unwrap();
        assert!(!skip_url(&ok));
    }

    #[test]
    fn content_links_preferred_over_nav() {
        let html = r#"
        <html><body>
          <nav><a href="/index.php/NavOnly">Nav</a></nav>
          <div id="mw-content-text">
            <p>Real article body with enough characters for extraction.</p>
            <a href="/index.php/ContentLink">Inside</a>
          </div>
        </body></html>"#;
        let root = extract_content_root(html).unwrap();
        assert!(root.contains("ContentLink"));
        assert!(!root.contains("NavOnly"));
        let text = html_to_text(&root);
        assert!(text.contains("Real article"));
    }
}
