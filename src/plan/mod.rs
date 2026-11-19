use crate::adapter;
use crate::config::Config;
use crate::coverage::{self, Uncovered};
use crate::debug::StackFrame;
use crate::index::index_symbols;
use crate::inventory::{collect_inventory, covers_symbol};
use crate::model::{
    Gap, Language, PlanItem, PlannedCase, ProjectProfile, Symbol, TestKind, TestPlan,
};
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default, Clone)]
pub struct PlanOptions {
    pub diff_base: Option<String>,
    pub bug_text: Option<String>,
    pub prefer_append: bool,
    pub max_items: Option<usize>,
    pub coverage_path: Option<PathBuf>,
    pub stack_frames: Vec<StackFrame>,
    pub reproduce_only: bool,
}

pub fn build_plan(profile: &ProjectProfile, config: &Config, kind: TestKind) -> Result<TestPlan> {
    build_plan_with(profile, config, kind, &PlanOptions::default())
}

pub fn build_plan_with(
    profile: &ProjectProfile,
    config: &Config,
    kind: TestKind,
    opts: &PlanOptions,
) -> Result<TestPlan> {
    if !profile.supported {
        return Ok(TestPlan {
            items: vec![],
            warnings: profile.notes.clone(),
        });
    }

    let adapter = adapter::adapter_for(profile.language)?;
    let mut symbols = index_symbols(profile)?;
    if let Some(pkg) = &profile.package_filter {
        symbols.retain(|s| {
            s.file.to_string_lossy().contains(pkg.as_str())
                || profile.package_name == *pkg
                || s.name.contains(pkg.as_str())
        });
    }
    let inventory = collect_inventory(profile)?;
    let mut gaps = Vec::new();
    let mut warnings = profile.notes.clone();

    let changed_files = opts
        .diff_base
        .as_ref()
        .map(|base| git_changed_files(&profile.root, base))
        .transpose()?
        .unwrap_or_default();
    let caller_files = if !changed_files.is_empty() {
        expand_callers(&symbols, &changed_files)
    } else {
        HashSet::new()
    };
    let bug_tokens = opts
        .bug_text
        .as_ref()
        .map(|t| extract_bug_tokens(t))
        .unwrap_or_default();
    let uncovered: Uncovered = opts
        .coverage_path
        .as_ref()
        .map(|p| coverage::load_coverage(p))
        .or_else(|| {
            coverage::discover_coverage_file(&profile.root).map(|p| coverage::load_coverage(&p))
        })
        .transpose()?
        .unwrap_or_default();

    for symbol in symbols {
        if is_trivial(&symbol) {
            continue;
        }
        let covered = covers_symbol(&inventory, &symbol.name);
        if covered && !opts.reproduce_only && opts.stack_frames.is_empty() {
            continue;
        }
        if !changed_files.is_empty() {
            let file = symbol.file.to_string_lossy();
            let in_diff = changed_files
                .iter()
                .any(|c| c.ends_with(file.as_ref()) || file.ends_with(c));
            let in_caller = caller_files
                .iter()
                .any(|c| c.ends_with(file.as_ref()) || file.ends_with(c.as_str()));
            if !in_diff && !in_caller {
                continue;
            }
        }
        let stack_boost = stack_match_score(&symbol, &opts.stack_frames);
        if !bug_tokens.is_empty() || !opts.stack_frames.is_empty() {
            let hit = bug_tokens.iter().any(|t| fuzzy_hit(&symbol, t))
                || stack_boost > 0
                || opts.reproduce_only;
            if !opts.stack_frames.is_empty() && stack_boost == 0 && !opts.reproduce_only {
                let soft = bug_tokens.iter().any(|t| fuzzy_hit(&symbol, t));
                if !soft {
                    continue;
                }
            } else if bug_tokens.is_empty() && opts.stack_frames.is_empty() {

            } else if !hit && !bug_tokens.is_empty() && stack_boost == 0 {
                continue;
            }
        }
        if covered && opts.reproduce_only && stack_boost == 0 {
            continue;
        }

        let inferred = infer_kind(profile, &symbol);
        let effective = if kind != TestKind::All {
            if kind.expand().contains(&inferred)
                || kind == TestKind::Unit && inferred == TestKind::Unit
            {
                kind.expand()
                    .into_iter()
                    .find(|k| *k == inferred)
                    .unwrap_or(inferred)
            } else {
                inferred
            }
        } else {
            inferred
        };

        if !capability_allows(profile, effective) {
            continue;
        }

        if effective == TestKind::Contract && !has_api_signal(profile, &symbol) {
            continue;
        }

        let cov_lines = coverage::symbol_uncovered(
            &uncovered,
            &symbol.file,
            symbol.start_line,
            symbol.end_line,
        );
        let mut prio = priority(&symbol, effective) + stack_boost * 50 + cov_lines as u32 * 3;
        if opts.reproduce_only && stack_boost > 0 {
            prio += 500;
        }
        gaps.push(Gap {
            priority: prio,
            reason: if cov_lines > 0 {
                format!(
                    "{}; непокрыто строк: {cov_lines}",
                    reason(&symbol, effective)
                )
            } else {
                reason(&symbol, effective)
            },
            kind: effective,
            symbol,
        });
    }

    let kinds = kind.expand();
    if kinds.contains(&TestKind::Ui) && profile.capabilities.ui {
        push_ui_gaps(profile, &mut gaps, &mut warnings);
    }

    if profile.language == Language::Flutter
        && kinds.contains(&TestKind::Integration)
        && profile.notes.iter().any(|n| n.contains("integration_test"))
    {
        warnings.push(
            "пакет integration_test отсутствует; цели Flutter integration пропускаются".into(),
        );
    }

    gaps.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.symbol.name.cmp(&b.symbol.name))
    });

    let limit = opts
        .max_items
        .unwrap_or(config.run.max_items as usize)
        .max(1);

    let mut items = Vec::new();
    for gap in gaps {
        if items.len() >= limit {
            break;
        }
        if !kinds.contains(&gap.kind) {
            if kind != TestKind::All && kinds.len() == 1 {
                if !(opts.reproduce_only && kinds[0] == TestKind::Unit) {
                    continue;
                }
            } else {
                continue;
            }
        }
        if kind == TestKind::All && !config.kind_enabled(gap.kind) {
            continue;
        }

        if kind != TestKind::All && kind != TestKind::Ui && gap.kind != kind && kinds.len() == 1 {
            continue;
        }
        if gap.kind == TestKind::Ui && !profile.ui_ready && profile.language != Language::Flutter {
            warnings.push(format!(
                "пропуск UI-цели '{}': UI-раннер не готов",
                gap.symbol.name
            ));
            continue;
        }
        if gap.kind == TestKind::Integration
            && profile.language == Language::Flutter
            && profile.notes.iter().any(|n| n.contains("integration_test"))
        {
            continue;
        }

        let target_path = adapter.suggest_path(profile, &gap.symbol, gap.kind);
        let abs = profile.root.join(&target_path);
        let append_to_existing = opts.prefer_append && abs.exists();
        let cases = default_cases(&gap);
        items.push(PlanItem {
            kind: gap.kind,
            gap,
            target_path,
            cases,
            append_to_existing,
            bug_context: opts.bug_text.clone(),
        });
    }

    let plan = TestPlan { items, warnings };
    let _ = crate::plan_store::save_plan(&profile.root, &plan);
    Ok(plan)
}

