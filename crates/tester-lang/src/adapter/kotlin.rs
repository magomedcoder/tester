use super::{LanguageAdapter, base_profile};
use crate::index::iter_source_files;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};
use tester_core::model::*;

pub struct KotlinAdapter;

impl LanguageAdapter for KotlinAdapter {
    fn language(&self) -> Language {
        Language::Kotlin
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("build.gradle.kts").exists()
            || root.join("build.gradle").exists()
            || root.join("pom.xml").exists()
            || root.join("settings.gradle.kts").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let gradle = root.join("gradlew").exists()
            || root.join("build.gradle.kts").exists()
            || root.join("build.gradle").exists();
        let build_text = std::fs::read_to_string(root.join("build.gradle.kts"))
            .or_else(|_| std::fs::read_to_string(root.join("build.gradle")))
            .unwrap_or_default();
        let is_android = build_text.contains("com.android") || build_text.contains("android {");
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("kotlin_app")
            .to_string();
        let cmd = if gradle { "./gradlew test" } else { "mvn test" };
        let mut p = base_profile(root, Language::Kotlin, &name, cmd, TestStyle::JUnit);
        p.unit_framework = Some("junit5".into());
        p.src_dir = [
            root.join("src/main/kotlin"),
            root.join("src/main/java"),
            root.join("src"),
        ]
        .into_iter()
        .find(|d| d.is_dir());
        p.tests_dir = root
            .join("src/test/kotlin")
            .is_dir()
            .then(|| root.join("src/test/kotlin"));
        p.has_existing_tests = p.tests_dir.is_some();
        if is_android {
            p.notes.push(i18n::t(Msg::NoteAndroidOptIn).into());
            p.ui_framework = Some("android-instrumented".into());
        }
        p.capabilities = Capabilities {
            unit: true,
            integration: true,
            contract: build_text.contains("ktor") || build_text.contains("spring"),
            ui: is_android,
        };
        Ok(p)
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        let fn_re = Regex::new(
            r"(?m)^(?P<indent>\s*)(?:(?:public|internal|protected|private|open|override|suspend)\s+)*fun\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^)]*\))",
        )?;
        let mut out = Vec::new();
        let search = profile
            .src_dir
            .clone()
            .unwrap_or_else(|| profile.root.clone());
        for file in iter_source_files(&search, &["kt", "kts"]) {
            if file
                .components()
                .any(|c| c.as_os_str() == "test" || c.as_os_str() == "androidTest")
            {
                continue;
            }
            let text = std::fs::read_to_string(&file)?;
            let rel = file.strip_prefix(&profile.root).unwrap_or(&file);
            for caps in fn_re.captures_iter(&text) {
                let n = caps.name("name").unwrap().as_str();
                if n == "main" {
                    continue;
                }
                let indent = caps.name("indent").unwrap().as_str();

                let line_start = text[..caps.get(0).unwrap().start()]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let header = &text[line_start..caps.get(0).unwrap().end()];
                if header.contains("private fun") {
                    continue;
                }
                let sig = caps.name("sig").unwrap().as_str();
                let start = caps.get(0).unwrap().start();
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                let body = extract_brace(&text, caps.get(0).unwrap().end());
                let uses_io = body.contains("File(")
                    || body.contains("java.io")
                    || body.contains("Http")
                    || body.contains("OkHttp");
                out.push(Symbol {
                    name: n.into(),
                    kind: if indent.is_empty() {
                        "fun".into()
                    } else {
                        "method".into()
                    },
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("fun {n}{sig}"),
                    body,
                    is_public: true,
                    uses_io,
                    branch_score: 1,
                    qualname: String::new(),
                    visibility: String::new(),
                    parent: None,
                    docstring: String::new(),
                    start_byte: 0,
                });
            }
        }
        Ok(out)
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["kt", "kts"]
    }

    fn is_test_file(&self, path: &Path, text: &str) -> bool {
        path.components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("test" | "androidTest")))
            || text.contains("@Test")
            || path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with("Test.kt"))
    }

    fn suggest_path(&self, profile: &ProjectProfile, symbol: &Symbol, kind: TestKind) -> PathBuf {
        let base = if kind == TestKind::Ui && profile.android_opt_in {
            PathBuf::from("src/androidTest/kotlin")
        } else {
            PathBuf::from("src/test/kotlin")
        };
        base.join(format!("{}Test.kt", capitalize(&symbol.name)))
    }

    fn path_allowed(&self, profile: &ProjectProfile, _kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        if s.starts_with("src/androidTest/") {
            return profile.android_opt_in;
        }
        s.starts_with("src/test/")
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        let class = draft
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&draft.filter_name);
        let instrumented =
            profile.android_opt_in && draft.path.to_string_lossy().contains("androidTest");
        if profile.root.join("gradlew").exists() {
            if instrumented {
                vec![
                    "./gradlew".into(),
                    "connectedAndroidTest".into(),
                    format!("-Pandroid.testInstrumentationRunnerArguments.class=*{class}*"),
                ]
            } else {
                vec![
                    "./gradlew".into(),
                    "test".into(),
                    "--tests".into(),
                    format!("*{class}*"),
                ]
            }
        } else {
            vec!["mvn".into(), format!("-Dtest={class}"), "test".into()]
        }
    }

    fn capabilities(&self, profile: &ProjectProfile) -> Capabilities {
        let mut caps = profile.capabilities.clone();
        if profile.android_opt_in {
            caps.ui = true;
        }
        caps
    }

    fn toolchain_bin(&self, profile: &ProjectProfile) -> &'static str {
        if profile.root.join("gradlew").exists() {
            "java"
        } else {
            "mvn"
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn extract_brace(text: &str, from: usize) -> String {
    let rest = &text[from..];
    let Some(i) = rest.find('{') else {
        return rest.lines().next().unwrap_or("").to_string();
    };
    let mut depth = 0usize;
    let bytes = rest.as_bytes();
    for j in i..bytes.len() {
        match bytes[j] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return rest[i..=j].to_string();
                }
            }
            _ => {}
        }
    }
    String::new()
}
