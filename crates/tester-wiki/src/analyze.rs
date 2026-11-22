use crate::cache;
use crate::types::{
    GraphEdge, PageClass, ScenarioStep, TocEntry, WikiAnalysis, WikiEntity, WikiScenario,
    WikiScenarios,
};
use anyhow::Result;
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use tester_core::i18n::{self, Msg};
use tester_providers::Llm;
use tracing::warn;

pub fn analyze(project_root: &Path, llm: Option<&dyn Llm>) -> Result<(WikiAnalysis, WikiScenarios)> {
    let index = cache::load_index(project_root)?;
    if index.pages.is_empty() {
        anyhow::bail!("{}", i18n::t(Msg::ErrWikiCacheEmpty));
    }

    let mut versions = BTreeSet::new();
    let mut entities = Vec::new();
    let mut page_classes = Vec::new();
    let mut hub_pages = Vec::new();
    let mut scenarios = Vec::new();
    let mut namespaces = BTreeSet::new();
    let mut graph_edges = Vec::new();
    let mut toc = Vec::new();

    let title_by_norm: HashMap<String, String> = index
        .pages
        .iter()
        .map(|p| (normalize_link(&p.title), p.id.clone()))
        .collect();

    let howto_re = Regex::new(r"(?i)\b(how\s*to|tutorial|guide|walkthrough|инструкц|руководств)\b")
        .expect("regex");
    let feature_re =
        Regex::new(r"(?i)\b(feature|экран|страниц|dashboard|login|настройк)\b").expect("regex");
    let step_re = Regex::new(r"(?m)^\s*(?:\d+[\.\)]\s+|[-*]\s+)(.+)$").expect("regex");
    let heading_re = Regex::new(r"(?m)^#{1,3}\s+(.+)$").expect("regex");

    for meta in &index.pages {
        versions.insert(meta.version.clone());
        if let Some(ns) = namespace_of(&meta.title) {
            namespaces.insert(ns);
        }
        for link in &meta.links {
            let to = normalize_link(link);
            if let Some(to_id) = title_by_norm.get(&to) {
                graph_edges.push(GraphEdge {
                    from: meta.id.clone(),
                    to: to_id.clone(),
                });
            }
        }

        let body = cache::load_page_body(project_root, meta).unwrap_or_default();
        let blob = format!("{} {}", meta.title, body);
        let class = if howto_re.is_match(&blob) {
            "howto"
        } else if feature_re.is_match(&blob) {
            "feature"
        } else if meta.links.len() >= 5 {
            hub_pages.push(meta.id.clone());
            "nav"
        } else {
            "other"
        };
        page_classes.push(PageClass {
            page_id: meta.id.clone(),
            class: class.into(),
        });

        if class == "nav" || meta.links.len() >= 5 {
            let children: Vec<String> = meta
                .links
                .iter()
                .filter_map(|l| title_by_norm.get(&normalize_link(l)).cloned())
                .take(40)
                .collect();
            toc.push(TocEntry {
                page_id: meta.id.clone(),
                title: meta.title.clone(),
                version: meta.version.clone(),
                children,
            });
        }

        for cap in heading_re.captures_iter(&body) {
            let name = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if name.len() >= 3 && name.len() <= 80 {
                entities.push(WikiEntity {
                    name: name.to_string(),
                    kind: "heading".into(),
                    source_pages: vec![meta.id.clone()],
                });
            }
        }

        if class == "howto" || class == "feature" {
            let mut steps = Vec::new();
            for cap in step_re.captures_iter(&body) {
                let line = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                if line.len() < 3 {
                    continue;
                }
                let action = classify_step_action(line);
                steps.push(ScenarioStep {
                    action,
                    target: Some(line.to_string()),
                    value: None,
                });
                if steps.len() >= 12 {
                    break;
                }
            }
            if steps.len() >= 2 {
                let id = cache::sanitize_id(&format!("{}-{}", meta.version, meta.title))
                    .to_ascii_lowercase();
                scenarios.push(WikiScenario {
                    id,
                    version: meta.version.clone(),
                    title: meta.title.clone(),
                    preconditions: vec![],
                    steps,
                    source_pages: vec![meta.id.clone()],
                    kind: "ui".into(),
                    coverage: "uncovered".into(),
                });
            }

            entities.push(WikiEntity {
                name: meta.title.clone(),
                kind: class.into(),
                source_pages: vec![meta.id.clone()],
            });
        }
    }

    if let Some(llm) = llm {
        match llm_deep_analyze(project_root, llm, &index, &scenarios) {
            Ok(extra) => {
                if !extra.scenarios.is_empty() {
                    scenarios.extend(extra.scenarios);
                }
                for e in extra.entities {
                    entities.push(e);
                }
                for pc in extra.page_classes {
                    if let Some(existing) = page_classes.iter_mut().find(|p| p.page_id == pc.page_id)
                    {
                        existing.class = pc.class;
                    } else {
                        page_classes.push(pc);
                    }
                }
            }
            Err(e) => {
                warn!(
                    "{}",
                    i18n::tr(Msg::WarnWikiLlmPartial, &[("err", &e.to_string())])
                );
            }
        }
    } else {
        warn!("{}", i18n::t(Msg::NoteWikiHeuristicAnalyze));
    }

    let mut seen = BTreeSet::new();
    scenarios.retain(|s| seen.insert(s.id.clone()));

    let mut ent_seen = BTreeSet::new();
    entities.retain(|e| ent_seen.insert(e.name.to_ascii_lowercase()));

    let mut inbound: HashMap<String, usize> = HashMap::new();
    for e in &graph_edges {
        *inbound.entry(e.to.clone()).or_default() += 1;
    }
    let hub_set: BTreeSet<String> = hub_pages.iter().cloned().collect();
    let mut inbound_hubs: Vec<_> = inbound
        .into_iter()
        .filter(|(id, n)| *n >= 3 && !hub_set.contains(id))
        .collect();
    inbound_hubs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (id, _) in inbound_hubs.into_iter().take(20) {
        if let Some(meta) = index.pages.iter().find(|p| p.id == id) {
            let children: Vec<String> = meta
                .links
                .iter()
                .filter_map(|l| title_by_norm.get(&normalize_link(l)).cloned())
                .take(40)
                .collect();
            if !toc.iter().any(|t| t.page_id == id) {
                toc.push(TocEntry {
                    page_id: id.clone(),
                    title: meta.title.clone(),
                    version: meta.version.clone(),
                    children,
                });
            }
            hub_pages.push(id);
        }
    }

    toc.sort_by_key(|a| std::cmp::Reverse(a.children.len()));

    let analysis = WikiAnalysis {
        versions: versions.into_iter().collect(),
        entities,
        page_classes,
        hub_pages,
        namespaces: namespaces.into_iter().collect(),
        toc,
        graph_edges,
    };
    let mut wiki_scenarios = WikiScenarios { scenarios };
    crate::align::align_scenarios(project_root, &mut wiki_scenarios)?;
    cache::write_analysis(project_root, &analysis)?;
    cache::write_scenarios(project_root, &wiki_scenarios)?;
    Ok((analysis, wiki_scenarios))
}

