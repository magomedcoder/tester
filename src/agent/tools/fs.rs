use super::{Tool, ToolKind, arg_str, arg_str_opt, truncate};
use crate::agent::AgentContext;
use crate::security::{path_escapes_root, skip_dir_name};
use anyhow::{Result, bail};
use regex::Regex;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn resolve(ctx: &AgentContext, rel: &str) -> Result<PathBuf> {
    let path = PathBuf::from(rel);
    if path_escapes_root(&ctx.profile.root, &path) {
        bail!("путь выходит за корень проекта: {rel}");
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
                "max_chars": {"type": "integer", "description": "Лимит символов"}
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
        ctx.audit.record("read_file", &rel, "ok");
        Ok(truncate(&text, max))
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
        "Поиск regex по исходникам проекта"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path": {"type": "string"},
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
        let re = Regex::new(&pattern)?;
        let start = arg_str_opt(args, "path")
            .map(|p| resolve(ctx, &p))
            .transpose()?
            .unwrap_or_else(|| ctx.profile.root.clone());
        let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(80) as usize;
        let mut hits = Vec::new();
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
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            for (i, line) in text.lines().enumerate() {
                if re.is_match(line) {
                    let rel = entry
                        .path()
                        .strip_prefix(&ctx.profile.root)
                        .unwrap_or(entry.path());
                    hits.push(format!("{}:{}:{}", rel.display(), i + 1, line.trim()));
                    if hits.len() >= max {
                        break;
                    }
                }
            }
            if hits.len() >= max {
                break;
            }
        }
        ctx.audit
            .record("grep", &pattern, &format!("{} hits", hits.len()));
        Ok(if hits.is_empty() {
            "(нет совпадений)".into()
        } else {
            hits.join("\n")
        })
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
        use crate::apply::apply_draft;
        use crate::model::{Draft, TestKind};
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
        use crate::apply::apply_draft;
        use crate::model::{Draft, TestKind};
        let rel = arg_str(args, "path")?;
        let old = arg_str(args, "old")?;
        let new = arg_str(args, "new")?;
        let abs = resolve(ctx, &rel)?;
        let text = std::fs::read_to_string(&abs)?;
        if !text.contains(&old) {
            bail!("фрагмент old не найден в {rel}");
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
