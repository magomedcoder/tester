mod blank;
mod cpp;
mod dart;
mod go;
mod kotlin;
mod php;
mod python;
mod rust;
mod typescript;

pub use blank::scaffold_playwright;

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};
use tester_core::model::{
    Capabilities, Draft, Language, ProjectProfile, Symbol, TestKind, TestStyle,
};

pub trait LanguageAdapter: Send + Sync {
    fn language(&self) -> Language;
    fn detect(&self, root: &Path) -> bool;
    fn discover(&self, root: &Path) -> Result<ProjectProfile>;
    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>>;
    fn inventory_globs(&self) -> &'static [&'static str];
    fn is_test_file(&self, path: &Path, text: &str) -> bool;
    fn suggest_path(&self, profile: &ProjectProfile, symbol: &Symbol, kind: TestKind) -> PathBuf;
    fn path_allowed(&self, profile: &ProjectProfile, kind: TestKind, path: &Path) -> bool;
    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String>;
    fn capabilities(&self, profile: &ProjectProfile) -> Capabilities;
    fn toolchain_bin(&self, profile: &ProjectProfile) -> &'static str;
}

pub(crate) fn registry() -> Vec<&'static dyn LanguageAdapter> {
    vec![
        &rust::RustAdapter,
        &typescript::TypeScriptAdapter,
        &go::GoAdapter,
        &python::PythonAdapter,
        &dart::DartAdapter,
        &kotlin::KotlinAdapter,
        &php::PhpAdapter,
        &cpp::CppAdapter,
        &blank::BlankAdapter,
    ]
}

pub fn adapter_for(language: Language) -> Result<&'static dyn LanguageAdapter> {
    registry()
        .into_iter()
        .find(|a| a.language() == language)
        .or_else(|| {
            if language == Language::Flutter {
                Some(&dart::DartAdapter as &dyn LanguageAdapter)
            } else if language == Language::JavaScript {
                Some(&typescript::TypeScriptAdapter as &dyn LanguageAdapter)
            } else if language == Language::Blank {
                Some(&blank::BlankAdapter as &dyn LanguageAdapter)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{}",
                i18n::tr(Msg::ErrNoAdapterForLang, &[("lang", language.as_str())],)
            )
        })
}

pub(crate) fn detect_language(root: &Path) -> Language {
    if root.join("pubspec.yaml").exists() {
        let text = std::fs::read_to_string(root.join("pubspec.yaml")).unwrap_or_default();
        if text.contains("flutter:") || text.contains("sdk: flutter") {
            return Language::Flutter;
        }
        return Language::Dart;
    }
    if root.join("Cargo.toml").exists() {
        return Language::Rust;
    }
    if root.join("go.mod").exists() {
        return Language::Go;
    }
    if root.join("composer.json").exists() {
        return Language::Php;
    }
    if root.join("CMakeLists.txt").exists() {
        return Language::Cpp;
    }
    if root.join("build.gradle.kts").exists()
        || root.join("build.gradle").exists()
        || root.join("settings.gradle.kts").exists()
        || root.join("settings.gradle").exists()
        || root.join("pom.xml").exists()
    {
        return Language::Kotlin;
    }
    if root.join("package.json").exists() {
        if root.join("tsconfig.json").exists()
            || crate::index::iter_source_files(root, &["ts", "tsx"])
                .iter()
                .any(|p| {
                    let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    !n.contains(".test.") && !n.contains(".spec.")
                })
        {
            return Language::TypeScript;
        }
        return Language::JavaScript;
    }
    if root.join("pyproject.toml").exists()
        || root.join("requirements.txt").exists()
        || root.join("setup.cfg").exists()
    {
        return Language::Python;
    }
    Language::Unknown
}

pub fn ensure_scaffold(profile: &ProjectProfile) -> Result<()> {
    match profile.language {
        Language::Cpp => cpp::ensure_scaffold(&profile.root),
        _ => Ok(()),
    }
}

pub(crate) fn discover(root: &Path) -> Result<ProjectProfile> {
    let lang = detect_language(root);
    if lang == Language::Unknown {
        let mut profile = blank::blank_profile(root);
        profile.supported = true;
        return Ok(profile);
    }
    let adapter = adapter_for(lang)?;
    let mut profile = adapter.discover(root)?;
    profile.capabilities = adapter.capabilities(&profile);
    if crate::api_detect::detect_api_surface(root) {
        profile.capabilities.contract = true;
        profile.notes.push(i18n::t(Msg::NoteApiSurface).into());
    }
    if matches!(lang, Language::Cpp) {
        profile.capabilities.contract = false;
        profile.capabilities.ui = false;
        profile.notes.push(i18n::t(Msg::NoteCppKinds).into());
    }
    profile.supported = lang.supported();
    Ok(profile)
}

