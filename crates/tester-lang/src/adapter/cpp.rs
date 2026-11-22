use super::{LanguageAdapter, base_profile};
use crate::index::iter_source_files;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};
use tester_core::model::*;

pub struct CppAdapter;

pub fn ensure_scaffold(root: &Path) -> Result<()> {
    let cmake_path = root.join("CMakeLists.txt");
    if !cmake_path.exists() {
        std::fs::write(
            &cmake_path,
            r#"cmake_minimum_required(VERSION 3.16)
project(app CXX)
set(CMAKE_CXX_STANDARD 17)
enable_testing()
include(FetchContent)
FetchContent_Declare(googletest URL https://github.com/google/googletest/archive/refs/tags/v1.18.0.zip)
FetchContent_MakeAvailable(googletest)
add_subdirectory(tests)
"#,
        )
        .with_context(|| i18n::io_ctx(Msg::IoWrite, &cmake_path))?;
    } else {
        let mut cmake = std::fs::read_to_string(&cmake_path)?;
        let has_runner = cmake.contains("GTest")
            || cmake.contains("googletest")
            || cmake.contains("gtest")
            || cmake.contains("Catch2")
            || cmake.contains("catch2");
        if !has_runner {
            if !cmake.contains("enable_testing") {
                cmake.push_str("\nenable_testing()\n");
            }
            if !cmake.contains("FetchContent") {
                cmake.push_str(
                    "\ninclude(FetchContent)\nFetchContent_Declare(googletest URL https://github.com/google/googletest/archive/refs/tags/v1.18.0.zip)\nFetchContent_MakeAvailable(googletest)\n",
                );
            }
            if !cmake.contains("add_subdirectory(tests)") {
                cmake.push_str("\nadd_subdirectory(tests)\n");
            }
            std::fs::write(&cmake_path, cmake)
                .with_context(|| i18n::io_ctx(Msg::IoWrite, &cmake_path))?;
        }
    }

    std::fs::create_dir_all(root.join("tests"))?;
    let tests_cmake = root.join("tests/CMakeLists.txt");
    if !tests_cmake.exists() {
        std::fs::write(
            &tests_cmake,
            "enable_testing()\n# тесты добавляет tester при генерации\n",
        )?;
    }
    Ok(())
}

impl LanguageAdapter for CppAdapter {
    fn language(&self) -> Language {
        Language::Cpp
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("CMakeLists.txt").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let cmake = std::fs::read_to_string(root.join("CMakeLists.txt")).unwrap_or_default();
        let name = Regex::new(r"project\s*\(\s*([A-Za-z0-9_]+)")
            .ok()
            .and_then(|re| {
                re.captures(&cmake)
                    .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            })
            .unwrap_or_else(|| {
                root.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cpp_app")
                    .to_string()
            });
        let has_gtest =
            cmake.contains("GTest") || cmake.contains("googletest") || cmake.contains("gtest");
        let has_catch = cmake.contains("Catch2") || cmake.contains("catch2");
        let mut p = base_profile(root, Language::Cpp, &name, "ctest", TestStyle::GTest);
        if has_gtest {
            p.unit_framework = Some("gtest".into());
        } else if has_catch {
            p.unit_framework = Some("catch2".into());
        } else {
            p.unit_framework = Some("gtest".into());
            p.notes
                .push("GTest-каркас будет создан автоматически при plan/generate/run".into());
        }
        p.src_dir = ["src", "include", "."]
            .iter()
            .map(|d| root.join(d))
            .find(|d| d.is_dir());
        p.tests_dir = ["tests", "test"]
            .iter()
            .map(|d| root.join(d))
            .find(|d| d.is_dir());
        p.has_existing_tests = p.tests_dir.is_some();
        p.capabilities = Capabilities {
            unit: true,
            integration: true,
            contract: false,
            ui: false,
        };
        Ok(p)
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        let fn_re = Regex::new(
            r"(?m)^(?P<ret>[A-Za-z_][\w:<>,\s\*&]*)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^;{]*\))\s*(?:const)?\s*\{",
        )?;
        let mut out = Vec::new();
        for file in iter_source_files(&profile.root, &["cpp", "cc", "cxx", "h", "hpp"]) {
            if file.components().any(|c| {
                matches!(
                    c.as_os_str().to_str(),
                    Some("tests" | "test" | "build" | "cmake-build-debug")
                )
            }) {
                continue;
            }
            let text = std::fs::read_to_string(&file)?;
            let rel = file.strip_prefix(&profile.root).unwrap_or(&file);
            for caps in fn_re.captures_iter(&text) {
                let n = caps.name("name").unwrap().as_str();
                if matches!(n, "if" | "for" | "while" | "switch" | "return" | "main") {
                    continue;
                }
                let sig = caps.name("sig").unwrap().as_str();
                let ret = caps.name("ret").unwrap().as_str().trim();
                if ret.contains("TEST") || ret.contains("#") {
                    continue;
                }
                let start = caps.get(0).unwrap().start();
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                let body = extract_brace(&text, caps.get(0).unwrap().end());
                let uses_io = body.contains("fstream")
                    || body.contains("cstdio")
                    || body.contains("socket")
                    || body.contains("iostream");
                out.push(Symbol {
                    name: n.into(),
                    kind: "fn".into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("{ret} {n}{sig}"),
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
        &["cpp", "cc", "cxx"]
    }

    fn is_test_file(&self, path: &Path, text: &str) -> bool {
        path.components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("tests" | "test")))
            || text.contains("TEST(")
            || text.contains("TEST_F(")
            || text.contains("CATCH_TEST_CASE")
            || text.contains("TEST_CASE(")
    }

    fn suggest_path(&self, _profile: &ProjectProfile, symbol: &Symbol, _kind: TestKind) -> PathBuf {
        PathBuf::from("tests").join(format!("test_{}.cpp", symbol.name))
    }

    fn path_allowed(&self, _profile: &ProjectProfile, _kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        (s.starts_with("tests/") || s.starts_with("test/")) && !s.contains("CMakeLists.txt")
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        let build = profile.root.join("build");
        let filter = draft.filter_name.clone();
        if build.join("CTestTestfile.cmake").exists() || build.is_dir() {
            vec![
                "ctest".into(),
                "--test-dir".into(),
                build.display().to_string(),
                "-R".into(),
                filter,
                "--output-on-failure".into(),
            ]
        } else {
            vec![
                "cmake".into(),
                "-S".into(),
                ".".into(),
                "-B".into(),
                "build".into(),
                "-DBUILD_TESTING=ON".into(),
            ]
        }
    }

    fn capabilities(&self, profile: &ProjectProfile) -> Capabilities {
        profile.capabilities.clone()
    }

    fn toolchain_bin(&self, _profile: &ProjectProfile) -> &'static str {
        "cmake"
    }
}

fn extract_brace(text: &str, from: usize) -> String {
    let rest = &text[from..];
    let Some(i) = rest.find('{') else {
        return String::new();
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
