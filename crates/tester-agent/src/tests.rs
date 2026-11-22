use crate::agent_loop::{AgentLoop, AgentOptions};
use crate::session::AgentSession;
use crate::tools::ToolRegistry;
use crate::{AgentContext, AuditLog, PermissionPolicy};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tester_core::config::Config;
use tester_core::model::{Capabilities, Language, ProjectProfile, TestStyle};
use tester_providers::{AssistantToolCall, ChatMessage, ChatResult, FunctionCall, MockLlm, Usage};

fn profile(root: &std::path::Path) -> ProjectProfile {
    ProjectProfile {
        root: root.to_path_buf(),
        language: Language::Rust,
        package_name: "demo".into(),
        edition: Some("2021".into()),
        supported: true,
        test_command: "cargo test".into(),
        test_style: TestStyle::InlineCfgTest,
        src_dir: Some(root.join("src")),
        tests_dir: Some(root.join("tests")),
        has_existing_tests: false,
        unit_framework: Some("cargo".into()),
        ui_framework: None,
        ui_ready: false,
        capabilities: Capabilities {
            unit: true,
            integration: true,
            contract: true,
            ui: false,
        },
        notes: vec![],
        android_opt_in: false,
        package_filter: None,
    }
}

#[test]
fn agent_loop_ends_on_text_without_tools() {
    let dir = tempfile::tempdir().unwrap();
    let llm = Arc::new(MockLlm::with_complete("готово"));
    let cfg = Config::load().unwrap();
    let ctx = AgentContext {
        profile: profile(dir.path()),
        config: cfg,
        llm: llm.clone(),
        dry_run: true,
        yes: true,
        stream: false,
        cancel: Arc::new(AtomicBool::new(false)),
        audit: AuditLog::new(),
        permissions: PermissionPolicy::from_flags(true, true),
    };
    let registry = ToolRegistry::new(vec![]);
    let mut session = AgentSession::new();
    session.messages.push(ChatMessage::system("sys"));
    session.messages.push(ChatMessage::user("hi"));
    let mut engine = AgentLoop::new(
        &ctx,
        registry,
        AgentOptions {
            max_turns: 5,
            max_tool_calls: 10,
            deadline_secs: 30,
            stream: false,
            on_delta: None,
        },
    );
    let out = engine.run(&mut session).unwrap();
    assert_eq!(out.final_text, "готово");
    assert!(!out.cancelled);
}

#[test]
fn agent_loop_executes_tool_then_finishes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.txt"), "world").unwrap();
    let script = vec![
        ChatResult {
            content: None,
            tool_calls: vec![AssistantToolCall {
                id: "1".into(),
                kind: "function".into(),
                function: FunctionCall {
                    name: "read_file".into(),
                    arguments: r#"{"path":"hello.txt"}"#.into(),
                },
            }],
            usage: Some(Usage {
                prompt_tokens: 1,
                completion_tokens: 1,
            }),
        },
        ChatResult {
            content: Some("прочитал".into()),
            tool_calls: vec![],
            usage: Some(Usage {
                prompt_tokens: 2,
                completion_tokens: 2,
            }),
        },
    ];
    let llm = Arc::new(MockLlm::with_chat(script));
    let cfg = Config::load().unwrap();
    let ctx = AgentContext {
        profile: profile(dir.path()),
        config: cfg,
        llm,
        dry_run: true,
        yes: true,
        stream: false,
        cancel: Arc::new(AtomicBool::new(false)),
        audit: AuditLog::new(),
        permissions: PermissionPolicy::from_flags(true, true),
    };
    let registry = crate::tools::default_registry(&ctx);
    let mut session = AgentSession::new();
    session.messages.push(ChatMessage::user("read hello"));
    let mut engine = AgentLoop::new(
        &ctx,
        registry,
        AgentOptions {
            max_turns: 5,
            max_tool_calls: 10,
            deadline_secs: 30,
            stream: false,
            on_delta: None,
        },
    );
    let out = engine.run(&mut session).unwrap();
    assert_eq!(out.final_text, "прочитал");
    assert_eq!(out.tool_calls, 1);
    assert!(ctx.audit.entries().iter().any(|e| e.tool == "read_file"));
}

#[test]
fn tool_error_becomes_tool_result_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let script = vec![
        ChatResult {
            content: None,
            tool_calls: vec![AssistantToolCall {
                id: "1".into(),
                kind: "function".into(),
                function: FunctionCall {
                    name: "read_file".into(),
                    arguments: r#"{"path":"../outside.txt"}"#.into(),
                },
            }],
            usage: None,
        },
        ChatResult {
            content: Some("понял".into()),
            tool_calls: vec![],
            usage: None,
        },
    ];
    let llm = Arc::new(MockLlm::with_chat(script));
    let cfg = Config::load().unwrap();
    let ctx = AgentContext {
        profile: profile(dir.path()),
        config: cfg,
        llm,
        dry_run: true,
        yes: true,
        stream: false,
        cancel: Arc::new(AtomicBool::new(false)),
        audit: AuditLog::new(),
        permissions: PermissionPolicy::from_flags(true, true),
    };
    let registry = crate::tools::default_registry(&ctx);
    let mut session = AgentSession::new();
    session.messages.push(ChatMessage::user("escape"));
    let mut engine = AgentLoop::new(
        &ctx,
        registry,
        AgentOptions {
            max_turns: 5,
            max_tool_calls: 10,
            deadline_secs: 30,
            stream: false,
            on_delta: None,
        },
    );
    let out = engine.run(&mut session).unwrap();
    assert_eq!(out.final_text, "понял");
    let tool_msg = session.messages.iter().find(|m| m.role == "tool").unwrap();
    assert!(
        tool_msg
            .content
            .as_deref()
            .unwrap_or("")
            .contains("ошибка инструмента")
            || tool_msg
                .content
                .as_deref()
                .unwrap_or("")
                .contains("выходит за корень")
    );
}
