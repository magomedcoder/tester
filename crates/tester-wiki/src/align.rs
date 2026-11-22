use crate::cache;
use crate::types::WikiScenarios;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub(crate) fn align_scenarios(project_root: &Path, scenarios: &mut WikiScenarios) -> Result<()> {
    let corpus = collect_ui_corpus(project_root);
    let corpus_l = corpus.to_ascii_lowercase();

    for sc in &mut scenarios.scenarios {
        if sc.kind == "non_ui" {
            sc.coverage = "unmapped".into();
            continue;
        }
        let mut hits = 0usize;
        let mut needles = Vec::new();
        needles.push(sc.title.clone());
        needles.push(sc.id.replace('_', " "));
        for step in &sc.steps {
            if let Some(t) = &step.target {
                needles.push(t.clone());
            }
        }
        for n in needles {
            let n = n.to_ascii_lowercase();
            let token = n
                .split(|c: char| !c.is_alphanumeric())
                .filter(|t| t.len() >= 4)
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            if token.is_empty() {
                continue;
            }
            if corpus_l.contains(&token) || corpus_l.contains(&n) {
                hits += 1;
            }
        }
        sc.coverage = if hits == 0 {
            "uncovered".into()
        } else if hits >= 2 {
            if looks_like_test_hit(&corpus_l, &sc.id, &sc.title) {
                "covered".into()
            } else {
                "partial".into()
            }
        } else {
            "partial".into()
        };
    }

    scenarios.scenarios.sort_by(|a, b| {
        coverage_rank(&a.coverage)
            .cmp(&coverage_rank(&b.coverage))
            .then(a.title.cmp(&b.title))
    });

    cache::write_scenarios(project_root, scenarios)?;
    Ok(())
}

fn coverage_rank(c: &str) -> u8 {
    match c {
        "uncovered" => 0,
        "partial" => 1,
        "unmapped" => 2,
        "covered" => 3,
        _ => 4,
    }
}

fn looks_like_test_hit(corpus_l: &str, id: &str, title: &str) -> bool {
    let id_l = id.to_ascii_lowercase();
    let title_l = title.to_ascii_lowercase();
    corpus_l.contains(&format!("wiki_{id_l}"))
        || corpus_l.contains(&id_l)
        || (corpus_l.contains("test(") && corpus_l.contains(&title_l))
}

fn collect_ui_corpus(project_root: &Path) -> String {
    let mut out = String::new();
    let mut dirs = vec![
        project_root.join("e2e"),
        project_root.join("tests"),
        project_root.join("test"),
        project_root.join("src"),
        project_root.join("app"),
        project_root.join("pages"),
        project_root.join("routes"),
    ];

    if let Ok(rd) = fs::read_dir(project_root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name.contains("e2e") || name.contains("playwright") || name == "ui" {
                    dirs.push(p);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir).max_depth(6).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(
                ext.as_str(),
                "ts" | "js" | "tsx" | "jsx" | "rs" | "py" | "go" | "html" | "vue" | "svelte"
            ) {
                continue;
            }
            let key = path.display().to_string();
            if !seen.insert(key) {
                continue;
            }
            if let Ok(text) = fs::read_to_string(path) {
                let take: String = text.chars().take(20_000).collect();
                out.push_str(&take);
                out.push('\n');
                if out.len() > 2_000_000 {
                    return out;
                }
            }
        }
    }
    out
}