fn namespace_of(title: &str) -> Option<String> {
    let (ns, rest) = title.split_once(':')?;
    if rest.is_empty() || ns.contains(' ') {
        return None;
    }
    Some(ns.to_string())
}

fn normalize_link(s: &str) -> String {
    s.trim()
        .trim_start_matches("./")
        .trim_end_matches(".md")
        .replace('_', " ")
        .to_ascii_lowercase()
}

fn classify_step_action(line: &str) -> String {
    let l = line.to_ascii_lowercase();
    if l.contains("open") || l.contains("открой") || l.contains("перейд") || l.contains("go to") {
        "open".into()
    } else if l.contains("click") || l.contains("нажм") || l.contains("press") {
        "click".into()
    } else if l.contains("enter")
        || l.contains("type")
        || l.contains("fill")
        || l.contains("введ")
        || l.contains("заполн")
    {
        "fill".into()
    } else if l.contains("expect")
        || l.contains("should")
        || l.contains("verify")
        || l.contains("проверь")
        || l.contains("убедись")
        || l.contains("должен")
    {
        "expect".into()
    } else {
        "step".into()
    }
}

struct LlmExtra {
    scenarios: Vec<WikiScenario>,
    entities: Vec<WikiEntity>,
    page_classes: Vec<PageClass>,
}

