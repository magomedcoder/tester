use crate::model::{Finding, FixHint, PatchStrategy, Severity};
use crate::walk;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

pub fn query_osv(root: &Path) -> Result<Vec<Finding>> {
    let pkgs = collect_package_names(root);
    if pkgs.is_empty() {
        return Ok(Vec::new());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let mut out = Vec::new();
    for (ecosystem, name, version) in pkgs.into_iter().take(20) {
        let body = json!({
            "package": { "name": name, "ecosystem": ecosystem },
            "version": version,
        });
        let resp = client
            .post("https://api.osv.dev/v1/query")
            .json(&body)
            .send()
            .context("OSV request")?;
        if !resp.status().is_success() {
            continue;
        }
        let v: serde_json::Value = resp.json().unwrap_or_default();
        let Some(vulns) = v.get("vulns").and_then(|x| x.as_array()) else {
            continue;
        };
        for vuln in vulns.iter().take(3) {
            let id = vuln
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("OSV")
                .to_string();
            let summary = vuln
                .get("summary")
                .and_then(|x| x.as_str())
                .unwrap_or("advisory hit")
                .to_string();
            out.push(Finding {
                id: String::new(),
                rule_id: format!("sec.deps.osv.{id}"),
                severity: Severity::High,
                category: "deps".into(),
                title: format!("{id}: {name}@{version}"),
                detail: summary,
                path: walk::rel(root, root),
                start_line: 1,
                end_line: 1,
                symbol: Some(name.clone()),
                snippet: None,
                evidence: "osv.dev".into(),
                confidence: 0.7,
                cwe: vec![],
                owasp: vec!["A06:2021".into()],
                fix: FixHint {
                    summary: format!("Upgrade {name} to a non-affected version"),
                    strategy: PatchStrategy::Deps,
                    template_id: None,
                },
            });
        }
    }
    Ok(out)
}

fn collect_package_names(root: &Path) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let cargo = root.join("Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(cargo) {
        if let Ok(v) = text.parse::<toml::Value>() {
            if let Some(deps) = v.get("dependencies").and_then(|d| d.as_table()) {
                for (name, spec) in deps {
                    let ver = match spec {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("*")
                            .to_string(),
                        _ => continue,
                    };
                    out.push(("crates.io".into(), name.clone(), ver));
                }
            }
        }
    }
    let pkg = root.join("package.json");
    if let Ok(text) = std::fs::read_to_string(pkg) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            for key in ["dependencies", "devDependencies"] {
                if let Some(deps) = v.get(key).and_then(|d| d.as_object()) {
                    for (name, ver) in deps {
                        if let Some(ver) = ver.as_str() {
                            out.push((
                                "npm".into(),
                                name.clone(),
                                ver.trim_start_matches(['^', '~']).to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
    out
}