fn push_ui_gaps(profile: &ProjectProfile, gaps: &mut Vec<Gap>, warnings: &mut Vec<String>) {
    if profile.language == Language::Flutter {
        gaps.push(Gap {
            symbol: Symbol {
                name: "widget_smoke".into(),
                kind: "ui".into(),
                file: PathBuf::from("lib/main.dart"),
                start_line: 1,
                end_line: 1,
                signature: "Flutter widget test".into(),
                body: String::new(),
                is_public: true,
                uses_io: true,
                branch_score: 1,
            },
            kind: TestKind::Ui,
            reason: "Flutter widget/UI smoke".into(),
            priority: 55,
        });
        return;
    }
    if profile.root.join("index.html").exists() {
        if !profile.ui_ready {
            warnings.push(
                "Playwright не установлен; перед UI-тестами выполните `tester ui init`".into(),
            );
        } else {
            gaps.push(ui_gap(profile));
        }
    }
}

fn capability_allows(profile: &ProjectProfile, kind: TestKind) -> bool {
    match kind {
        TestKind::Unit => profile.capabilities.unit,
        TestKind::Integration => profile.capabilities.integration,
        TestKind::Contract => profile.capabilities.contract,
        TestKind::Ui => profile.capabilities.ui,
        TestKind::All => true,
    }
}