pub fn has_manifest(dir: &Path) -> bool {
    [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pubspec.yaml",
        "pyproject.toml",
        "requirements.txt",
        "composer.json",
        "CMakeLists.txt",
        "build.gradle.kts",
        "build.gradle",
        "pom.xml",
    ]
    .iter()
    .any(|m| dir.join(m).exists())
}

pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let start = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    let start = start.canonicalize().unwrap_or(start);
    let mut cur = start.clone();
    loop {
        if has_manifest(&cur) {
            return Ok(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    if let Some(child) = unique_child_project(&start)? {
        return Ok(child);
    }
    if start.is_dir() {
        return Ok(start);
    }
    bail!(
        "{}",
        i18n::tr(
            Msg::ErrProjectManifestNotFound,
            &[("path", &start.display().to_string())]
        )
    )
}

fn unique_child_project(dir: &Path) -> Result<Option<PathBuf>> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    let mut hits = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') || name.eq_ignore_ascii_case("node_modules") {
            continue;
        }
        if has_manifest(&path) {
            hits.push(path);
        }
    }
    match hits.len() {
        0 => Ok(None),
        1 => Ok(Some(hits.remove(0))),
        _ => {
            let candidates = hits
                .iter()
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "{}",
                i18n::tr(
                    Msg::ErrProjectManifestAmbiguous,
                    &[
                        ("path", &dir.display().to_string()),
                        ("candidates", &candidates),
                    ]
                )
            )
        }
    }
}

pub(crate) fn base_profile(
    root: &Path,
    language: Language,
    package_name: &str,
    test_command: &str,
    test_style: TestStyle,
) -> ProjectProfile {
    ProjectProfile {
        root: root.to_path_buf(),
        language,
        package_name: package_name.into(),
        edition: None,
        supported: true,
        test_command: test_command.into(),
        test_style,
        src_dir: None,
        tests_dir: None,
        has_existing_tests: false,
        unit_framework: None,
        ui_framework: None,
        ui_ready: false,
        capabilities: Capabilities::default(),
        notes: vec![],
        android_opt_in: false,
        package_filter: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_expected_languages() {
        let langs: Vec<_> = registry().iter().map(|a| a.language()).collect();
        assert!(langs.contains(&Language::Rust));
        assert!(langs.contains(&Language::Go));
        assert!(langs.contains(&Language::Python));
        assert!(langs.contains(&Language::Cpp));
        assert!(langs.contains(&Language::Kotlin));
        assert!(langs.contains(&Language::Php));
        assert!(langs.contains(&Language::Dart));
        assert!(langs.contains(&Language::Blank));
    }

    #[test]
    fn scaffold_playwright_creates_manifest() {
        let dir = tempfile::tempdir().unwrap();
        blank::scaffold_playwright_with(dir.path(), false).unwrap();
        assert!(dir.path().join("package.json").exists());
        assert!(dir.path().join("playwright.config.ts").exists());
        assert!(dir.path().join("e2e").is_dir());
        assert!(blank::playwright_ready(dir.path()));
    }

    #[test]
    fn discovers_rust_via_adapter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"d\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[lib]\npath=\"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn add(a:i32,b:i32)->i32{a+b}\n",
        )
        .unwrap();
        let profile = discover(dir.path()).unwrap();
        assert_eq!(profile.language, Language::Rust);
        assert!(profile.capabilities.unit);
        assert!(profile.capabilities.integration);
    }

    #[test]
    fn finds_unique_nested_project() {
        let home = tempfile::tempdir().unwrap();
        let nested = home.path().join("app");
        std::fs::create_dir_all(nested.join("src")).unwrap();
        std::fs::write(
            nested.join("Cargo.toml"),
            "[package]\nname=\"d\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[lib]\npath=\"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::write(nested.join("src/lib.rs"), "pub fn x(){}\n").unwrap();
        std::fs::create_dir_all(home.path().join("docs")).unwrap();
        let root = find_project_root(home.path()).unwrap();
        assert_eq!(root, nested.canonicalize().unwrap_or(nested));
    }

    #[test]
    fn empty_dir_becomes_blank_workspace() {
        let home = tempfile::tempdir().unwrap();
        let root = find_project_root(home.path()).unwrap();
        assert_eq!(root, home.path().canonicalize().unwrap_or(home.path().to_path_buf()));
        let profile = discover(&root).unwrap();
        assert_eq!(profile.language, Language::Blank);
        assert!(profile.supported);
        assert!(profile.capabilities.ui);
        assert!(!profile.capabilities.unit);
    }
}
