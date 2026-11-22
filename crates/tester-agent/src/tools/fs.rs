use super::{Tool, ToolKind, arg_str, arg_str_opt, truncate};
use crate::AgentContext;
use anyhow::{Result, bail};
use regex::Regex;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tester_core::error::AppError;
use tester_core::i18n::{self, Msg};
use tester_core::security::{path_escapes_root, skip_dir_name};
use walkdir::WalkDir;

fn resolve(ctx: &AgentContext, rel: &str) -> Result<PathBuf> {
    let path = PathBuf::from(rel);
    if path_escapes_root(&ctx.profile.root, &path) {
        return Err(AppError::PathEscapesRoot {
            path: rel.to_string(),
        }
        .into());
    }
    Ok(ctx.profile.root.join(path))
}

pub struct ReadFile;
impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Прочитать файл проекта по относительному пути"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Относительный путь"},
                "max_chars": {"type": "integer", "description": "Лимит символов"},
                "start_line": {"type": "integer", "description": "Начальная строка (1-based)"},
                "end_line": {"type": "integer", "description": "Конечная строка (включительно)"}
            },
            "required": ["path"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let rel = arg_str(args, "path")?;
        let abs = resolve(ctx, &rel)?;
        let max = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(20_000) as usize;
        let text = std::fs::read_to_string(&abs)?;
        let start = args
            .get("start_line")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let end = args
            .get("end_line")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let body = if start.is_some() || end.is_some() {
            let s = start.unwrap_or(1).max(1);
            let e = end.unwrap_or(usize::MAX);
            let mut out = String::new();
            for (i, line) in text.lines().enumerate() {
                let n = i + 1;
                if n < s {
                    continue;
                }
                if n > e {
                    break;
                }
                out.push_str(&format!("{n:>5}| {line}\n"));
            }
            out
        } else {
            text
        };
        ctx.audit.record("read_file", &rel, "ok");
        Ok(truncate(&body, max))
    }
}

pub struct ListDir;
impl Tool for ListDir {
    fn name(&self) -> &str {
        "list_dir"
    }
    fn description(&self) -> &str {
        "Список файлов и каталогов"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Относительный каталог (по умолчанию .)"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let rel = arg_str_opt(args, "path").unwrap_or_else(|| ".".into());
        let abs = resolve(ctx, &rel)?;
        let mut entries = Vec::new();
        for e in std::fs::read_dir(&abs)? {
            let e = e?;
            let name = e.file_name().to_string_lossy().into_owned();
            if skip_dir_name(&name) {
                continue;
            }
            let kind = if e.file_type()?.is_dir() {
                "dir"
            } else {
                "file"
            };
            entries.push(format!("{kind}\t{name}"));
        }
        entries.sort();
        ctx.audit.record("list_dir", &rel, "ok");
        Ok(entries.join("\n"))
    }
}

pub struct GlobFiles;
impl Tool for GlobFiles {
    fn name(&self) -> &str {
        "glob"
    }
    fn description(&self) -> &str {
        "Найти файлы по шаблону (substring/suffix, напр. *.rs или test_)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "max": {"type": "integer"}
            },
            "required": ["pattern"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let pattern = arg_str(args, "pattern")?;
        let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
        let needle = pattern.replace('*', "");
        let mut out = Vec::new();
        for entry in WalkDir::new(&ctx.profile.root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry
                .path()
                .components()
                .any(|c| c.as_os_str().to_str().map(skip_dir_name).unwrap_or(false))
            {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&ctx.profile.root)
                .unwrap_or(entry.path());
            let s = rel.to_string_lossy();
            let name = entry.file_name().to_string_lossy();
            let ok = if pattern.starts_with('*') {
                name.ends_with(needle.as_str()) || s.ends_with(needle.as_str())
            } else if pattern.ends_with('*') {
                name.starts_with(needle.as_str()) || s.starts_with(needle.as_str())
            } else {
                s.contains(&pattern) || name.contains(&pattern)
            };
            if ok {
                out.push(s.into_owned());
                if out.len() >= max {
                    break;
                }
            }
        }
        ctx.audit
            .record("glob", &pattern, &format!("{} hits", out.len()));
        Ok(out.join("\n"))
    }
}

