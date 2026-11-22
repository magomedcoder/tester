use crate::cache;
use crate::http::{self, urlencode};
use crate::types::{WikiIndex, WikiPage};
use crate::version::{infer_version_from_title, version_or_default};
use crate::wikitext::{extract_content_root, html_to_text, wikitext_to_text};
use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;
use std::path::Path;
use std::time::Duration;
use tester_core::i18n::{self, Msg};
use tracing::{debug, warn};
use url::Url;

#[derive(Debug, Default)]
pub struct IngestStats {
    pub empty_skipped: usize,
}

#[derive(Eq, PartialEq)]
struct QueueItem {
    priority: u8,
    title: String,
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.title.cmp(&other.title))
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn probe_api(base: &str) -> Result<Option<String>> {
    let candidates = api_candidates(base)?;
    let client = http::wiki_client()?;
    for api in candidates {
        let url = format!("{api}?action=query&meta=siteinfo&siprop=general|namespaces&format=json");
        debug!(%url, "probe mediawiki");
        match client.get(&url).send() {
            Ok(resp) if resp.status().is_success() => {
                let v: Value = resp.json().unwrap_or(Value::Null);
                if v.get("query").is_some() || v.get("batchcomplete").is_some() {
                    return Ok(Some(api));
                }
            }
            _ => continue,
        }
    }
    Ok(None)
}

pub fn ingest_mediawiki(
    project_root: &Path,
    api: &str,
    start_url: &str,
    max_pages: usize,
) -> Result<(WikiIndex, IngestStats)> {
    cache::ensure_wiki_dir(project_root)?;
    let client = http::wiki_client()?;
    let mut stats = IngestStats::default();

    let mut titles = collect_titles(&client, api, start_url, max_pages)?;
    if titles.is_empty() {
        bail!("{}", i18n::tr(Msg::ErrWikiMwZeroPages, &[("api", api)]));
    }

    if titles.len() > max_pages {
        titles.truncate(max_pages);
    }

    let link_re = Regex::new(r"\[\[([^\]|#]+)").expect("regex");
    let mut index = WikiIndex::default();
    for chunk in titles.chunks(8) {
        let pages = fetch_pages_with_fallback(&client, api, chunk)?;
        for (title, text, page_url) in pages {
            let body = text.trim().to_string();
            if body.len() < 20 {
                stats.empty_skipped += 1;
                warn!(%title, "wiki page empty after fallbacks - skipped");
                continue;
            }
            let version = version_or_default(infer_version_from_title(&title));
            let id = format!("mw:{}", title.replace(' ', "_"));
            let mut links = HashSet::new();
            for cap in link_re.captures_iter(&body) {
                if let Some(m) = cap.get(1) {
                    links.insert(m.as_str().trim().to_string());
                }
            }
            let page = WikiPage {
                id,
                title: title.clone(),
                version,
                url: page_url,
                body,
                links: links.into_iter().collect(),
            };
            let meta = cache::write_page(project_root, &page)?;
            index.pages.push(meta);
        }
        std::thread::sleep(Duration::from_millis(80));
    }

    if index.pages.is_empty() {
        bail!("{}", i18n::tr(Msg::ErrWikiMwZeroPages, &[("api", api)]));
    }
    cache::write_index(project_root, &index)?;
    Ok((index, stats))
}

fn collect_titles(
    client: &reqwest::blocking::Client,
    api: &str,
    start_url: &str,
    max_pages: usize,
) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut heap = BinaryHeap::new();

    let push = |heap: &mut BinaryHeap<QueueItem>, seen: &mut HashSet<String>, title: String, pri: u8| {
        let key = title.replace('_', " ");
        if key.is_empty() || seen.contains(&key) {
            return;
        }
        seen.insert(key.clone());
        heap.push(QueueItem {
            priority: pri,
            title: key,
        });
    };

    if let Some(seed) = title_from_url(start_url) {
        push(&mut heap, &mut seen, seed, 0);
    }

    let namespaces = content_namespaces(client, api).unwrap_or_else(|_| vec![0]);
    for ns in namespaces {
        if let Ok(list) = list_all_pages_ns(client, api, ns, max_pages) {
            for t in list {
                let pri = title_priority(&t);
                push(&mut heap, &mut seen, t, pri);
            }
        }
    }

    if let Some(seed) = title_from_url(start_url)
        && seed.to_ascii_lowercase().starts_with("category:")
        && let Ok(members) = category_members(client, api, &seed, max_pages)
    {
        for t in members {
            let pri = title_priority(&t);
            push(&mut heap, &mut seen, t, pri);
        }
    }

    let mut out = Vec::new();
    while let Some(item) = heap.pop() {
        if item.title.to_ascii_lowercase().starts_with("category:")
            && let Ok(members) = category_members(client, api, &item.title, max_pages)
        {
            for t in members {
                let pri = title_priority(&t);
                push(&mut heap, &mut seen, t, pri);
            }
            continue;
        }
        out.push(item.title);
        if out.len() >= max_pages {
            break;
        }
    }
    Ok(out)
}