fn llm_deep_analyze(
    project_root: &Path,
    llm: &dyn Llm,
    index: &crate::types::WikiIndex,
    existing: &[WikiScenario],
) -> Result<LlmExtra> {
    let mut ranked: Vec<_> = index.pages.iter().collect();
    ranked.sort_by_key(|p| {
        let t = p.title.to_ascii_lowercase();
        let score = if t.contains("how")
            || t.contains("guide")
            || t.contains("tutorial")
            || t.contains("инструк")
            || t.contains("руковод")
        {
            0u8
        } else if t.contains("login") || t.contains("вход") || t.contains("feature") {
            1
        } else {
            3
        };
        (score, p.title.clone())
    });

    let mut extra = LlmExtra {
        scenarios: Vec::new(),
        entities: Vec::new(),
        page_classes: Vec::new(),
    };
    let existing_ids: BTreeSet<String> = existing.iter().map(|s| s.id.clone()).collect();
    let mut batches = 0usize;
    for chunk in ranked.chunks(4) {
        if batches >= 8 {
            break;
        }
        let mut corpus = String::new();
        let mut ids = Vec::new();
        for meta in chunk {
            let body = cache::load_page_body(project_root, meta).unwrap_or_default();
            let body = strip_cache_meta(&body);
            if body.chars().count() < 80 {
                continue;
            }
            ids.push(meta.id.clone());
            corpus.push_str(&format!(
                "## {} [id={}]\nversion: {}\n{}\n\n",
                meta.title,
                meta.id,
                meta.version,
                body.chars().take(3500).collect::<String>()
            ));
        }
        if corpus.is_empty() {
            continue;
        }
        batches += 1;
        let system = r#"You study product wiki pages to prepare Playwright UI tests.
Reply with ONE JSON object only (no markdown):
{
  "page_classes": [{"page_id":"...","class":"howto|tutorial|feature|api|changelog|nav|other"}],
  "entities": [{"name":"...","kind":"screen|role|feature|route","source_pages":["..."]}],
  "scenarios": [{
    "id":"kebab-id","version":"default","title":"...",
    "preconditions":["..."],
    "steps":[{"action":"open|click|fill|expect","target":"...","value":null}],
    "source_pages":["..."],"kind":"ui|non_ui","coverage":"uncovered"
  }]
}
Prefer UI scenarios with >=2 concrete steps. Skip pure API pages (kind non_ui)."#;
        let user = format!(
            "Already known scenario ids: {:?}\n\nPages:\n{corpus}\nReturn up to 6 scenarios for these pages.",
            existing_ids.iter().chain(extra.scenarios.iter().map(|s| &s.id)).collect::<Vec<_>>()
        );
        let raw = match llm.complete(system, &user) {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "wiki llm batch failed");
                continue;
            }
        };
        match parse_llm_bundle(&raw) {
            Ok(bundle) => {
                for mut sc in bundle.scenarios {
                    if sc.id.is_empty() {
                        continue;
                    }
                    if existing_ids.contains(&sc.id)
                        || extra.scenarios.iter().any(|s| s.id == sc.id)
                    {
                        continue;
                    }
                    if sc.source_pages.is_empty() {
                        sc.source_pages = ids.clone();
                    }
                    if sc.kind.is_empty() {
                        sc.kind = "ui".into();
                    }
                    if sc.coverage.is_empty() {
                        sc.coverage = "uncovered".into();
                    }
                    extra.scenarios.push(sc);
                }
                extra.entities.extend(bundle.entities);
                extra.page_classes.extend(bundle.page_classes);
            }
            Err(e) => warn!(error = %e, "wiki llm json parse failed"),
        }
    }
    Ok(extra)
}