fn has_api_signal(profile: &ProjectProfile, symbol: &Symbol) -> bool {
    if detect_api_surface(&profile.root) {
        return true;
    }
    let b = format!("{} {}", symbol.signature, symbol.body).to_ascii_lowercase();
    b.contains("router")
        || b.contains("handler")
        || b.contains("endpoint")
        || b.contains("request")
        || b.contains("response")
        || b.contains("axum")
        || b.contains("actix")
        || b.contains("fastapi")
        || b.contains("express")
        || b.contains("ktor")
        || b.contains("spring")
        || b.contains("laravel")
        || b.contains("http.get")
        || b.contains("http.post")
}

fn detect_api_surface(root: &Path) -> bool {
    crate::api_detect::detect_api_surface(root)
}

fn ui_gap(_profile: &ProjectProfile) -> Gap {
    Gap {
        symbol: Symbol {
            name: "greet_form".into(),
            kind: "ui".into(),
            file: PathBuf::from("index.html"),
            start_line: 1,
            end_line: 1,
            signature: "UI: greet form".into(),
            body: String::new(),
            is_public: true,
            uses_io: true,
            branch_score: 1,
        },
        kind: TestKind::Ui,
        reason: "пользовательский сценарий формы".into(),
        priority: 50,
    }
}

fn is_trivial(symbol: &Symbol) -> bool {
    let trimmed = symbol.body.trim();
    let name = symbol.name.to_ascii_lowercase();
    if trimmed == "{}"
        || trimmed == "{ }"
        || trimmed.contains("unimplemented!")
        || trimmed.contains("todo!")
        || trimmed.contains("pass  #")
        || trimmed == "pass"
        || trimmed.contains("NotImplemented")
    {
        return true;
    }

    if (name.starts_with("get_") || name.starts_with("is_") || name.starts_with("has_"))
        && symbol.branch_score <= 1
        && !symbol.uses_io
        && symbol.body.lines().count() <= 4
    {
        return true;
    }
    symbol.branch_score == 0 && !symbol.uses_io && symbol.body.lines().count() <= 2
}

fn infer_kind(profile: &ProjectProfile, symbol: &Symbol) -> TestKind {
    if uses_io_strong(symbol) {
        if profile.language.supported()
            && has_api_signal(profile, symbol)
            && profile.capabilities.contract
        {
            return TestKind::Contract;
        }
        if profile.capabilities.integration {
            return TestKind::Integration;
        }
    }
    TestKind::Unit
}

fn uses_io_strong(symbol: &Symbol) -> bool {
    if symbol.uses_io {
        return true;
    }
    let b = symbol.body.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "std::fs",
        "file::",
        "open(",
        "read_to_string",
        "reqwest",
        "ureq",
        "http.get",
        "http.post",
        "fetch(",
        "axios",
        "requests.",
        "httpclient",
        "okhttp",
        "sqlx",
        "diesel",
        "postgres",
        "mongodb",
        "redis",
        "sqlite",
        "curl_",
        "pdo(",
        "mysqli",
        "file(",
        "pathlib",
        "tempfile",
    ];
    MARKERS.iter().any(|m| b.contains(m))
}

