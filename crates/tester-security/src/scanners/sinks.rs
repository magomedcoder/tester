use super::match_rule_file;
use crate::model::{Finding, ScanOptions};
use crate::rules;
use crate::walk::iter_files;
use anyhow::Result;
use tester_core::model::ProjectProfile;

pub fn scan(profile: &ProjectProfile, opts: &ScanOptions) -> Result<Vec<Finding>> {
    let root = &profile.root;
    let mut out = Vec::new();
    let mut n = out.len();
    for path in iter_files(root, opts.max_files) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for rule in rules::sink_rules() {
            match_rule_file(root, &path, &text, rule, opts, &mut n, &mut out);
        }
    }
    Ok(out)
}
