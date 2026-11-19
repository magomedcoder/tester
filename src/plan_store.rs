use crate::model::TestPlan;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

fn plan_path(root: &Path) -> PathBuf {
    root.join(".tester").join("plan.json")
}

pub fn save_plan(root: &Path, plan: &TestPlan) -> Result<()> {
    let path = plan_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(plan)?)
        .with_context(|| format!("запись {}", path.display()))?;
    Ok(())
}

pub fn load_plan(root: &Path) -> Result<TestPlan> {
    let path = plan_path(root);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("чтение {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}