fn priority(symbol: &Symbol, kind: TestKind) -> u32 {
    let base = match kind {
        TestKind::Unit => 100,
        TestKind::Integration => 80,
        TestKind::Contract => 70,
        TestKind::Ui => 50,
        TestKind::All => 0,
    };
    base + symbol.branch_score * 8 + if symbol.uses_io { 0 } else { 10 }
}

fn reason(symbol: &Symbol, kind: TestKind) -> String {
    match kind {
        TestKind::Unit => format!("публичный {} без подходящих тестов", symbol.kind),
        TestKind::Integration => "граница I/O или сервиса без интеграционного покрытия".into(),
        TestKind::Contract => "HTTP-поверхность без контрактного покрытия".into(),
        TestKind::Ui => "пользовательский сценарий без UI-покрытия".into(),
        TestKind::All => String::new(),
    }
}

fn default_cases(gap: &Gap) -> Vec<PlannedCase> {
    match gap.kind {
        TestKind::Ui => vec![
            PlannedCase {
                name: "greets_entered_name".into(),
                description: "отправить форму приветствия".into(),
                input: "name=Ada".into(),
                expectation: "статус показывает Hello, Ada!".into(),
            },
            PlannedCase {
                name: "empty_name_validation".into(),
                description: "пустой ввод / валидация".into(),
                input: "name=".into(),
                expectation: "показана ошибка или кнопка disabled".into(),
            },
        ],
        TestKind::Integration | TestKind::Contract => vec![
            PlannedCase {
                name: format!("{}_ok", gap.symbol.name),
                description: "успешный путь на локальных данных (temp dir / in-memory)".into(),
                input: "валидный локальный ввод".into(),
                expectation: "возвращает Ok/ожидаемое значение без внешних сервисов".into(),
            },
            PlannedCase {
                name: format!("{}_err", gap.symbol.name),
                description: "отсутствующий или неверный ввод".into(),
                input: "нет пути / неизвестное имя".into(),
                expectation: "возвращает Err/None/ошибку".into(),
            },
            PlannedCase {
                name: format!("{}_boundary", gap.symbol.name),
                description: "граничный контракт (статус/схема)".into(),
                input: "минимально валидный payload".into(),
                expectation: "статус и поля ответа соответствуют контракту".into(),
            },
        ],
        _ => vec![
            PlannedCase {
                name: format!("{}_happy", gap.symbol.name),
                description: "типичный успешный ввод".into(),
                input: "представительные значения".into(),
                expectation: "совпадает с наблюдаемым поведением".into(),
            },
            PlannedCase {
                name: format!("{}_error", gap.symbol.name),
                description: "путь ошибки (если виден в коде)".into(),
                input: "невалидный / отсутствующий ввод".into(),
                expectation: "ошибка или безопасный fallback".into(),
            },
            PlannedCase {
                name: format!("{}_edge", gap.symbol.name),
                description: "граница / крайний случай".into(),
                input: "нули, пустые значения или экстремумы из кода".into(),
                expectation: "стабильное поведение на границе".into(),
            },
        ],
    }
}

