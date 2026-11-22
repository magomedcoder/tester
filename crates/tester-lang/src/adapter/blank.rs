use super::{LanguageAdapter, base_profile};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};
use tester_core::model::*;

pub struct BlankAdapter;

impl LanguageAdapter for BlankAdapter {
    fn language(&self) -> Language {
        Language::Blank
    }

    fn detect(&self, _root: &Path) -> bool {
        false
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        Ok(blank_profile(root))
    }

    fn index(&self, _profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        Ok(Vec::new())
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["ts", "js"]
    }

    fn is_test_file(&self, path: &Path, _text: &str) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name.contains(".spec.")
            || name.contains(".test.")
            || path.components().any(|c| c.as_os_str() == "e2e")
    }

    fn suggest_path(&self, profile: &ProjectProfile, symbol: &Symbol, kind: TestKind) -> PathBuf {
        match kind {
            TestKind::Ui => {
                if let Some(dir) = &profile.tests_dir {
                    let rel = dir.strip_prefix(&profile.root).unwrap_or(dir);
                    return rel.join(format!("{}.spec.ts", symbol.name));
                }
                PathBuf::from("e2e").join(format!("{}.spec.ts", symbol.name))
            }
            _ => PathBuf::from("e2e").join(format!("{}.spec.ts", symbol.name)),
        }
    }

    fn path_allowed(&self, _profile: &ProjectProfile, kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(
            name,
            "package.json" | "playwright.config.ts" | "playwright.config.js"
        ) {
            return false;
        }
        match kind {
            TestKind::Ui => s.starts_with("e2e/") || name.contains(".spec."),
            _ => false,
        }
    }

    fn validate_command(&self, _profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        vec![
            "npx".into(),
            "playwright".into(),
            "test".into(),
            draft.path.display().to_string(),
        ]
    }

    fn capabilities(&self, _profile: &ProjectProfile) -> Capabilities {
        Capabilities {
            unit: false,
            integration: false,
            contract: false,
            ui: true,
        }
    }

    fn toolchain_bin(&self, _profile: &ProjectProfile) -> &'static str {
        "npx"
    }
}

pub(crate) fn playwright_ready(root: &Path) -> bool {
    if root.join("playwright.config.ts").exists() || root.join("playwright.config.js").exists() {
        return true;
    }
    let pkg = root.join("package.json");
    if !pkg.exists() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(&pkg) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = v.get(key).and_then(|x| x.as_object())
            && (obj.contains_key("@playwright/test") || obj.contains_key("playwright"))
        {
            return true;
        }
    }
    false
}

pub fn scaffold_playwright(root: &Path) -> Result<()> {
    scaffold_playwright_with(root, true)
}

pub(crate) fn scaffold_playwright_with(root: &Path, npm_install: bool) -> Result<()> {
    std::fs::create_dir_all(root)?;
    let pkg_path = root.join("package.json");
    if !pkg_path.exists() {
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        let pkg = serde_json::json!({
            "name": name,
            "private": true,
            "devDependencies": {
                "@playwright/test": "^1.48.0"
            }
        });
        std::fs::write(&pkg_path, serde_json::to_string_pretty(&pkg)?)?;
    } else {
        let text = std::fs::read_to_string(&pkg_path)?;
        let mut pkg: serde_json::Value = serde_json::from_str(&text)?;
        let dev = pkg
            .as_object_mut()
            .unwrap()
            .entry("devDependencies")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(obj) = dev.as_object_mut() {
            obj.entry("@playwright/test".to_string())
                .or_insert_with(|| serde_json::json!("^1.48.0"));
        }
        std::fs::write(&pkg_path, serde_json::to_string_pretty(&pkg)?)?;
    }

    let config_path = root.join("playwright.config.ts");
    if !config_path.exists() {
        let cfg = r#"import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL: 'http://127.0.0.1:3456',
    trace: 'retain-on-failure',
  },
});
"#;
        std::fs::write(&config_path, cfg)?;
    }

    let e2e = root.join("e2e");
    std::fs::create_dir_all(&e2e)?;
    let gitkeep = e2e.join(".gitkeep");
    if !gitkeep.exists() {
        std::fs::write(&gitkeep, b"")?;
    }

    if npm_install {
        let status = std::process::Command::new("npm")
            .arg("install")
            .current_dir(root)
            .status();
        if let Ok(st) = status
            && st.success()
        {
            let _ = std::process::Command::new("npx")
                .args(["playwright", "install", "chromium"])
                .current_dir(root)
                .status();
        }
    }
    Ok(())
}

pub(crate) fn blank_profile(root: &Path) -> ProjectProfile {
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace")
        .to_string();
    let ready = playwright_ready(root);
    let e2e = root.join("e2e");
    let mut p = base_profile(
        root,
        Language::Blank,
        &name,
        "npx playwright test",
        TestStyle::Playwright,
    );
    p.tests_dir = e2e.is_dir().then_some(e2e);
    p.ui_framework = Some("playwright".into());
    p.ui_ready = ready;
    p.capabilities = Capabilities {
        unit: false,
        integration: false,
        contract: false,
        ui: true,
    };
    p.notes = vec![i18n::t(Msg::NoteBlankWorkspace).into()];
    if !ready {
        p.notes.push(i18n::t(Msg::NoteBlankNeedUiInit).into());
    }
    p
}
