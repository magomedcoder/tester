use crate::cli::args::{OutputFormat, WikiCommands};
use crate::cli::helpers::{need_llm_with_model, prepare_project};
use anyhow::{Result, bail};
use std::path::PathBuf;
use tester_core::error::ExitCode;
use tester_core::i18n::{self, Msg};
use tester_core::model::TestKind;
use tester_pipeline::generate::generate_draft;
use tester_pipeline::plan::{PlanOptions, build_plan_with};
use tester_pipeline::plan_store;
use tester_pipeline::validate::validate_draft;
use tester_wiki::{SyncOpts, analyze, sync_ex};

pub(crate) fn dispatch_wiki(
    cmd: WikiCommands,
    format: OutputFormat,
    model: Option<&str>,
) -> Result<ExitCode> {
    match cmd {
        WikiCommands::Sync {
            path,
            url,
            wiki_path,
            max_pages,
        } => cmd_wiki_sync(&path, url, wiki_path, max_pages, format),
        WikiCommands::Analyze { path, llm, no_llm } => {
            cmd_wiki_analyze(&path, model, llm && !no_llm, format)
        }
        WikiCommands::Plan {
            path,
            version,
            max_items,
        } => cmd_wiki_plan(&path, version, max_items, format),
        WikiCommands::Run {
            path,
            url,
            wiki_path,
            version,
            max_pages,
            max_items,
            write,
            run_tests,
        } => cmd_wiki_run(
            &path,
            url,
            wiki_path,
            version,
            max_pages,
            max_items,
            write,
            run_tests,
            model,
            format,
        ),
    }
}

pub(crate) fn cmd_wiki_sync(
    path: &PathBuf,
    url: Option<String>,
    wiki_path: Option<PathBuf>,
    max_pages: usize,
    format: OutputFormat,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let result = sync_ex(
        &profile.root,
        SyncOpts {
            url,
            path: wiki_path,
            max_pages,
        },
    )?;
    let source = &result.source;
    let index = &result.index;
    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "kind": source.kind,
                "url": source.url,
                "path": source.path,
                "pages": index.pages.len(),
                "empty_skipped": result.empty_skipped,
            })
        ),
        OutputFormat::Text => {
            println!(
                "{}",
                i18n::tr(
                    Msg::WikiSyncOk,
                    &[
                        ("kind", &format!("{:?}", source.kind)),
                        ("pages", &index.pages.len().to_string()),
                    ],
                )
            );
            if result.empty_skipped > 0 {
                println!(
                    "{}",
                    i18n::tr(
                        Msg::WikiSyncEmptySkipped,
                        &[("n", &result.empty_skipped.to_string())]
                    )
                );
            }
            if let Some(u) = &source.url {
                println!("{}", i18n::tr(Msg::WikiSourceLine, &[("src", u)]));
            }
            if let Some(p) = &source.path {
                println!(
                    "{}",
                    i18n::tr(Msg::WikiSourceLine, &[("src", &p.display().to_string())])
                );
            }
            println!(
                "{}",
                i18n::tr(
                    Msg::WikiCacheLine,
                    &[(
                        "path",
                        &profile.root.join(".tester/wiki").display().to_string()
                    )],
                )
            );
        }
    }
    Ok(ExitCode::OK)
}

pub(crate) fn cmd_wiki_analyze(
    path: &PathBuf,
    model: Option<&str>,
    use_llm: bool,
    format: OutputFormat,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let llm = if use_llm {
        let config = tester_core::config::Config::load()?;
        Some(need_llm_with_model(&config, model)?)
    } else {
        None
    };
    let (analysis, scenarios) = analyze(
        &profile.root,
        llm.as_ref().map(|l| l.as_ref() as &dyn tester_providers::Llm),
    )?;
    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "versions": analysis.versions,
                "entities": analysis.entities.len(),
                "scenarios": scenarios.scenarios.len(),
                "hubs": analysis.hub_pages.len(),
                "toc": analysis.toc.len(),
                "graph_edges": analysis.graph_edges.len(),
                "namespaces": analysis.namespaces,
            })
        ),
        OutputFormat::Text => {
            if !use_llm {
                println!("{}", i18n::t(Msg::NoteWikiHeuristicAnalyze));
            }
            println!(
                "{}",
                i18n::tr(
                    Msg::WikiAnalyzeOk,
                    &[
                        ("versions", &analysis.versions.len().to_string()),
                        ("scenarios", &scenarios.scenarios.len().to_string()),
                        ("entities", &analysis.entities.len().to_string()),
                        ("hubs", &analysis.hub_pages.len().to_string()),
                        ("toc", &analysis.toc.len().to_string()),
                        ("edges", &analysis.graph_edges.len().to_string()),
                    ],
                )
            );
            for sc in scenarios.scenarios.iter().take(20) {
                println!(
                    "{}",
                    i18n::tr(
                        Msg::WikiScenarioLine,
                        &[
                            ("version", &sc.version),
                            ("title", &sc.title),
                            ("steps", &sc.steps.len().to_string()),
                        ],
                    )
                );
            }
        }
    }
    Ok(ExitCode::OK)
}

