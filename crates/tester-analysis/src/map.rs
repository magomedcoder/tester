use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tester_core::security::skip_dir_name;
use walkdir::WalkDir;

const MAP_CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMap {
    pub root: PathBuf,
    pub languages: BTreeMap<String, usize>,
    pub packages: Vec<PackageInfo>,
    pub monorepo: bool,
    pub entrypoints: Vec<String>,
    pub test_roots: Vec<String>,
    pub configs: Vec<String>,
    pub tree: Vec<String>,
    pub manifests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MapCache {
    version: u32,
    git_head: Option<String>,
    fingerprint: BTreeMap<String, u64>,
    map: ProjectMap,
}

fn git_head(root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn light_fingerprint(root: &Path) -> BTreeMap<String, u64> {
    let mut map = BTreeMap::new();
    for name in [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "pom.xml",
        "build.gradle",
        "composer.json",
        "pubspec.yaml",
        "CMakeLists.txt",
        "pnpm-workspace.yaml",
        "Cargo.lock",
    ] {
        let p = root.join(name);
        if let Ok(meta) = std::fs::metadata(&p) {
            let m = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            map.insert(name.into(), m);
        }
    }
    map
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(".tester").join("map.json")
}

fn load_cached_map(root: &Path) -> Result<Option<ProjectMap>> {
    let path = cache_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    let cached: MapCache = serde_json::from_str(&text)?;
    if cached.version != MAP_CACHE_VERSION {
        return Ok(None);
    }
    if cached.git_head != git_head(root) {
        return Ok(None);
    }
    if cached.fingerprint != light_fingerprint(root) {
        return Ok(None);
    }
    Ok(Some(cached.map))
}

fn save_map(root: &Path, map: &ProjectMap) -> Result<()> {
    let path = cache_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cached = MapCache {
        version: MAP_CACHE_VERSION,
        git_head: git_head(root),
        fingerprint: light_fingerprint(root),
        map: map.clone(),
    };
    std::fs::write(path, serde_json::to_string_pretty(&cached)?)?;
    Ok(())
}

pub fn build_project_map(root: &Path, max_depth: usize) -> Result<ProjectMap> {
    if let Some(cached) = load_cached_map(root)? {
        return Ok(cached);
    }
    let map = build_project_map_fresh(root, max_depth)?;
    let _ = save_map(root, &map);
    Ok(map)
}

pub fn build_project_map_fresh(root: &Path, max_depth: usize) -> Result<ProjectMap> {
    let mut languages: BTreeMap<String, usize> = BTreeMap::new();
    let mut tree = Vec::new();
    let mut manifests = Vec::new();
    let mut test_roots = Vec::new();
    let mut configs = Vec::new();
    let mut entrypoints = Vec::new();
    let mut packages = Vec::new();

    const MANIFEST_NAMES: &[&str] = &[
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "requirements.txt",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "composer.json",
        "pubspec.yaml",
        "CMakeLists.txt",
        "pnpm-workspace.yaml",
        "lerna.json",
        "nx.json",
        "turbo.json",
    ];

    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(max_depth.max(1))
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            e.depth() == 0 || !skip_dir_name(&name)
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if rel_s.is_empty() {
            continue;
        }

        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if matches!(
                name.as_ref(),
                "tests" | "test" | "__tests__" | "e2e" | "spec" | "specs"
            ) {
                test_roots.push(rel_s.clone());
            }
            if entry.depth() <= 3 {
                tree.push(format!("{rel_s}/"));
            }
            continue;
        }

        let fname = entry.file_name().to_string_lossy();
        if MANIFEST_NAMES.contains(&fname.as_ref()) {
            manifests.push(rel_s.clone());
        }
        if matches!(
            fname.as_ref(),
            "Cargo.toml"
                | "package.json"
                | "pytest.ini"
                | "jest.config.js"
                | "jest.config.ts"
                | "vitest.config.ts"
                | "playwright.config.ts"
                | ".github/workflows"
                | "CMakeLists.txt"
        ) || rel_s.contains(".github/workflows/")
            || fname.ends_with(".yml") && rel_s.contains("ci")
        {
            if configs.len() < 40 {
                configs.push(rel_s.clone());
            }
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let lang = match ext {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "go" => "go",
                "py" => "python",
                "kt" | "kts" => "kotlin",
                "java" => "java",
                "php" => "php",
                "dart" => "dart",
                "cpp" | "cc" | "cxx" | "hpp" | "h" | "c" => "c/c++",
                _ => "",
            };
            if !lang.is_empty() {
                *languages.entry(lang.into()).or_insert(0) += 1;
            }
            if matches!(
                fname.as_ref(),
                "main.rs"
                    | "lib.rs"
                    | "mod.rs"
                    | "main.go"
                    | "main.py"
                    | "index.ts"
                    | "index.js"
                    | "App.tsx"
                    | "app.ts"
            ) || rel_s.contains("/bin/")
                || rel_s.starts_with("apps/")
                || rel_s.starts_with("packages/")
            {
                if entrypoints.len() < 40 {
                    entrypoints.push(rel_s.clone());
                }
            }
            if fname.contains("_test.")
                || fname.contains(".test.")
                || fname.contains(".spec.")
                || fname.starts_with("test_")
            {
                let parent = rel
                    .parent()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                if !parent.is_empty() && !test_roots.contains(&parent) {
                    test_roots.push(parent);
                }
            }
        }

        if entry.depth() <= 2 {
            tree.push(rel_s);
        }
    }

    let cargo = root.join("Cargo.toml");
    if cargo.exists() {
        if let Ok(text) = std::fs::read_to_string(&cargo) {
            let is_workspace = text.contains("[workspace]");
            if is_workspace {
                for line in text.lines() {
                    let t = line.trim().trim_matches(['"', ',', ' ', '\\']);
                    if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
                        continue;
                    }
                    if text.contains("[workspace]")
                        && (line.contains('/')
                            || Path::new(t).join("Cargo.toml").exists()
                            || root.join(t).join("Cargo.toml").exists())
                    {
                        let p = t.trim_matches('"');
                        if root.join(p).join("Cargo.toml").exists() {
                            packages.push(PackageInfo {
                                name: Path::new(p)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(p)
                                    .into(),
                                path: p.into(),
                                kind: "cargo".into(),
                            });
                        }
                    }
                }
            } else {
                let name = text
                    .lines()
                    .find_map(|l| {
                        l.trim()
                            .strip_prefix("name")
                            .and_then(|r| r.split('=').nth(1))
                            .map(|v| v.trim().trim_matches('"').to_string())
                    })
                    .unwrap_or_else(|| "root".into());
                packages.push(PackageInfo {
                    name,
                    path: ".".into(),
                    kind: "cargo".into(),
                });
            }
        }
    }

    let pkg_json = root.join("package.json");
    if pkg_json.exists() {
        if let Ok(text) = std::fs::read_to_string(&pkg_json) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let name = v
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("root")
                    .to_string();
                packages.push(PackageInfo {
                    name,
                    path: ".".into(),
                    kind: "npm".into(),
                });
                if let Some(workspaces) = v.get("workspaces") {
                    collect_npm_workspaces(root, workspaces, &mut packages);
                }
            }
        }
    }
    if root.join("pnpm-workspace.yaml").exists() {
        if let Ok(text) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
            for line in text.lines() {
                let t = line
                    .trim()
                    .trim_start_matches("- ")
                    .trim_matches(['"', '\'']);
                if t.is_empty() || t.starts_with('#') || t.starts_with("packages:") {
                    continue;
                }
                let pattern = t.trim_end_matches("/*");
                if root.join(pattern).is_dir() {
                    packages.push(PackageInfo {
                        name: Path::new(pattern)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(pattern)
                            .into(),
                        path: pattern.into(),
                        kind: "pnpm".into(),
                    });
                }
            }
        }
    }

    let monorepo = packages.len() > 1
        || root.join("pnpm-workspace.yaml").exists()
        || manifests
            .iter()
            .filter(|m| m.ends_with("Cargo.toml"))
            .count()
            > 1;

    tree.sort();
    tree.dedup();
    tree.truncate(120);
    test_roots.sort();
    test_roots.dedup();
    test_roots.truncate(40);
    configs.sort();
    configs.dedup();
    configs.truncate(40);
    entrypoints.sort();
    entrypoints.dedup();
    entrypoints.truncate(40);
    manifests.sort();
    manifests.dedup();

    Ok(ProjectMap {
        root: root.to_path_buf(),
        languages,
        packages,
        monorepo,
        entrypoints,
        test_roots,
        configs,
        tree,
        manifests,
    })
}