pub struct GrepFiles;
impl Tool for GrepFiles {
    fn name(&self) -> &str {
        "grep"
    }
    fn description(&self) -> &str {
        "Поиск regex по исходникам: path:line:col + snippet; фильтры glob/path_prefix/lang"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path": {"type": "string", "description": "Каталог или path_prefix"},
                "path_prefix": {"type": "string"},
                "glob": {"type": "string", "description": "Напр. *.rs или *.ts"},
                "lang": {"type": "string", "description": "rust|ts|python|go|..."},
                "case_insensitive": {"type": "boolean"},
                "word": {"type": "boolean", "description": "Граница слова"},
                "context": {"type": "integer", "description": "Строк контекста -C"},
                "max": {"type": "integer"}
            },
            "required": ["pattern"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let mut pattern = arg_str(args, "pattern")?;
        let word = args.get("word").and_then(|v| v.as_bool()).unwrap_or(false);
        if word {
            pattern = format!(r"\b(?:{pattern})\b");
        }
        let case_i = args
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let re = if case_i {
            Regex::new(&format!("(?i){pattern}"))?
        } else {
            Regex::new(&pattern)?
        };
        let start = arg_str_opt(args, "path")
            .or_else(|| arg_str_opt(args, "path_prefix"))
            .map(|p| resolve(ctx, &p))
            .transpose()?
            .unwrap_or_else(|| ctx.profile.root.clone());
        let path_prefix = arg_str_opt(args, "path_prefix").unwrap_or_default();
        let glob = arg_str_opt(args, "glob");
        let lang = arg_str_opt(args, "lang");
        let context = args.get("context").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let max = args
            .get("max")
            .and_then(|v| v.as_u64())
            .unwrap_or(ctx.config.analysis.grep_max_hits as u64)
            .max(1) as usize;
        let mut hits = Vec::new();
        let mut total = 0usize;
        for entry in WalkDir::new(&start).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry
                .path()
                .components()
                .any(|c| c.as_os_str().to_str().map(skip_dir_name).unwrap_or(false))
            {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&ctx.profile.root)
                .unwrap_or(entry.path());
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            if !path_prefix.is_empty() && !rel_s.starts_with(&path_prefix) {
                continue;
            }
            if let Some(ref g) = glob {
                let g = g.replace('*', "");
                let name = entry.file_name().to_string_lossy();
                if !(name.contains(g.as_str())
                    || rel_s.ends_with(g.as_str())
                    || name.ends_with(g.as_str()))
                {
                    continue;
                }
            }
            if let Some(ref lang) = lang {
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let ok = match lang.to_lowercase().as_str() {
                    "rust" | "rs" => ext == "rs",
                    "ts" | "typescript" => matches!(ext, "ts" | "tsx"),
                    "js" | "javascript" => matches!(ext, "js" | "jsx"),
                    "python" | "py" => ext == "py",
                    "go" => ext == "go",
                    "kotlin" | "kt" => matches!(ext, "kt" | "kts"),
                    "java" => ext == "java",
                    "php" => ext == "php",
                    "dart" => ext == "dart",
                    other => ext == other,
                };
                if !ok {
                    continue;
                }
            }
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if let Some(m) = re.find(line) {
                    total += 1;
                    if hits.len() < max {
                        let col = m.start() + 1;
                        let mut block = format!("{rel_s}:{}:{col}:{}", i + 1, line.trim());
                        if context > 0 {
                            let a = i.saturating_sub(context);
                            let b = (i + context + 1).min(lines.len());
                            for (j, l) in lines.iter().enumerate().take(b).skip(a) {
                                if j == i {
                                    continue;
                                }
                                block.push_str(&format!("\n  {}:{}", j + 1, l.trim_end()));
                            }
                        }
                        hits.push(block);
                    }
                }
            }
        }
        ctx.audit.record("grep", &pattern, &format!("{total} hits"));
        if hits.is_empty() {
            Ok("(нет совпадений)".into())
        } else if total > hits.len() {
            Ok(format!(
                "{}\n... и ещё {} совпадений",
                hits.join("\n"),
                total - hits.len()
            ))
        } else {
            Ok(hits.join("\n"))
        }
    }
}

pub struct WriteFile;
impl Tool for WriteFile {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Записать файл (только тестовые пути по write policy адаптера)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"},
                "kind": {"type": "string", "description": "unit|integration|ui|contract"}
            },
            "required": ["path", "content"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        use tester_core::model::{Draft, TestKind};
        use tester_pipeline::apply::apply_draft;
        let rel = arg_str(args, "path")?;
        let content = arg_str(args, "content")?;
        let kind: TestKind = arg_str_opt(args, "kind")
            .unwrap_or_else(|| "unit".into())
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        let draft = Draft {
            path: PathBuf::from(&rel),
            content,
            kind,
            symbol_name: Path::new(&rel)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("test")
                .to_string(),
            filter_name: String::new(),
            append: false,
        };
        if ctx.dry_run {
            let out = apply_draft(&ctx.profile, &draft, false, true)?;
            ctx.audit.record("write_file", &rel, "dry_run");
            return Ok(format!("{out:?}"));
        }
        let out = apply_draft(&ctx.profile, &draft, true, true)?;
        ctx.audit.record("write_file", &rel, "written");
        Ok(format!("{out:?}"))
    }
}

pub struct ApplyPatch;
impl Tool for ApplyPatch {
    fn name(&self) -> &str {
        "apply_patch"
    }
    fn description(&self) -> &str {
        "Применить простой unified-like patch: заменить old на new в файле"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "old": {"type": "string"},
                "new": {"type": "string"},
                "kind": {"type": "string"}
            },
            "required": ["path", "old", "new"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        use tester_core::model::{Draft, TestKind};
        use tester_pipeline::apply::apply_draft;
        let rel = arg_str(args, "path")?;
        let old = arg_str(args, "old")?;
        let new = arg_str(args, "new")?;
        let abs = resolve(ctx, &rel)?;
        let text = std::fs::read_to_string(&abs)?;
        if !text.contains(&old) {
            bail!(
                "{}",
                i18n::tr(Msg::ErrPatchFragmentNotFound, &[("path", &rel)])
            );
        }
        let content = text.replacen(&old, &new, 1);
        let kind: TestKind = arg_str_opt(args, "kind")
            .unwrap_or_else(|| "unit".into())
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        let draft = Draft {
            path: PathBuf::from(&rel),
            content,
            kind,
            symbol_name: "patch".into(),
            filter_name: String::new(),
            append: false,
        };
        let written = !ctx.dry_run;
        let out = apply_draft(&ctx.profile, &draft, written, true)?;
        ctx.audit.record(
            "apply_patch",
            &rel,
            if written { "written" } else { "dry_run" },
        );
        Ok(format!("{out:?}"))
    }
}