fn title_priority(title: &str) -> u8 {
    let l = title.to_ascii_lowercase();
    if l.contains("how")
        || l.contains("guide")
        || l.contains("tutorial")
        || l.contains("инструк")
        || l.contains("руковод")
    {
        1
    } else if l.contains("login")
        || l.contains("вход")
        || l.contains("dashboard")
        || l.contains("настрой")
    {
        2
    } else {
        5
    }
}

fn title_from_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    if let Some((_, title)) = parsed.query_pairs().find(|(k, _)| k == "title") {
        let t = title.replace('_', " ");
        if !t.is_empty() {
            return Some(t);
        }
    }
    let path = parsed.path();
    if let Some(rest) = path.strip_prefix("/index.php/") {
        let t = urlencoding_decode(rest).replace('_', " ");
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Some(rest) = path.strip_prefix("/wiki/") {
        let t = urlencoding_decode(rest).replace('_', " ");
        if !t.is_empty() {
            return Some(t);
        }
    }
    None
}

fn urlencoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(h) = h
                && let Ok(v) = u8::from_str_radix(h, 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn content_namespaces(client: &reqwest::blocking::Client, api: &str) -> Result<Vec<i64>> {
    let url = format!("{api}?action=query&meta=siteinfo&siprop=namespaces&format=json");
    let resp = client.get(&url).send().context("siteinfo namespaces")?;
    let v: Value = resp.json().context("siteinfo json")?;
    let mut out = vec![0];
    if let Some(map) = v.pointer("/query/namespaces").and_then(|x| x.as_object()) {
        for (id_s, ns) in map {
            let id: i64 = id_s.parse().unwrap_or(-1);
            if id <= 0 {
                continue;
            }
            let canonical = ns
                .get("canonical")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let content = ns.get("*").and_then(|x| x.as_str()).unwrap_or("");
            if canonical.contains("talk")
                || canonical == "special"
                || canonical == "media"
                || canonical == "user"
                || content.is_empty()
            {
                continue;
            }
            if ns.get("content").and_then(|x| x.as_bool()) == Some(true)
                || matches!(
                    canonical.as_str(),
                    "help" | "project" | "manual" | "handbook"
                )
            {
                out.push(id);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn list_all_pages_ns(
    client: &reqwest::blocking::Client,
    api: &str,
    ns: i64,
    max_pages: usize,
) -> Result<Vec<String>> {
    let mut titles = Vec::new();
    let mut cont: Option<String> = None;
    loop {
        let mut url = format!(
            "{api}?action=query&list=allpages&aplimit=50&apnamespace={ns}&format=json"
        );
        if let Some(c) = &cont {
            url.push_str(&format!("&apcontinue={}", urlencode(c)));
        }
        let resp = client.get(&url).send().context("allpages request")?;
        if !resp.status().is_success() {
            bail!(
                "{}",
                i18n::tr(
                    Msg::ErrWikiMwAllpagesHttp,
                    &[("status", &resp.status().to_string())]
                )
            );
        }
        let v: Value = resp.json().context("allpages json")?;
        if let Some(arr) = v.pointer("/query/allpages").and_then(|x| x.as_array()) {
            for item in arr {
                if let Some(t) = item.get("title").and_then(|x| x.as_str()) {
                    titles.push(t.to_string());
                    if titles.len() >= max_pages {
                        return Ok(titles);
                    }
                }
            }
        }
        cont = v
            .pointer("/continue/apcontinue")
            .and_then(|x| x.as_str())
            .map(str::to_string);
        if cont.is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    Ok(titles)
}

fn category_members(
    client: &reqwest::blocking::Client,
    api: &str,
    category: &str,
    max_pages: usize,
) -> Result<Vec<String>> {
    let mut titles = Vec::new();
    let mut cont: Option<String> = None;
    loop {
        let mut url = format!(
            "{api}?action=query&list=categorymembers&cmtitle={}&cmlimit=50&format=json",
            urlencode(category)
        );
        if let Some(c) = &cont {
            url.push_str(&format!("&cmcontinue={}", urlencode(c)));
        }
        let resp = client.get(&url).send().context("categorymembers")?;
        if !resp.status().is_success() {
            break;
        }
        let v: Value = resp.json().unwrap_or(Value::Null);
        if let Some(arr) = v
            .pointer("/query/categorymembers")
            .and_then(|x| x.as_array())
        {
            for item in arr {
                if let Some(t) = item.get("title").and_then(|x| x.as_str()) {
                    titles.push(t.to_string());
                    if titles.len() >= max_pages {
                        return Ok(titles);
                    }
                }
            }
        }
        cont = v
            .pointer("/continue/cmcontinue")
            .and_then(|x| x.as_str())
            .map(str::to_string);
        if cont.is_none() {
            break;
        }
    }
    Ok(titles)
}

fn fetch_pages_with_fallback(
    client: &reqwest::blocking::Client,
    api: &str,
    titles: &[String],
) -> Result<Vec<(String, String, Option<String>)>> {
    let mut out: Vec<(String, String, Option<String>)> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();

    let extracts = fetch_extracts(client, api, titles).unwrap_or_default();
    for (t, body, url) in extracts {
        if body.trim().len() >= 20 {
            done.insert(t.clone());
            out.push((t, body, url));
        }
    }

    let need: Vec<String> = titles
        .iter()
        .filter(|t| !done.contains(*t))
        .cloned()
        .collect();
    if !need.is_empty() {
        let revs = fetch_revisions(client, api, &need).unwrap_or_default();
        for (t, body, url) in revs {
            if body.trim().len() >= 20 {
                done.insert(t.clone());
                out.push((t, body, url));
            }
        }
    }

    for t in titles {
        if done.contains(t) {
            continue;
        }
        if let Ok(Some((tt, text, u))) = fetch_parse(client, api, t)
            && text.trim().len() >= 20
        {
            done.insert(tt.clone());
            out.push((tt, text, u));
        }
    }
    Ok(out)
}

fn fetch_extracts(
    client: &reqwest::blocking::Client,
    api: &str,
    titles: &[String],
) -> Result<Vec<(String, String, Option<String>)>> {
    let joined = titles
        .iter()
        .map(|t| urlencode(t))
        .collect::<Vec<_>>()
        .join("%7C");
    let url = format!(
        "{api}?action=query&prop=extracts|info&explaintext=1&exlimit=max&inprop=url&titles={joined}&format=json"
    );
    let resp = client.get(&url).send().context("extracts request")?;
    if !resp.status().is_success() {
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrWikiMwExtractsHttp,
                &[("status", &resp.status().to_string())]
            )
        );
    }
    let v: Value = resp.json().context("extracts json")?;
    Ok(pages_from_query(&v, |page| {
        page.get("extract")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    }))
}

fn fetch_revisions(
    client: &reqwest::blocking::Client,
    api: &str,
    titles: &[String],
) -> Result<Vec<(String, String, Option<String>)>> {
    let joined = titles
        .iter()
        .map(|t| urlencode(t))
        .collect::<Vec<_>>()
        .join("%7C");
    let url = format!(
        "{api}?action=query&prop=revisions|info&rvprop=content&rvslots=main&inprop=url&titles={joined}&format=json"
    );
    let resp = client.get(&url).send().context("revisions request")?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let v: Value = resp.json().unwrap_or(Value::Null);
    Ok(pages_from_query(&v, |page| {
        let wikitext = page
            .pointer("/revisions/0/slots/main/*")
            .and_then(|x| x.as_str())
            .or_else(|| page.pointer("/revisions/0/*").and_then(|x| x.as_str()))
            .unwrap_or("");
        wikitext_to_text(wikitext)
    }))
}

fn fetch_parse(
    client: &reqwest::blocking::Client,
    api: &str,
    title: &str,
) -> Result<Option<(String, String, Option<String>)>> {
    let url = format!(
        "{api}?action=parse&page={}&prop=text|links&format=json",
        urlencode(title)
    );
    let resp = client.get(&url).send().context("parse request")?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let v: Value = resp.json().unwrap_or(Value::Null);
    let parse = match v.get("parse") {
        Some(p) => p,
        None => return Ok(None),
    };
    let title = parse
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or(title)
        .to_string();
    let html = parse
        .pointer("/text/*")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if html.is_empty() {
        return Ok(None);
    }
    let content = extract_content_root(html).unwrap_or_else(|| html.to_string());
    let text = html_to_text(&content);
    let mut body = text;
    if let Some(arr) = parse.get("links").and_then(|x| x.as_array()) {
        let mut link_lines = Vec::new();
        for item in arr {
            if let Some(t) = item.get("*").and_then(|x| x.as_str()) {
                link_lines.push(format!("[[{t}]]"));
            }
        }
        if !link_lines.is_empty() {
            body.push_str("\n\n");
            body.push_str(&link_lines.join(" "));
        }
    }
    Ok(Some((title, body, None)))
}

fn pages_from_query(
    v: &Value,
    body_fn: impl Fn(&Value) -> String,
) -> Vec<(String, String, Option<String>)> {
    let mut out = Vec::new();
    let Some(map) = v.pointer("/query/pages").and_then(|x| x.as_object()) else {
        return out;
    };
    for (_id, page) in map {
        let title = page
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if title.is_empty() || page.get("missing").is_some() {
            continue;
        }
        let body = body_fn(page);
        let fullurl = page
            .get("fullurl")
            .and_then(|x| x.as_str())
            .map(str::to_string);
        out.push((title, body, fullurl));
    }
    out
}

fn api_candidates(base: &str) -> Result<Vec<String>> {
    let parsed = Url::parse(base).with_context(|| format!("parse url {base}"))?;
    let origin = format!(
        "{}://{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or("localhost")
    );
    let mut out = Vec::new();
    let path = parsed.path().trim_end_matches('/');
    if path.ends_with("api.php") {
        out.push(format!(
            "{}://{}{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or(""),
            path
        ));
    }
    out.push(format!("{origin}/w/api.php"));
    out.push(format!("{origin}/api.php"));
    if !path.is_empty() && path != "/" {
        let parent = path.rsplit_once('/').map(|(a, _)| a).unwrap_or("");
        if !parent.is_empty() {
            out.push(format!("{origin}{parent}/api.php"));
        }
        if path.contains("index.php") {
            out.push(format!("{origin}/api.php"));
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn title_from_query() {
        let t = title_from_url("http://x/index.php?title=Login_guide").unwrap();
        assert_eq!(t, "Login guide");
    }

    #[test]
    fn parses_extract_fixture() {
        let v = json!({
            "query": {
                "pages": {
                    "1": {
                        "title": "How to login",
                        "extract": "Open the login page and enter credentials.",
                        "fullurl": "http://wiki/How_to_login"
                    },
                    "2": {
                        "title": "Empty",
                        "extract": ""
                    }
                }
            }
        });
        let pages = pages_from_query(&v, |page| {
            page.get("extract")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        });
        assert_eq!(pages.len(), 2);
        assert!(pages[0].1.contains("login") || pages[1].1.contains("login"));
        assert!(pages.iter().any(|(_, b, _)| b.is_empty()));
    }

    #[test]
    fn parses_revision_slot_fixture() {
        let v = json!({
            "query": {
                "pages": {
                    "9": {
                        "title": "Guide",
                        "revisions": [{
                            "slots": {
                                "main": {
                                    "*": "== Steps ==\n# Open [[Dashboard]]\n"
                                }
                            }
                        }]
                    }
                }
            }
        });
        let pages = pages_from_query(&v, |page| {
            let wikitext = page
                .pointer("/revisions/0/slots/main/*")
                .and_then(|x| x.as_str())
                .or_else(|| page.pointer("/revisions/0/*").and_then(|x| x.as_str()))
                .unwrap_or("");
            wikitext_to_text(wikitext)
        });
        assert_eq!(pages.len(), 1);
        assert!(pages[0].1.contains("Dashboard") || pages[0].1.contains("Steps"));
    }

    #[test]
    fn urlencode_cyrillic_title() {
        let enc = urlencode("Вход в систему");
        assert!(enc.contains('%'));
        assert!(!enc.contains(' '));
    }

    #[test]
    fn howto_title_gets_higher_priority() {
        assert!(title_priority("How to reset password") < title_priority("Random page"));
    }
}