fn collect_npm_workspaces(
    root: &Path,
    workspaces: &serde_json::Value,
    packages: &mut Vec<PackageInfo>,
) {
    let list: Vec<String> = match workspaces {
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        serde_json::Value::Object(o) => o
            .get("packages")
            .and_then(|p| p.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        _ => vec![],
    };
    for pat in list {
        let base = pat.trim_end_matches("/*").trim_end_matches('*');
        let dir = root.join(base);
        if dir.is_dir() {
            packages.push(PackageInfo {
                name: Path::new(base)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(base)
                    .into(),
                path: base.into(),
                kind: "npm-workspace".into(),
            });
        }
    }
}

pub fn render_map_compact(map: &ProjectMap, max_tokens: u32) -> String {
    let max_chars = (max_tokens as usize).saturating_mul(4).max(400);
    let mut out = String::new();
    out.push_str(&format!(
        "monorepo={} packages={}\n",
        map.monorepo,
        map.packages.len()
    ));
    out.push_str("languages: ");
    for (k, v) in &map.languages {
        out.push_str(&format!("{k}={v} "));
    }
    out.push('\n');
    if !map.packages.is_empty() {
        out.push_str("packages:\n");
        for p in map.packages.iter().take(20) {
            out.push_str(&format!("  - {} ({}) {}\n", p.name, p.kind, p.path));
        }
    }
    if !map.entrypoints.is_empty() {
        out.push_str("entrypoints:\n");
        for e in map.entrypoints.iter().take(15) {
            out.push_str(&format!("  - {e}\n"));
        }
    }
    if !map.test_roots.is_empty() {
        out.push_str("test_roots:\n");
        for t in map.test_roots.iter().take(15) {
            out.push_str(&format!("  - {t}\n"));
        }
    }
    if !map.manifests.is_empty() {
        out.push_str("manifests:\n");
        for m in map.manifests.iter().take(15) {
            out.push_str(&format!("  - {m}\n"));
        }
    }
    if !map.configs.is_empty() {
        out.push_str("configs:\n");
        for c in map.configs.iter().take(10) {
            out.push_str(&format!("  - {c}\n"));
        }
    }
    out.push_str("tree:\n");
    for t in map.tree.iter().take(40) {
        out.push_str(&format!("  {t}\n"));
    }
    if out.len() > max_chars {
        out.truncate(max_chars);
        out.push_str("\n... truncated");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_map_for_rust_crate() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"demo\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn x(){}\n").unwrap();
        std::fs::create_dir_all(dir.path().join("tests")).unwrap();
        let map = build_project_map_fresh(dir.path(), 4).unwrap();
        assert!(map.languages.contains_key("rust"));
        assert!(!map.packages.is_empty());
        assert!(map.test_roots.iter().any(|t| t == "tests"));
    }
}
