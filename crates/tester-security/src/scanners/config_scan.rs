use super::match_rule_file;
use crate::model::{Finding, ScanOptions};
use crate::rules;
use crate::walk::iter_files;
use anyhow::Result;
use tester_core::model::ProjectProfile;

pub fn scan(profile: &ProjectProfile, opts: &ScanOptions) -> Result<Vec<Finding>> {
    let root = &profile.root;
    let mut out = Vec::new();
    let mut n = 0usize;
    for path in iter_files(root, opts.max_files) {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let ext = crate::walk::ext_of(&path);
        let interesting = matches!(
            name,
            "Dockerfile"
                | "docker-compose.yml"
                | "docker-compose.yaml"
                | "compose.yml"
                | "compose.yaml"
        ) || matches!(ext.as_str(), "yml" | "yaml" | "sh" | "bash")
            || path.components().any(|c| c.as_os_str() == ".github");
        if !interesting {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for rule in rules::config_rules() {
            match_rule_file(root, &path, &text, rule, opts, &mut n, &mut out);
        }
    }
    Ok(out)
}
