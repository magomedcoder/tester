use super::analysis::OutlineFile;
use super::{Tool, ToolKind, arg_str, arg_str_opt, truncate};
use crate::AgentContext;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::PathBuf;
use tester_core::i18n::{self, Msg};
use tester_docs::{ConvertTo, convert_document, prefers_outline, show_document};

pub struct ShowFile;
impl Tool for ShowFile {
    fn name(&self) -> &str {
        "show_file"
    }
    fn description(&self) -> &str {
        i18n::t(Msg::ToolShowFile)
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "max_chars": {"type": "integer"}
            },
            "required": ["path"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let path = arg_str(args, "path")?;
        let max_chars = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(20_000) as usize;
        let p = PathBuf::from(&path);

        if prefers_outline(&p) {
            return OutlineFile.execute(ctx, args);
        }

        let shown = show_document(&ctx.profile.root, &p, &ctx.config.documents)?;
        ctx.audit.record(
            "show_file",
            &format!(
                "{} kind={} source={} cache={}",
                shown.path.display(),
                shown.kind.as_str(),
                shown.source,
                shown.cache_hit
            ),
            "ok",
        );
        let header = i18n::tr(
            Msg::DocsShowOk,
            &[
                ("path", &shown.path.display().to_string()),
                ("kind", shown.kind.as_str()),
                ("source", &shown.source),
                ("engine", &shown.engine),
                ("hit", &shown.cache_hit.to_string()),
                ("chars", &shown.text.chars().count().to_string()),
            ],
        );
        Ok(format!("{header}\n\n{}", truncate(&shown.text, max_chars)))
    }
}

pub struct ConvertDocument;
impl Tool for ConvertDocument {
    fn name(&self) -> &str {
        "convert_document"
    }
    fn description(&self) -> &str {
        i18n::t(Msg::ToolConvertDocument)
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "to": {"type": "string", "description": "txt | md | pdf"},
                "out": {"type": "string", "description": "optional output path under project"}
            },
            "required": ["path", "to"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let path = arg_str(args, "path")?;
        let to_raw = arg_str(args, "to")?;
        let to = ConvertTo::parse(&to_raw).ok_or_else(|| {
            anyhow::anyhow!(
                "{}",
                i18n::tr(
                    Msg::ErrDocsUnsupportedConvert,
                    &[("kind", "document"), ("to", &to_raw)]
                )
            )
        })?;
        let out = arg_str_opt(args, "out").map(PathBuf::from);
        let res = convert_document(
            &ctx.profile.root,
            PathBuf::from(&path).as_path(),
            to,
            out.as_deref(),
            &ctx.config.documents,
        )?;
        ctx.audit.record(
            "convert_document",
            &format!("{} -> {}", path, res.out_path.display()),
            "ok",
        );
        Ok(i18n::tr(
            Msg::DocsConvertOk,
            &[
                ("path", &path),
                ("out", &res.out_path.display().to_string()),
                ("engine", &res.engine),
            ],
        ))
    }
}