pub(crate) fn cmd_wiki_plan(
    path: &PathBuf,
    version: Option<String>,
    max_items: Option<usize>,
    format: OutputFormat,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let config = tester_core::config::Config::load()?;
    let max = max_items.unwrap_or(config.run.max_items as usize);
    let items = tester_wiki::plan_items_from_wiki(&profile.root, version.as_deref(), max)?;
    if items.is_empty() {
        bail!("{}", i18n::t(Msg::ErrWikiNoScenarios));
    }
    let plan = tester_core::model::TestPlan {
        items,
        warnings: vec![i18n::t(Msg::WikiPlanWarning).into()],
    };
    plan_store::save_plan(&profile.root, &plan)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
        OutputFormat::Text => {
            println!(
                "{}",
                i18n::tr(Msg::WikiPlanOk, &[("n", &plan.items.len().to_string())])
            );
            for it in &plan.items {
                println!(
                    "{}",
                    i18n::tr(
                        Msg::WikiPlanItem,
                        &[
                            ("name", &it.gap.symbol.name),
                            ("path", &it.target_path.display().to_string()),
                        ],
                    )
                );
            }
        }
    }
    Ok(ExitCode::OK)
}

pub(crate) fn cmd_wiki_run(
    path: &PathBuf,
    url: Option<String>,
    wiki_path: Option<PathBuf>,
    version: Option<String>,
    max_pages: usize,
    max_items: Option<usize>,
    write: bool,
    run_tests: bool,
    model: Option<&str>,
    format: OutputFormat,
) -> Result<ExitCode> {
    cmd_wiki_sync(path, url, wiki_path, max_pages, OutputFormat::Text)?;
    cmd_wiki_analyze(path, model, true, OutputFormat::Text)?;

    let profile = prepare_project(path)?;
    let config = tester_core::config::Config::load()?;
    let llm = need_llm_with_model(&config, model)?;

    let plan = build_plan_with(
        &profile,
        &config,
        TestKind::Ui,
        &PlanOptions {
            max_items,
            ..Default::default()
        },
    )?;

    let plan = if let Some(v) = version {
        let max = max_items.unwrap_or(config.run.max_items as usize);
        let items = tester_wiki::plan_items_from_wiki(&profile.root, Some(&v), max)?;
        let plan = tester_core::model::TestPlan {
            items,
            warnings: plan.warnings,
        };
        plan_store::save_plan(&profile.root, &plan)?;
        plan
    } else {
        plan
    };

    if plan.items.is_empty() {
        bail!("{}", i18n::t(Msg::ErrWikiRunNoItems));
    }

    let mut profile = profile;
    if write && profile.language == tester_core::model::Language::Blank && !profile.ui_ready {
        println!("{}", i18n::t(Msg::WarnBlankUiScaffold));
        tester_lang::adapter::scaffold_playwright(&profile.root)?;
        profile = prepare_project(path)?;
    }

    println!(
        "{}",
        i18n::tr(
            Msg::WikiRunGenerating,
            &[("n", &plan.items.len().to_string())]
        )
    );
    let mut written = 0usize;
    let mut ran_ok = 0usize;
    let mut ran_fail = 0usize;
    if run_tests && !write {
        bail!("{}", i18n::t(Msg::ErrWikiRunNeedsWrite));
    }
    for item in &plan.items {
        let draft = generate_draft(llm.as_ref(), &profile, &config, item)?;
        if write {
            let abs = profile.root.join(&draft.path);
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&abs, &draft.content)?;
            written += 1;
            println!(
                "{}",
                i18n::tr(
                    Msg::WikiWrote,
                    &[("path", &draft.path.display().to_string())]
                )
            );
            if run_tests {
                match validate_draft(&profile, &config, &draft) {
                    Ok(res) => {
                        if res.ok {
                            ran_ok += 1;
                            println!(
                                "{}",
                                i18n::tr(
                                    Msg::WikiPlaywrightOk,
                                    &[("path", &draft.path.display().to_string())]
                                )
                            );
                        } else {
                            ran_fail += 1;
                            println!(
                                "{}",
                                i18n::tr(
                                    Msg::WikiPlaywrightFail,
                                    &[
                                        ("path", &draft.path.display().to_string()),
                                        ("cmd", &res.command),
                                    ],
                                )
                            );
                        }
                    }
                    Err(e) => {
                        ran_fail += 1;
                        println!(
                            "{}",
                            i18n::tr(Msg::WikiPlaywrightError, &[("err", &format!("{e:#}"))])
                        );
                    }
                }
            }
        } else {
            println!(
                "{}",
                i18n::tr(
                    Msg::WikiDraftPreview,
                    &[
                        ("path", &draft.path.display().to_string()),
                        ("bytes", &draft.content.len().to_string()),
                    ],
                )
            );
        }
    }

    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "items": plan.items.len(),
                "written": written,
                "playwright_ok": ran_ok,
                "playwright_fail": ran_fail,
            })
        ),
        OutputFormat::Text => println!(
            "{}",
            i18n::tr(
                Msg::WikiRunDone,
                &[
                    ("items", &plan.items.len().to_string()),
                    ("written", &written.to_string()),
                    ("ok", &ran_ok.to_string()),
                    ("fail", &ran_fail.to_string()),
                ],
            )
        ),
    }
    Ok(ExitCode::OK)
}