#[derive(Default, serde::Deserialize)]
struct LlmBundle {
    #[serde(default)]
    scenarios: Vec<WikiScenario>,
    #[serde(default)]
    entities: Vec<WikiEntity>,
    #[serde(default)]
    page_classes: Vec<PageClass>,
}

fn parse_llm_bundle(raw: &str) -> Result<LlmBundle> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{')
        && let Some(end) = trimmed.rfind('}')
    {
        let obj = &trimmed[..=end];
        if let Ok(b) = serde_json::from_str::<LlmBundle>(obj) {
            return Ok(b);
        }
    }
    if trimmed.starts_with('[')
        && let Some(end) = trimmed.rfind(']')
    {
        let arr = &trimmed[..=end];
        let scenarios: Vec<WikiScenario> = serde_json::from_str(arr)?;
        return Ok(LlmBundle {
            scenarios,
            ..Default::default()
        });
    }
    if let Some(start) = trimmed.find('{')
        && let Some(end) = trimmed.rfind('}')
        && end > start
    {
        let obj = &trimmed[start..=end];
        if let Ok(b) = serde_json::from_str::<LlmBundle>(obj) {
            return Ok(b);
        }
    }
    anyhow::bail!("{}", i18n::t(Msg::ErrWikiLlmNoJson));
}

fn strip_cache_meta(body: &str) -> String {
    body.lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("<!--") && t.ends_with("-->"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tester_providers::MockLlm;

    #[test]
    fn parse_llm_object_and_array() {
        let obj = r#"{"scenarios":[{"id":"login-flow","version":"default","title":"Login","preconditions":[],"steps":[{"action":"open","target":"/login"}],"source_pages":["p1"],"kind":"ui"}],"entities":[],"page_classes":[{"page_id":"p1","class":"howto"}]}"#;
        let b = parse_llm_bundle(obj).unwrap();
        assert_eq!(b.scenarios.len(), 1);
        assert_eq!(b.page_classes[0].class, "howto");

        let arr = r#"[{"id":"x","version":"default","title":"T","steps":[{"action":"click","target":"btn"}],"kind":"ui"}]"#;
        let b2 = parse_llm_bundle(arr).unwrap();
        assert_eq!(b2.scenarios[0].id, "x");
    }

    #[test]
    fn llm_analyze_merges_scenarios_from_mock() {
        let home = tempfile::tempdir().unwrap();
        let docs = home.path().join("wiki");
        let project = home.path().join("proj");
        std::fs::create_dir_all(docs.join("v1.0")).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(
            docs.join("v1.0/login.md"),
            r#"# How to login to the product dashboard

This guide explains the full login flow for operators.

1. Open the login page at /login
2. Enter email and password into the form fields
3. Click Sign in and wait for redirect
4. Expect the main dashboard with user menu
"#,
        )
        .unwrap();
        crate::sync(
            &project,
            crate::SyncOpts {
                url: None,
                path: Some(docs),
                max_pages: 50,
            },
        )
        .unwrap();

        let mock = MockLlm::with_complete(
            r#"{
  "page_classes": [{"page_id": "md:v1.0/login", "class": "howto"}],
  "entities": [{"name": "Login", "kind": "screen", "source_pages": ["md:v1.0/login"]}],
  "scenarios": [{
    "id": "llm-login-happy",
    "version": "1.0",
    "title": "Operator login happy path",
    "preconditions": ["user exists"],
    "steps": [
      {"action": "open", "target": "/login"},
      {"action": "fill", "target": "email", "value": "a@b.c"},
      {"action": "click", "target": "Sign in"},
      {"action": "expect", "target": "dashboard"}
    ],
    "source_pages": [],
    "kind": "ui",
    "coverage": "uncovered"
  }]
}"#,
        );
        let (_a, scenarios) = analyze(&project, Some(&mock)).unwrap();
        assert!(
            scenarios.scenarios.iter().any(|s| s.id == "llm-login-happy"),
            "expected LLM scenario id in {:?}",
            scenarios
                .scenarios
                .iter()
                .map(|s| &s.id)
                .collect::<Vec<_>>()
        );
    }
}
