use super::Msg;

pub(crate) fn t(msg: Msg) -> &'static str {
    match msg {
        Msg::ErrorPrefix => "error: ",
        Msg::CancelHint => "\n^C - cancel after the current step...",
        Msg::NoGoals => "no goals",
        Msg::WarningPrefix => "warning: ",
        Msg::Language => "language",
        Msg::Package => "package",
        Msg::Supported => "supported",
        Msg::TestCommand => "test_command",
        Msg::TestStyle => "test_style",
        Msg::HasTests => "has_tests",
        Msg::UnitFramework => "unit_framework",
        Msg::UiFramework => "ui_framework",
        Msg::UiReady => "ui_ready",
        Msg::Capabilities => "capabilities",
        Msg::Note => "note",
        Msg::PlanArrow => "->",
        Msg::LlmUnavailable => "llm unavailable",
        Msg::UnsupportedLanguage => "language recognized but not supported yet",
        Msg::PlaywrightInit => "Playwright initialized in",
        Msg::ConfigLabel => "config",
        Msg::TestsLabel => "tests",

        Msg::TuiNeedsTty => {
            "tester requires an interactive terminal; use `tester chat <PATH>` / `--plain`, or run in a TTY"
        }
        Msg::TitleCommands => " /commands↑↓TabEnter ",
        Msg::TitleOpenProject => " open project ",
        Msg::TitleFiles => " @files ",
        Msg::TitleWorktrees => " worktrees ",
        Msg::PopupNoMatches => "no matches",
        Msg::AttachHint => "attach file or symbol",
        Msg::SlashHelp => "commands and keybinds",
        Msg::SlashScan => "discover project language/tests",
        Msg::SlashPlan => "build test plan (unit|e2e|...)",
        Msg::SlashRun => "run agent with prompt",
        Msg::SlashClear => "clear chat session",
        Msg::SlashAudit => "audit capability summary",
        Msg::SlashExit => "quit session",
        Msg::SlashGrep => "regex search in repo",
        Msg::SlashSymbol => "find symbol definition",
        Msg::SlashRefs => "find symbol references",
        Msg::SlashSearch => "semantic / natural search",
        Msg::SlashIndex => "index status or rebuild",
        Msg::SlashMap => "project structure map",
        Msg::SlashListFiles => "list project files",
        Msg::SlashShowFile => "show source outline or PDF/office/image text",
        Msg::SlashConvert => "convert a document (txt|md|pdf)",
        Msg::SlashManual => "open manual overlay",
        Msg::SlashRecent => "recent projects picker",
        Msg::SlashSession => "list / switch / new chat session",
        Msg::SlashPrune => "delete stored sessions",
        Msg::SlashDiff => "git diff viewer",
        Msg::SlashBranch => "git branch checkout",
        Msg::SlashProposePlan => "preview plan before run",
        Msg::SlashSecurity => "security scan overlay",
        Msg::SecurityOverlayTitle => "security",
        Msg::PickerStatusProject => "project root detected - Ctrl+Enter / Ctrl+S to open",
        Msg::PickerStatusNav => {
            "↑↓ navigate  Enter open folder  -> worktrees  Ctrl+Enter select  Esc cancel"
        }
        Msg::PickerStatusWorktrees => {
            "worktrees - Enter/Ctrl+Enter open  <- back to folders  Esc cancel"
        }
        Msg::PickerFilter => "filter",
        Msg::PickerProjectTag => "[project]",
        Msg::PickerWorktreeTag => "[worktree]",
        Msg::SessionFailedOpen => "failed to open project",
        Msg::SessionFixHint => "fix project / config, then Ctrl+O",
        Msg::SessionStatusDryRun => "dry-run  Enter send  @file  /help",
        Msg::SessionStatusAuto => "auto-approve  Enter send  @file  /help",
        Msg::SessionStatusBlocked => "write/shell need y/N confirm  Enter send  @file  /help",
        Msg::SessionCleared => "session cleared",
        Msg::SessionSwitched => "session ready",
        Msg::SessionRunning => "running... Esc to cancel",
        Msg::SessionCancelReq => "cancel requested...",
        Msg::SessionHelp => {
            "\
Tester help

Keys
  Enter            send message
  Shift+Enter / Ctrl+Enter / Ctrl+J   new line in prompt
  Esc Esc          cancel / close overlay
  Ctrl+C Ctrl+C    quit
  Ctrl+O           open project
  Ctrl+R           search prompt history
  Ctrl+X Ctrl+E    external editor
  ↑↓ / PgUp PgDn   scroll transcript
  y / Enter        allow tool
  n / Esc          deny tool

Prompt editing
  Ctrl+A / E       line start / end
  Ctrl+Home / End  buffer start / end
  Ctrl+W / U / K   kill word / line<- / line->
  Ctrl+Y           yank
  Ctrl+P / N       prompt history
  Ctrl+<- / ->       word jump
  Ctrl+V           paste clipboard
  Shift+Insert     paste
  mouse            native terminal select / copy (as usual)
  wheel            scroll transcript (not the prompt)
  ↑↓ / PgUp PgDn   scroll transcript

Attach
  @path            attach file or folder
  @path#L10-40     attach line range
  @Symbol          jump to symbol

Commands"
        }
        Msg::SessionUnknownCmd => "unknown command; /help",
        Msg::SessionUsageRun => "usage: /run <prompt>",
        Msg::SessionNoGoals => "no plan goals",
        Msg::SessionBadKind => "bad kind",
        Msg::SessionEmptyResponse => "(empty response)",
        Msg::SessionError => "error - see transcript",
        Msg::SessionThreadExit => "agent thread exited",
        Msg::SupportedWord => "supported",
        Msg::UnsupportedWord => "unsupported",
        Msg::PermTitle => " allow tool? ",
        Msg::PermPrompt => "permission required - y/n",
        Msg::ComposerPlaceholder => "Ask tester...  (@file  /commands)",
        Msg::EmptyTranscript => "Start a prompt, or /help",
        Msg::EmptyWorktrees => "no git worktrees",
        Msg::DoctorTitle => "# tester doctor\n\n",
        Msg::DoctorOk => "ok",
        Msg::DoctorFail => "FAIL",
        Msg::ReportTitle => "# tester report\n\n",
        Msg::ReportProject => "project",
        Msg::ReportLanguage => "language",
        Msg::ReportPackage => "package",
        Msg::ReportWarnings => "## Warnings\n\n",
        Msg::ReportFailures => "## Failure summary\n\n",
        Msg::ReportResults => {
            "## Results\n\n| symbol | kind | file | attempts | status | first_pass | density | usefulness | command |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n"
        }
        Msg::ReportTrace => "Trace",
        Msg::ReportStatusPlanned => "planned",
        Msg::ReportStatusDryRun => "dry_run",
        Msg::ReportStatusWritten => "written",
        Msg::ReportStatusPassed => "passed",
        Msg::ReportStatusUnfixed => "unfixed",
        Msg::ReportStatusSkipped => "skipped",
        Msg::ReportStatusFailedPath => "failed_path",

        Msg::BadgeThinking => "Thinking",
        Msg::BadgeWorking => "Working",
        Msg::BadgePerm => "perm",
        Msg::BadgePlan => "PLAN",
        Msg::StatusIdle => "idle",
        Msg::CtxGauge => "ctx {used}/{limit}",
        Msg::PermWaitYn => "wait y/N",
        Msg::StatusHelpHint => " /help",
        Msg::StatusOpenEditor => "Ctrl+E - open $EDITOR",
        Msg::TuiDryRunBadge => "dry-run",
        Msg::TuiNoProject => "no project",
        Msg::WinTitleBusy => "testerbusy{model}",
        Msg::WinTitleIdle => "testeridle",
        Msg::WinTitlePicker => "testeropen project",
        Msg::PermAllow => "allow",
        Msg::PermDeny => "deny",
        Msg::ComposerRevSearch => "bck-i-search: ",

        Msg::SessionOpenFailed => "session open failed: {err}",
        Msg::AuditFailed => "audit failed: {err}",
        Msg::ScanFailed => "scan failed: {err}",
        Msg::ScanSummary => "language={lang} package={pkg} cmd={cmd}",
        Msg::PlanFailed => "plan failed: {err}",
        Msg::GitCheckoutOk => "checked out {branch}",
        Msg::GitCheckoutFailed => "checkout failed: {err}",
        Msg::GitCheckoutError => "checkout error: {err}",
        Msg::ProposePlanApproved => "/propose_plan (approved)",
        Msg::ProposePlanTitle => "plan ({kind})",
        Msg::ProposePlanPrompt => "Execute this test plan:\n{body}\nReply with progress.",
        Msg::UsageGrep => "usage: /grep <pattern>",
        Msg::UsageSymbol => "usage: /symbol <name>",
        Msg::UsageRefs => "usage: /refs <name>",
        Msg::UsageSearch => "usage: /search <natural query>",
        Msg::UsageShowFile => "usage: /show_file <path>",
        Msg::UsageConvert => "usage: /convert <path> [txt|md|pdf]",
        Msg::GrepFailed => "grep failed: {err}",
        Msg::ToolMissing => "{name} tool missing",
        Msg::ToolFailed => "{name} failed: {err}",
        Msg::IndexRebuilt => "index rebuilt: {n} symbols",
        Msg::IndexRebuildFailed => "index rebuild failed: {err}",
        Msg::IndexFailed => "index failed: {err}",
        Msg::SessionCmdError => "/session: {err}",
        Msg::PruneUsage => "usage: /prune <id|-w|-o <days>|all>",
        Msg::PruneDone => "pruned {n} session(s)",
        Msg::PruneError => "/prune: {err}",
        Msg::SessionSwitchFailed => "session switch failed: {err}",
        Msg::SessionOutcomeOk => "ok",
        Msg::SessionOutcomeCancelled => "cancelled",
        Msg::SessionOutcomeLine => "turns={turns} tools={tools}{status}",
        Msg::EditorFailed => "editor: {err}",

        Msg::ManualContent => {
            "\
# Tester manual

## Navigation
- Enter - send prompt
- @path / @Symbol - attach
- /commands - slash palette
- Esc Esc - cancel / close overlay
- Ctrl+C Ctrl+C - quit
- Ctrl+R - reverse history search
- Ctrl+X Ctrl+E - external editor

## Slash
/scan /plan /run /grep /symbol /refs /search /index /map /show_file /list_files
/manual /diff /branch /recent /session /prune /propose_plan /audit /clear /exit

## Analysis tools
find_symbol, project_map, graph_callers, semantic_search (opt-in embeddings)

## Approvals
y/N for write/shellplan approve/reject via /propose_plan
"
        }
        Msg::ManualTocTitle => " TOC ",
        Msg::ManualTitleSearching => " /manual  Alt+S search: {q}_ ",
        Msg::ManualTitleIdle => " /manual  Alt+S searchEsc close ",
        Msg::ManualSearchNoMatches => "(no matches)",
        Msg::PlanOverlayFooter => " a approver rejecte editEsc cancel ",
        Msg::PlanOverlayTitle => " propose_plan ",
        Msg::DiffUnavailable => "(git diff unavailable)",
        Msg::DiffClean => "(clean working tree)",
        Msg::DiffTitle => "diff",
        Msg::DiffOverlayChrome => " {title}Esc closePgUp/Dn ",
        Msg::BranchNone => "(no git branches)",
        Msg::BranchItemTag => "branch",
        Msg::BranchOverlayTitle => " branch switcherEnter checkoutEsc ",
        Msg::SessionsNone => "(no sessions)",
        Msg::SessionsNewHint => "n = new",
        Msg::SessionTitleEmpty => "(empty)",
        Msg::SessionsOverlayTitle => " sessionsEnter openn newEsc close ",
        Msg::RecentHasTester => "has .tester",
        Msg::RecentProject => "project",
        Msg::RecentOverlayTitle => " recent projectsEnter openEsc close ",

        Msg::PickerOkStatus => "ok: {path} ({word})",
        Msg::PickerWarnStatus => "warn: {err}",
        Msg::ReplBanner => "tester chat - agent REPL. /help for commands, Ctrl+C to quit.",
        Msg::ReplPrompt => "tester> ",
        Msg::ReplSlashHelp => {
            "\
Commands:
  /scan
  /plan [kind]
  /run <prompt>
  /clear
  /audit
  /grep <pat>
  /symbol <name>
  /refs <name>
  /search <q>
  /index status|rebuild
  /map
  /list_files
  /show_file <path>
  /exit"
        }
        Msg::AuditEmpty => "audit: empty",
        Msg::AuditHeader => "audit: {n} actions",

        Msg::PlanProgress => "| plan -> generate -> validate -> repair ({n})",
        Msg::PlanItemNotFound => "plan item '{symbol}' not found for repair",
        Msg::PlainNeedsPath => "`--plain` requires PATH",
        Msg::PruneNeedsArg => "specify id | --all | --workspace | --older-than",
        Msg::PruneCli => "pruned {n}",
        Msg::SecurityPatchNote => "note: {note}",
        Msg::SecurityApplied => "applied: {path}",
        Msg::SecurityDryRunHint => "dry-run (pass --apply to write)",
        Msg::FindingNotFound => "finding not found: {id}",
        Msg::UsageLine => "usage: prompt={prompt} completion={completion} total={total}",
        Msg::ReplTurnsLine => "- turns={turns} tools={tools} tokens≈{tokens} {status}",
        Msg::ItemWritten => "written",
        Msg::ItemPassed => "passed",
        Msg::PlanShowEmpty => "no goals in .tester/plan.json",
        Msg::LlmClientCreate => "creating llm client",
        Msg::ErrPathEscapes => "path escapes project root: {path}",
        Msg::ErrPathForbidden => "path forbidden: {detail}",
        Msg::ErrPathNeedsUpdate => "refusing to overwrite an existing file without --update",
        Msg::ErrManifestWrite => "refusing to write manifest {name}",
        Msg::ErrGateNoAssertion => "gate: no assertion (assert/expect/REQUIRE)",
        Msg::ErrGateWeakTest => "gate: weak test (only snapshot/sleep without behavior checks)",
        Msg::ErrGateForbiddenNetwork => "gate: forbidden network/Docker for an external service",
        Msg::ErrGateUnitNetwork => "gate: unit test pulls in network/DB - use integration",
        Msg::DoctorNoAdapter => "no adapter",
        Msg::DoctorNotRequired => "not required",
        Msg::DoctorGradlewFound => {
            "gradlew found; connectedAndroidTest will need a device/emulator"
        }
        Msg::DoctorGradlewMissing => "gradlew not found",
        Msg::DoctorUiState => "framework={framework} ready={ready}",
        Msg::DoctorSecurityRules => "{n} rules, enabled={enabled}, deep={deep}",
        Msg::SecReportTitle => "# Security report",
        Msg::SecCounts => "## Counts",
        Msg::SecNotes => "## Notes",
        Msg::SecFindings => "## Findings",
        Msg::SecNoFindings => "_No findings above configured severities._",
        Msg::SecScanSummary => "security scan: {n} findings ({pack} {version})",
        Msg::SecScanFailed => "security scan failed: {err}",
        Msg::SecFixLine => "Fix ({strategy}) : {summary}",
        Msg::SecNoteLine => "note: {note}",
        Msg::PermPromptCli => "Allow {tool}? {preview} [y/N] ",
        Msg::IoRead => "reading {path}",
        Msg::IoWrite => "writing {path}",
        Msg::IoCreate => "creating {path}",
        Msg::IoParse => "parsing {path}",
        Msg::AgentDeadline => "agent exceeded deadline {secs}s",
        Msg::MaxToolCalls => "max_tool_calls exceeded ({n})",
        Msg::UnknownTool => "unknown tool: {name}",
        Msg::AgentTurnLimit => "(turn limit reached)",
        Msg::AgentStepModel => "turn {turn}: calling model",
        Msg::AgentStepDone => "done",
        Msg::AgentStepTool => "tool {name}",
        Msg::SecMetaLine => {
            "- root: `{root}`\n- rule_pack: {pack}\n- deep: {deep}\n- findings: {n}\n"
        }
        Msg::SecFindingMeta => {
            "- file: `{path}`:{line}\n- category: {category}\n- confidence: {confidence}\n- evidence: {evidence}\n"
        }
        Msg::WarnFlutterIntegration => {
            "integration_test package is missing; Flutter integration goals are skipped"
        }
        Msg::WarnUiSkipped => "skipping UI goal '{symbol}': UI runner is not ready",
        Msg::WarnPlaywrightMissing => {
            "Playwright is not installed; run `tester ui init` before UI tests"
        }
        Msg::NoteDockerAllowed => {
            "allow_docker=true: the docker_info tool is available to the agent; external services are still not started automatically"
        }
        Msg::NoteApiSurface => {
            "API surface detected (OpenAPI/proto/router) - contract kind is available"
        }
        Msg::NoteCppKinds => "C++: contract/UI kinds unsupported; unit+integration via gtest",
        Msg::NoteBlankWorkspace => {
            "blank workspace: no language manifest - wiki/docs -> Playwright UI only"
        }
        Msg::NoteBlankNeedUiInit => {
            "Playwright not scaffolded yet - run `tester ui init` or `wiki run --write`"
        }
        Msg::WarnBlankUiScaffold => "blank workspace: scaffolding Playwright before write",
        Msg::NoteAndroidOptIn => {
            "Android project: instrumented tests require run.android_opt_in = true in ~/.tester/tester.toml"
        }
        Msg::NoteVenv => ".venv found",
        Msg::NoteLaravel => "Laravel detected",

        Msg::ErrContextTooLarge => "context for '{symbol}' exceeds max_context_chars ({max})",
        Msg::ErrParseDraftFromLlm => "could not parse test file from model response",
        Msg::ErrEmptyValidateCommand => "empty validation command",
        Msg::ErrUiBaseUnavailable => "UI base_url {base} unavailable and tests.ui.start is not set",
        Msg::ErrUiStartEarly => "UI start command exited early with {status}: {start}",
        Msg::ErrWaitUrlTimeout => "timeout waiting for {url}",
        Msg::ErrValidateTimeout => "validation exceeded timeout {secs}s",
        Msg::ErrBugInputRequired => "need --bug, --bug-file, or a traceback on stdin",
        Msg::ErrNoRepairState => "no saved repair state in {path}",
        Msg::ErrNoMutationTarget => "no covered pure symbol for mutation testing",
        Msg::ErrNoMutationApplied => "could not apply any mutation",
        Msg::ErrNoFixturesDir => "no fixtures directory: {path}",
        Msg::ErrNoAdapterForLang => "no adapter for {lang}",
        Msg::ErrProjectManifestNotFound => {
            "project manifest not found under {path}"
        }
        Msg::ErrProjectManifestAmbiguous => {
            "several projects under {path}: {candidates} - pass an explicit path"
        }
        Msg::ErrPatchFragmentNotFound => "old fragment not found in {path}",
        Msg::ErrToolArgRequired => "argument '{key}' required (string)",
        Msg::ErrPlanSymbolNotFound => "symbol '{symbol}' not found in plan",
        Msg::ErrNoPlanItemForSymbol => "no plan item for {symbol}",
        Msg::ErrNoLastRepairJson => {
            "no .tester/last_repair.json - run run_tests with a failure first"
        }
        Msg::ErrEmptyAdapterArgv => "empty adapter argv",
        Msg::ErrDockerDisabled => "docker disabled: run.allow_docker=false",
        Msg::ErrDockerUnavailable => "docker unavailable: {detail}",
        Msg::ErrLlmAccessDenied => "llm HTTP {status}: access denied: {detail}",
        Msg::ErrLlmHttp => "llm HTTP {status}: {detail}",
        Msg::ErrLlmHttpStatus => "llm HTTP {status}",
        Msg::ErrLlmEmptyResponse => "empty llm response",
        Msg::ErrEmbeddingsHttp => "embeddings HTTP {status}: {detail}",
        Msg::ErrEmbeddingsNoData => "embeddings: no data",
        Msg::ErrEmbeddingsNoEmbedding => "embeddings: no embedding",
        Msg::ErrEmbeddingsDisabled => "embeddings disabled (set [analysis] embeddings = true)",
        Msg::ErrEmbeddingsModelRequired => "need embeddings_model or select a chat model",
        Msg::ErrSemanticSearchNeedsEmbeddings => {
            "semantic_search: enable [analysis] embeddings = true and build the index"
        }
        Msg::ErrFileNotFound => "file not found: {path}",
        Msg::OutlineNoSymbols => "(no symbols)",
        Msg::SessionIdEmpty => "empty session id",
        Msg::SessionIdNoMatch => "no session matching '{prefix}'",
        Msg::SessionIdAmbiguous => "ambiguous session id '{prefix}' ({n} matches)",
        Msg::ErrNoModels => "no models at endpoint (GET /models returned empty)",
        Msg::ErrModelNeedsSelection => {
            "model not set: {count} models available - pick with /model or --model"
        }
        Msg::SlashModel => "select LLM model (from /models)",
        Msg::SlashWiki => "wiki sync/analyze/run from URL or path",
        Msg::ModelOverlayTitle => " models  Enter select  r refresh  Esc ",
        Msg::ModelOverlayEmpty => "(no models)",
        Msg::ModelSelected => "model: {model}",
        Msg::ModelSelectHint => "model not set - /model to choose (or --model)",
        Msg::ModelPersisted => "saved model={model} to {path}",
        Msg::ModelNotSetBadge => "model?",
        Msg::ModelBusyReject => "cannot switch model while a job is running",
        Msg::WikiHelpUsage => {
            "/wiki sync <url|path> | /wiki analyze | /wiki run <url|path> | /wiki status"
        }
        Msg::WikiStatusLine => {
            "wiki status: kind={kind} pages={pages} scenarios={scenarios} url={url} path={path}"
        }
        Msg::WikiNoCache => "wiki: no cache - /wiki sync <url|path>",
        Msg::WikiStatusError => "wiki status error: {err}",
        Msg::WikiNeedsArg => "/wiki {sub} needs <url|path>",
        Msg::WikiAlreadyRunning => "wiki: already running",
        Msg::WikiWorking => "wiki {sub}...",
        Msg::WikiOpOk => "wiki {sub} ok: kind={kind} pages={pages}{extra}",
        Msg::WikiOpFailed => "wiki {sub} failed: {err}",
        Msg::WikiAnalyzeWorking => "wiki analyze...",
        Msg::WikiAnalyzeOkTui => {
            "wiki analyze ok: versions={versions} scenarios={scenarios} hubs={hubs} edges={edges}"
        }
        Msg::WikiAnalyzeFailed => "wiki analyze failed: {err}",
        Msg::WikiUnknownSub => "/wiki {sub}: use sync|analyze|run|status",
        Msg::WikiExtraPlanItems => "; plan_items={n}",
        Msg::WikiExtraAnalyzeError => "; analyze error: {err}",
        Msg::WikiExtraScenarios => "; scenarios={n}",
        Msg::WikiSyncOk => "wiki sync ok: kind={kind} pages={pages}",
        Msg::WikiSyncEmptySkipped => "wiki sync: skipped {n} empty page(s)",
        Msg::WikiSourceLine => "source: {src}",
        Msg::WikiCacheLine => "cache: {path}",
        Msg::NoteWikiHeuristicAnalyze => {
            "wiki analyze: no LLM - heuristic scenarios only"
        }
        Msg::WarnWikiLlmPartial => "wiki LLM analyze partial: {err}",
        Msg::WikiAnalyzeOk => {
            "wiki analyze ok: versions={versions} scenarios={scenarios} entities={entities} hubs={hubs} toc={toc} edges={edges}"
        }
        Msg::WikiScenarioLine => "  - [{version}] {title} ({steps} steps)",
        Msg::WikiPlanOk => "wiki plan: {n} item(s)",
        Msg::WikiPlanItem => "  - {name} -> {path}",
        Msg::WikiPlanWarning => "wiki plan",
        Msg::WikiRunGenerating => "wiki run: generating {n} UI draft(s)",
        Msg::WikiWrote => "  wrote {path}",
        Msg::WikiPlaywrightOk => "  playwright ok: {path}",
        Msg::WikiPlaywrightFail => "  playwright FAIL: {path} ({cmd})",
        Msg::WikiPlaywrightError => "  playwright error: {err}",
        Msg::WikiDraftPreview => "  draft {path} ({bytes} bytes) - pass --write to save",
        Msg::WikiRunDone => {
            "wiki run done: items={items} written={written} playwright_ok={ok} playwright_fail={fail}"
        }
        Msg::ErrWikiNoScenarios => "no wiki UI scenarios - run `tester wiki analyze` first",
        Msg::ErrWikiRunNoItems => "wiki run: no UI plan items after analyze",
        Msg::ErrWikiRunNeedsWrite => "--run requires --write",
        Msg::ErrWikiCacheEmpty => "wiki cache empty - run sync first (--url or --path)",
        Msg::ErrWikiNeedUrlOrPath => {
            "provide --url or --path (nothing stored in .tester/wiki/source.json yet)"
        }
        Msg::ErrWikiHtmlNoHost => "url has no host",
        Msg::ErrWikiHtmlZeroPages => "HTML crawl downloaded 0 pages from {url}",
        Msg::ErrWikiMwZeroPages => "MediaWiki returned no pages from {api}",
        Msg::ErrWikiMwAllpagesHttp => "allpages HTTP {status}",
        Msg::ErrWikiMwExtractsHttp => "extracts HTTP {status}",
        Msg::ErrWikiMdNotDir => "markdown path is not a directory: {path}",
        Msg::ErrWikiMdZeroPages => "no markdown pages found under {path}",
        Msg::ErrWikiLlmNoJson => "no JSON array in llm response",
        Msg::WikiScenarioReason => "wiki scenario: {title}",
        Msg::WikiPreconditionsHeader => "Preconditions:",
        Msg::WikiScenarioLabel => "Wiki scenario: {title}",
        Msg::WikiCoverageLabel => "Coverage: {coverage}",
        Msg::WikiSourcesLabel => "Sources: {sources}",
        Msg::ToolWikiSync => {
            "Download wiki into .tester/wiki (MediaWiki/HTML via url, or local markdown path)"
        }
        Msg::ToolWikiSearch => "Search cached wiki pages by substring (title/body)",
        Msg::ToolWikiScenarioList => {
            "List UI scenarios from .tester/wiki/scenarios.json (after wiki_sync + analyze)"
        }
        Msg::ToolShowFile => {
            "Show a file: source outline for code, or extracted/OCR text for PDF/office/images (cached under .tester/docs)"
        }
        Msg::ToolConvertDocument => {
            "Convert a document to txt, md, or pdf (LibreOffice/pandoc); writes under .tester/docs/converted by default"
        }
        Msg::ErrDocsMissingBin => "document tool needs `{bin}` on PATH (or set [documents] path in tester.toml)",
        Msg::ErrDocsBinFailed => "document tool `{bin}` failed: {err}",
        Msg::ErrDocsNotFound => "document not found: {path}",
        Msg::ErrDocsExtract => "failed to extract text from {path}: {err}",
        Msg::ErrDocsOcr => "OCR failed for {path}: {err}",
        Msg::ErrDocsConvert => "convert failed for {path}: {err}",
        Msg::ErrDocsUnsupportedConvert => "cannot convert {kind} to {to}",
        Msg::ErrDocsUnsupportedKind => "unsupported document kind for {path}",
        Msg::DocsShowOk => "show {path} kind={kind} source={source} engine={engine} cache_hit={hit} chars={chars}",
        Msg::DocsConvertOk => "converted {path} -> {out} via {engine}",
    }
}