fn git_changed_files(root: &Path, base: &str) -> Result<HashSet<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return Ok(HashSet::new());
    };
    if !output.status.success() {
        return Ok(HashSet::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

fn expand_callers(symbols: &[Symbol], changed: &HashSet<String>) -> HashSet<String> {
    let changed_names: HashSet<_> = symbols
        .iter()
        .filter(|s| {
            let f = s.file.to_string_lossy();
            changed
                .iter()
                .any(|c| c.ends_with(f.as_ref()) || f.ends_with(c))
        })
        .map(|s| s.name.clone())
        .collect();
    let mut files = HashSet::new();
    for s in symbols {
        if changed_names.iter().any(|n| s.body.contains(n.as_str())) {
            files.insert(s.file.to_string_lossy().into_owned());
        }
    }
    files
}

fn extract_bug_tokens(bug: &str) -> HashSet<String> {
    let re = regex::Regex::new(r"[A-Za-z_][A-Za-z0-9_\./\\-]{2,}").unwrap();
    re.find_iter(bug)
        .map(|m| m.as_str().to_string())
        .filter(|t| {
            !matches!(
                t.as_str(),
                "Error"
                    | "error"
                    | "at"
                    | "in"
                    | "from"
                    | "thread"
                    | "panic"
                    | "Exception"
                    | "File"
                    | "line"
            )
        })
        .collect()
}

fn fuzzy_hit(symbol: &Symbol, token: &str) -> bool {
    let t = token.to_ascii_lowercase();
    let name = symbol.name.to_ascii_lowercase();
    let file = symbol.file.to_string_lossy().to_ascii_lowercase();
    name.contains(&t)
        || t.contains(&name)
        || file.contains(&t)
        || symbol.body.to_ascii_lowercase().contains(&t)
        || levenshtein_le3(&name, &t)
}

fn levenshtein_le3(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if (a.len() as isize - b.len() as isize).unsigned_abs() > 3 {
        return false;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost));
        }
        prev = cur;
        if prev.iter().all(|&d| d > 3) && i > 3 {

        }
    }
    prev[b.len()] <= 3
}

fn stack_match_score(symbol: &Symbol, frames: &[StackFrame]) -> u32 {
    let mut score = 0u32;
    for f in frames {
        if let Some(sym) = &f.symbol {
            if symbol.name == *sym || symbol.name.ends_with(sym) || sym.ends_with(&symbol.name) {
                score += 3;
            }
        }
        if !f.file.is_empty() {
            let sf = symbol.file.to_string_lossy();
            if sf.ends_with(&f.file) || f.file.ends_with(sf.as_ref()) {
                score += 2;
                if let Some(line) = f.line {
                    if line >= symbol.start_line && line <= symbol.end_line {
                        score += 4;
                    }
                }
            }
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::discover::discover;
    use std::path::Path;

    fn write_rust(dir: &Path) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"d\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[lib]\npath=\"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            r#"pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn multiply(a: i32, b: i32) -> i32 { a * b }
pub fn get_name() -> &'static str { "x" }
pub fn read_config(path: &std::path::Path) -> std::io::Result<String> {
    Ok(std::fs::read_to_string(path)?.trim().to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_works() { assert_eq!(add(2, 3), 5); }
}
"#,
        )
        .unwrap();
    }

    #[test]
    fn unit_plan_includes_multiply_not_add() {
        let dir = tempfile::tempdir().unwrap();
        write_rust(dir.path());
        let profile = discover(dir.path()).unwrap();
        let config = Config::load(dir.path()).unwrap();
        let plan = build_plan(&profile, &config, TestKind::Unit).unwrap();
        let names: Vec<_> = plan
            .items
            .iter()
            .map(|i| i.gap.symbol.name.as_str())
            .collect();
        assert!(names.contains(&"multiply"), "{names:?}");
        assert!(!names.contains(&"add"), "{names:?}");
        assert!(!names.contains(&"get_name"), "{names:?}");
    }

    #[test]
    fn integration_plan_includes_read_config() {
        let dir = tempfile::tempdir().unwrap();
        write_rust(dir.path());
        let profile = discover(dir.path()).unwrap();
        let config = Config::load(dir.path()).unwrap();
        let plan = build_plan(&profile, &config, TestKind::Integration).unwrap();
        let names: Vec<_> = plan
            .items
            .iter()
            .map(|i| i.gap.symbol.name.as_str())
            .collect();
        assert!(names.contains(&"read_config"), "{names:?}");
        assert!(!names.contains(&"multiply"), "{names:?}");
    }

    #[test]
    fn max_items_respected() {
        let dir = tempfile::tempdir().unwrap();
        write_rust(dir.path());
        let profile = discover(dir.path()).unwrap();
        let config = Config::load(dir.path()).unwrap();
        let plan = build_plan_with(
            &profile,
            &config,
            TestKind::Unit,
            &PlanOptions {
                max_items: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(plan.items.len() <= 1);
    }
}
