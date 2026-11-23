use super::Msg;

pub(crate) fn t(msg: Msg) -> &'static str {
    match msg {
        Msg::ErrorPrefix => "ошибка: ",
        Msg::CancelHint => "\n^C - отмена после текущего шага...",
        Msg::NoGoals => "нет целей",
        Msg::WarningPrefix => "предупреждение: ",
        Msg::Language => "язык",
        Msg::Package => "пакет",
        Msg::Supported => "поддерживается",
        Msg::TestCommand => "команда_тестов",
        Msg::TestStyle => "стиль_тестов",
        Msg::HasTests => "есть_тесты",
        Msg::UnitFramework => "фреймворк_юнитов",
        Msg::UiFramework => "фреймворк_ui",
        Msg::UiReady => "ui_готов",
        Msg::Capabilities => "возможности",
        Msg::Note => "заметка",
        Msg::PlanArrow => "->",
        Msg::LlmUnavailable => "llm недоступна",
        Msg::UnsupportedLanguage => "язык распознан, но пока не поддерживается",
        Msg::PlaywrightInit => "Playwright инициализирован в",
        Msg::ConfigLabel => "конфиг",
        Msg::TestsLabel => "тесты",

        Msg::TuiNeedsTty => {
            "tester требует интерактивный терминал; используйте `tester chat <PATH>` / `--plain`, или запустите в TTY"
        }
        Msg::TitleCommands => " /команды↑↓TabEnter ",
        Msg::TitleOpenProject => " открыть проект ",
        Msg::TitleFiles => " @файлы ",
        Msg::TitleWorktrees => " worktrees ",
        Msg::PopupNoMatches => "нет совпадений",
        Msg::AttachHint => "прикрепить файл или символ",
        Msg::SlashHelp => "команды и клавиши",
        Msg::SlashScan => "язык проекта и тесты",
        Msg::SlashPlan => "план тестов (unit|e2e|...)",
        Msg::SlashRun => "запуск агента с промптом",
        Msg::SlashClear => "очистить сессию чата",
        Msg::SlashAudit => "сводка возможностей",
        Msg::SlashExit => "выйти из сессии",
        Msg::SlashGrep => "поиск regex по репо",
        Msg::SlashSymbol => "определение символа",
        Msg::SlashRefs => "ссылки на символ",
        Msg::SlashSearch => "семантический поиск",
        Msg::SlashIndex => "статус / rebuild индекса",
        Msg::SlashMap => "карта структуры проекта",
        Msg::SlashListFiles => "список файлов проекта",
        Msg::SlashShowFile => "outline исходника или текст PDF/office/изображения",
        Msg::SlashConvert => "конвертировать документ (txt|md|pdf)",
        Msg::SlashManual => "открыть manual",
        Msg::SlashRecent => "недавние проекты",
        Msg::SlashSession => "список / смена / новая сессия чата",
        Msg::SlashPrune => "удалить сохранённые сессии",
        Msg::SlashDiff => "просмотр git diff",
        Msg::SlashBranch => "переключение ветки",
        Msg::SlashProposePlan => "превью плана перед run",
        Msg::SlashSecurity => "скан безопасности",
        Msg::SecurityOverlayTitle => "security",
        Msg::PickerStatusProject => "корень проекта - Ctrl+Enter / Ctrl+S открыть",
        Msg::PickerStatusNav => {
            "↑↓ навигация  Enter папка  -> worktrees  Ctrl+Enter выбрать  Esc отмена"
        }
        Msg::PickerStatusWorktrees => {
            "worktrees - Enter/Ctrl+Enter открыть  <- к папкам  Esc отмена"
        }
        Msg::PickerFilter => "фильтр",
        Msg::PickerProjectTag => "[проект]",
        Msg::PickerWorktreeTag => "[worktree]",
        Msg::SessionFailedOpen => "не удалось открыть проект",
        Msg::SessionFixHint => "исправьте проект / конфиг, затем Ctrl+O",
        Msg::SessionStatusDryRun => "dry-run  Enter отправить  @файл  /help",
        Msg::SessionStatusAuto => "auto-approve  Enter отправить  @файл  /help",
        Msg::SessionStatusBlocked => "write/shell требуют y/N  Enter отправить  @файл  /help",
        Msg::SessionCleared => "сессия очищена",
        Msg::SessionSwitched => "сессия готова",
        Msg::SessionRunning => "выполняется... Esc отмена",
        Msg::SessionCancelReq => "запрошена отмена...",
        Msg::SessionHelp => {
            "\
Tester - справка

Клавиши
  Enter            отправить сообщение
  Shift+Enter / Ctrl+Enter / Ctrl+J   новая строка в вводе
  Esc Esc          отмена / закрыть overlay
  Ctrl+C Ctrl+C    выход
  Ctrl+O           открыть проект
  Ctrl+R           поиск по истории ввода
  Ctrl+X Ctrl+E    внешний редактор
  ↑↓ / PgUp PgDn   прокрутка диалога
  y / Enter        разрешить tool
  n / Esc          отклонить tool

Редактирование ввода
  Ctrl+A / E       начало / конец строки
  Ctrl+Home / End  начало / конец текста
  Ctrl+W / U / K   удалить слово / до начала / до конца
  Ctrl+Y           вставить удалённое
  Ctrl+P / N       история ввода
  Ctrl+<- / ->       по словам
  Ctrl+V           вставить из буфера
  Shift+Insert     вставить
  мышь             обычное выделение / копирование терминала
  колесо           прокрутка чата (не поле ввода)
  ↑↓ / PgUp PgDn   прокрутка чата

Вложения
  @path            прикрепить файл или папку
  @path#L10-40     диапазон строк
  @Symbol          переход к символу

Команды"
        }
        Msg::SessionUnknownCmd => "неизвестная команда; /help",
        Msg::SessionUsageRun => "usage: /run <prompt>",
        Msg::SessionNoGoals => "нет целей в плане",
        Msg::SessionBadKind => "неверный kind",
        Msg::SessionEmptyResponse => "(пустой ответ)",
        Msg::SessionError => "ошибка - смотрите диалог",
        Msg::SessionThreadExit => "поток агента завершился",
        Msg::SupportedWord => "поддерживается",
        Msg::UnsupportedWord => "не поддерживается",
        Msg::PermTitle => " разрешить tool? ",
        Msg::PermPrompt => "нужно разрешение - y/n",
        Msg::ComposerPlaceholder => "Спросите tester...  (@файл  /команды)",
        Msg::EmptyTranscript => "Напишите промпт или /help",
        Msg::EmptyWorktrees => "нет git worktrees",
        Msg::DoctorTitle => "# tester doctor\n\n",
        Msg::DoctorOk => "ok",
        Msg::DoctorFail => "FAIL",
        Msg::ReportTitle => "# Отчёт tester\n\n",
        Msg::ReportProject => "проект",
        Msg::ReportLanguage => "язык",
        Msg::ReportPackage => "пакет",
        Msg::ReportWarnings => "## Предупреждения\n\n",
        Msg::ReportFailures => "## Сводка failures\n\n",
        Msg::ReportResults => {
            "## Результаты\n\n| символ | вид | файл | попытки | статус | first_pass | density | полезность | команда |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n"
        }
        Msg::ReportTrace => "Трассировка",
        Msg::ReportStatusPlanned => "запланирован",
        Msg::ReportStatusDryRun => "сухой_прогон",
        Msg::ReportStatusWritten => "записан",
        Msg::ReportStatusPassed => "пройден",
        Msg::ReportStatusUnfixed => "не_исправлен",
        Msg::ReportStatusSkipped => "пропущен",
        Msg::ReportStatusFailedPath => "отказ_пути",

        Msg::BadgeThinking => "Думает",
        Msg::BadgeWorking => "Работает",
        Msg::BadgePerm => "доступ",
        Msg::BadgePlan => "ПЛАН",
        Msg::StatusIdle => "ожидание",
        Msg::CtxGauge => "ctx {used}/{limit}",
        Msg::PermWaitYn => "ждём y/N",
        Msg::StatusHelpHint => " /help",
        Msg::StatusOpenEditor => "Ctrl+E - открыть $EDITOR",
        Msg::TuiDryRunBadge => "dry-run",
        Msg::TuiNoProject => "нет проекта",
        Msg::WinTitleBusy => "testerзанят{model}",
        Msg::WinTitleIdle => "testerожидание",
        Msg::WinTitlePicker => "testerоткрыть проект",
        Msg::PermAllow => "разрешить",
        Msg::PermDeny => "отклонить",
        Msg::ComposerRevSearch => "поиск-назад: ",

        Msg::SessionOpenFailed => "не удалось открыть сессию: {err}",
        Msg::AuditFailed => "аудит не удался: {err}",
        Msg::ScanFailed => "scan не удался: {err}",
        Msg::ScanSummary => "язык={lang} пакет={pkg} cmd={cmd}",
        Msg::PlanFailed => "план не удался: {err}",
        Msg::GitCheckoutOk => "переключено на {branch}",
        Msg::GitCheckoutFailed => "checkout не удался: {err}",
        Msg::GitCheckoutError => "ошибка checkout: {err}",
        Msg::ProposePlanApproved => "/propose_plan (одобрено)",
        Msg::ProposePlanTitle => "план ({kind})",
        Msg::ProposePlanPrompt => "Выполни этот план тестов:\n{body}\nОтвечай с прогрессом.",
        Msg::UsageGrep => "usage: /grep <pattern>",
        Msg::UsageSymbol => "usage: /symbol <name>",
        Msg::UsageRefs => "usage: /refs <name>",
        Msg::UsageSearch => "usage: /search <запрос>",
        Msg::UsageShowFile => "usage: /show_file <path>",
        Msg::UsageConvert => "usage: /convert <path> [txt|md|pdf]",
        Msg::GrepFailed => "grep не удался: {err}",
        Msg::ToolMissing => "tool {name} отсутствует",
        Msg::ToolFailed => "{name} не удался: {err}",
        Msg::IndexRebuilt => "индекс пересобран: {n} символов",
        Msg::IndexRebuildFailed => "пересборка индекса не удалась: {err}",
        Msg::IndexFailed => "индекс не удался: {err}",
        Msg::SessionCmdError => "/session: {err}",
        Msg::PruneUsage => "usage: /prune <id|-w|-o <days>|all>",
        Msg::PruneDone => "удалено сессий: {n}",
        Msg::PruneError => "/prune: {err}",
        Msg::SessionSwitchFailed => "смена сессии не удалась: {err}",
        Msg::SessionOutcomeOk => "ok",
        Msg::SessionOutcomeCancelled => "отменено",
        Msg::SessionOutcomeLine => "ходы={turns} tools={tools}{status}",
        Msg::EditorFailed => "редактор: {err}",

        Msg::ManualContent => {
            "\
# Справка Tester

## Навигация
- Enter - отправить промпт
- @path / @Symbol - вложение
- /команды - палитра slash
- Esc Esc - отмена / закрыть overlay
- Ctrl+C Ctrl+C - выход
- Ctrl+R - поиск по истории
- Ctrl+X Ctrl+E - внешний редактор

## Slash
/scan /plan /run /grep /symbol /refs /search /index /map /show_file /list_files
/manual /diff /branch /recent /session /prune /propose_plan /audit /clear /exit

## Analysis tools
find_symbol, project_map, graph_callers, semantic_search (opt-in embeddings)

## Approvals
y/N для write/shellapprove/reject плана через /propose_plan
"
        }
        Msg::ManualTocTitle => " оглавление ",
        Msg::ManualTitleSearching => " /manual  Alt+S поиск: {q}_ ",
        Msg::ManualTitleIdle => " /manual  Alt+S поискEsc закрыть ",
        Msg::ManualSearchNoMatches => "(нет совпадений)",
        Msg::PlanOverlayFooter => " a одобритьr отклонитьe правитьEsc отмена ",
        Msg::PlanOverlayTitle => " propose_plan ",
        Msg::DiffUnavailable => "(git diff недоступен)",
        Msg::DiffClean => "(чистое дерево)",
        Msg::DiffTitle => "diff",
        Msg::DiffOverlayChrome => " {title}Esc закрытьPgUp/Dn ",
        Msg::BranchNone => "(нет git веток)",
        Msg::BranchItemTag => "ветка",
        Msg::BranchOverlayTitle => " веткиEnter checkoutEsc ",
        Msg::SessionsNone => "(нет сессий)",
        Msg::SessionsNewHint => "n = новая",
        Msg::SessionTitleEmpty => "(пусто)",
        Msg::SessionsOverlayTitle => " сессииEnter открытьn новаяEsc ",
        Msg::RecentHasTester => "есть .tester",
        Msg::RecentProject => "проект",
        Msg::RecentOverlayTitle => " недавние проектыEnterEsc ",

        Msg::PickerOkStatus => "ok: {path} ({word})",
        Msg::PickerWarnStatus => "warn: {err}",
        Msg::ReplBanner => "tester chat - REPL агента. /help для команд, Ctrl+C выход.",
        Msg::ReplPrompt => "tester> ",
        Msg::ReplSlashHelp => {
            "\
Команды:
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
        Msg::AuditEmpty => "аудит: пусто",
        Msg::AuditHeader => "аудит: {n} действий",

        Msg::PlanProgress => "| plan -> generate -> validate -> repair ({n})",
        Msg::PlanItemNotFound => "элемент плана '{symbol}' не найден для repair",
        Msg::PlainNeedsPath => "`--plain` требует PATH",
        Msg::PruneNeedsArg => "укажите id | --all | --workspace | --older-than",
        Msg::PruneCli => "удалено: {n}",
        Msg::SecurityPatchNote => "заметка: {note}",
        Msg::SecurityApplied => "применено: {path}",
        Msg::SecurityDryRunHint => "dry-run (добавьте --apply для записи)",
        Msg::FindingNotFound => "находка не найдена: {id}",
        Msg::UsageLine => "расход: prompt={prompt} completion={completion} total={total}",
        Msg::ReplTurnsLine => "- ходы={turns} tools={tools} tokens≈{tokens} {status}",
        Msg::ItemWritten => "записан",
        Msg::ItemPassed => "пройден",
        Msg::PlanShowEmpty => "нет целей в .tester/plan.json",
        Msg::LlmClientCreate => "создание клиента llm",
        Msg::ErrPathEscapes => "путь выходит за корень проекта: {path}",
        Msg::ErrPathForbidden => "путь запрещён: {detail}",
        Msg::ErrPathNeedsUpdate => "отказ перезаписать существующий файл без --update",
        Msg::ErrManifestWrite => "отказ писать манифест {name}",
        Msg::ErrGateNoAssertion => "gate: нет утверждения (assert/expect/REQUIRE)",
        Msg::ErrGateWeakTest => "gate: слабый тест (только snapshot/sleep без проверки поведения)",
        Msg::ErrGateForbiddenNetwork => "gate: запрещённая сеть/Docker во внешнем сервисе",
        Msg::ErrGateUnitNetwork => "gate: unit-тест тянет сеть/БД - используйте integration",
        Msg::DoctorNoAdapter => "нет адаптера",
        Msg::DoctorNotRequired => "не требуется",
        Msg::DoctorGradlewFound => "gradlew найден; connectedAndroidTest потребует device/emulator",
        Msg::DoctorGradlewMissing => "gradlew не найден",
        Msg::DoctorUiState => "framework={framework} ready={ready}",
        Msg::DoctorSecurityRules => "правил: {n}, enabled={enabled}, deep={deep}",
        Msg::SecReportTitle => "# Отчёт безопасности",
        Msg::SecCounts => "## Количество",
        Msg::SecNotes => "## Заметки",
        Msg::SecFindings => "## Находки",
        Msg::SecNoFindings => "_Нет находок выше заданных уровней серьёзности._",
        Msg::SecScanSummary => "проверка безопасности: находок {n} ({pack} {version})",
        Msg::SecScanFailed => "проверка безопасности не удалась: {err}",
        Msg::SecFixLine => "Исправление ({strategy}) : {summary}",
        Msg::SecNoteLine => "заметка: {note}",
        Msg::PermPromptCli => "Разрешить {tool}? {preview} [y/N] ",
        Msg::IoRead => "чтение {path}",
        Msg::IoWrite => "запись {path}",
        Msg::IoCreate => "создание {path}",
        Msg::IoParse => "разбор {path}",
        Msg::AgentDeadline => "агент превысил deadline {secs}s",
        Msg::MaxToolCalls => "превышен max_tool_calls ({n})",
        Msg::UnknownTool => "неизвестный tool: {name}",
        Msg::AgentTurnLimit => "(лимит ходов)",
        Msg::AgentStepModel => "ход {turn}: вызов модели",
        Msg::AgentStepDone => "готово",
        Msg::AgentStepTool => "tool {name}",
        Msg::SecMetaLine => {
            "- корень: `{root}`\n- rule_pack: {pack}\n- deep: {deep}\n- находок: {n}\n"
        }
        Msg::SecFindingMeta => {
            "- файл: `{path}`:{line}\n- категория: {category}\n- уверенность: {confidence}\n- свидетельство: {evidence}\n"
        }
        Msg::WarnFlutterIntegration => {
            "пакет integration_test отсутствует; цели Flutter integration пропускаются"
        }
        Msg::WarnUiSkipped => "пропуск UI-цели '{symbol}': UI-раннер не готов",
        Msg::WarnPlaywrightMissing => {
            "Playwright не установлен; перед UI-тестами выполните `tester ui init`"
        }
        Msg::NoteDockerAllowed => {
            "allow_docker=true: tool docker_info доступен агенту; внешние сервисы по-прежнему не поднимаются автоматически"
        }
        Msg::NoteApiSurface => {
            "обнаружена API-поверхность (OpenAPI/proto/router) - contract kind доступен"
        }
        Msg::NoteCppKinds => {
            "C++: contract/UI kinds не поддерживаются; unit+integration через gtest"
        }
        Msg::NoteBlankWorkspace => {
            "blank workspace: нет манифеста языка - только wiki/docs -> Playwright UI"
        }
        Msg::NoteBlankNeedUiInit => {
            "Playwright ещё не scaffold - выполните `tester ui init` или `wiki run --write`"
        }
        Msg::WarnBlankUiScaffold => "blank workspace: scaffolding Playwright перед записью",
        Msg::NoteAndroidOptIn => {
            "Android-проект: instrumented-тесты требуют run.android_opt_in = true в ~/.tester/tester.toml"
        }
        Msg::NoteVenv => "найден .venv",
        Msg::NoteLaravel => "обнаружен Laravel",

        Msg::ErrContextTooLarge => "контекст для '{symbol}' превышает max_context_chars ({max})",
        Msg::ErrParseDraftFromLlm => "не удалось разобрать тестовый файл из ответа модели",
        Msg::ErrEmptyValidateCommand => "пустая команда проверки",
        Msg::ErrUiBaseUnavailable => "UI base_url {base} недоступен, а tests.ui.start не задан",
        Msg::ErrUiStartEarly => "команда старта UI завершилась рано с {status}: {start}",
        Msg::ErrWaitUrlTimeout => "таймаут ожидания {url}",
        Msg::ErrValidateTimeout => "проверка превысила таймаут {secs}с",
        Msg::ErrBugInputRequired => "нужен --bug, --bug-file или traceback на stdin",
        Msg::ErrNoRepairState => "нет сохранённого состояния repair в {path}",
        Msg::ErrNoMutationTarget => "нет покрытого чистого символа для мутации",
        Msg::ErrNoMutationApplied => "не удалось применить ни одну мутацию",
        Msg::ErrNoFixturesDir => "нет каталога fixtures: {path}",
        Msg::ErrNoAdapterForLang => "нет адаптера для {lang}",
        Msg::ErrProjectManifestNotFound => {
            "манифест проекта не найден в {path}"
        }
        Msg::ErrProjectManifestAmbiguous => {
            "несколько проектов в {path}: {candidates} - укажите путь явно"
        }
        Msg::ErrPatchFragmentNotFound => "фрагмент old не найден в {path}",
        Msg::ErrToolArgRequired => "нужен аргумент '{key}' (string)",
        Msg::ErrPlanSymbolNotFound => "символ '{symbol}' не найден в плане",
        Msg::ErrNoPlanItemForSymbol => "нет plan item для {symbol}",
        Msg::ErrNoLastRepairJson => "нет .tester/last_repair.json - сначала run_tests с падением",
        Msg::ErrEmptyAdapterArgv => "пустой argv адаптера",
        Msg::ErrDockerDisabled => "docker отключён: run.allow_docker=false",
        Msg::ErrDockerUnavailable => "docker недоступен: {detail}",
        Msg::ErrLlmAccessDenied => "llm HTTP {status}: доступ запрещён: {detail}",
        Msg::ErrLlmHttp => "llm HTTP {status}: {detail}",
        Msg::ErrLlmHttpStatus => "llm HTTP {status}",
        Msg::ErrLlmEmptyResponse => "пустой ответ llm",
        Msg::ErrEmbeddingsHttp => "embeddings HTTP {status}: {detail}",
        Msg::ErrEmbeddingsNoData => "embeddings: нет data",
        Msg::ErrEmbeddingsNoEmbedding => "embeddings: нет embedding",
        Msg::ErrEmbeddingsDisabled => "embeddings выключены ([analysis] embeddings = true)",
        Msg::ErrEmbeddingsModelRequired => "нужен embeddings_model или выбранная chat-модель",
        Msg::ErrSemanticSearchNeedsEmbeddings => {
            "semantic_search: включите [analysis] embeddings = true и постройте индекс"
        }
        Msg::ErrFileNotFound => "файл не найден: {path}",
        Msg::OutlineNoSymbols => "(нет символов)",
        Msg::SessionIdEmpty => "пустой id сессии",
        Msg::SessionIdNoMatch => "нет сессии по префиксу '{prefix}'",
        Msg::SessionIdAmbiguous => "неоднозначный id сессии '{prefix}' ({n} совпадений)",
        Msg::ErrNoModels => "на endpoint нет моделей (GET /models пустой)",
        Msg::ErrModelNeedsSelection => {
            "model не задан: доступно {count} - выберите /model или --model"
        }
        Msg::SlashModel => "выбрать LLM model (из /models)",
        Msg::SlashWiki => "wiki sync/analyze/run по URL или пути",
        Msg::ModelOverlayTitle => " модели  Enter выбрать  r обновить  Esc ",
        Msg::ModelOverlayEmpty => "(нет моделей)",
        Msg::ModelSelected => "model: {model}",
        Msg::ModelSelectHint => "model не задан - /model для выбора (или --model)",
        Msg::ModelPersisted => "сохранено model={model} в {path}",
        Msg::ModelNotSetBadge => "model?",
        Msg::ModelBusyReject => "нельзя сменить model пока идёт задача",
        Msg::WikiHelpUsage => {
            "/wiki sync <url|path> | /wiki analyze | /wiki run <url|path> | /wiki status"
        }
        Msg::WikiStatusLine => {
            "wiki статус: kind={kind} pages={pages} scenarios={scenarios} url={url} path={path}"
        }
        Msg::WikiNoCache => "wiki: нет кэша - /wiki sync <url|path>",
        Msg::WikiStatusError => "wiki status ошибка: {err}",
        Msg::WikiNeedsArg => "/wiki {sub} нужен <url|path>",
        Msg::WikiAlreadyRunning => "wiki: уже выполняется",
        Msg::WikiWorking => "wiki {sub}...",
        Msg::WikiOpOk => "wiki {sub} ок: kind={kind} pages={pages}{extra}",
        Msg::WikiOpFailed => "wiki {sub} ошибка: {err}",
        Msg::WikiAnalyzeWorking => "wiki analyze...",
        Msg::WikiAnalyzeOkTui => {
            "wiki analyze ок: versions={versions} scenarios={scenarios} hubs={hubs} edges={edges}"
        }
        Msg::WikiAnalyzeFailed => "wiki analyze ошибка: {err}",
        Msg::WikiUnknownSub => "/wiki {sub}: используйте sync|analyze|run|status",
        Msg::WikiExtraPlanItems => "; plan_items={n}",
        Msg::WikiExtraAnalyzeError => "; analyze ошибка: {err}",
        Msg::WikiExtraScenarios => "; scenarios={n}",
        Msg::WikiSyncOk => "wiki sync ок: kind={kind} pages={pages}",
        Msg::WikiSyncEmptySkipped => "wiki sync: пропущено пустых страниц: {n}",
        Msg::WikiSourceLine => "источник: {src}",
        Msg::WikiCacheLine => "кэш: {path}",
        Msg::NoteWikiHeuristicAnalyze => {
            "wiki analyze: без LLM - только эвристические сценарии"
        }
        Msg::WarnWikiLlmPartial => "wiki LLM analyze частично: {err}",
        Msg::WikiAnalyzeOk => {
            "wiki analyze ок: versions={versions} scenarios={scenarios} entities={entities} hubs={hubs} toc={toc} edges={edges}"
        }
        Msg::WikiScenarioLine => "  - [{version}] {title} ({steps} шагов)",
        Msg::WikiPlanOk => "wiki plan: {n} элемент(ов)",
        Msg::WikiPlanItem => "  - {name} -> {path}",
        Msg::WikiPlanWarning => "wiki plan",
        Msg::WikiRunGenerating => "wiki run: генерация {n} UI draft(ов)",
        Msg::WikiWrote => "  записано {path}",
        Msg::WikiPlaywrightOk => "  playwright ок: {path}",
        Msg::WikiPlaywrightFail => "  playwright FAIL: {path} ({cmd})",
        Msg::WikiPlaywrightError => "  playwright ошибка: {err}",
        Msg::WikiDraftPreview => "  draft {path} ({bytes} байт) - передайте --write чтобы сохранить",
        Msg::WikiRunDone => {
            "wiki run готово: items={items} written={written} playwright_ok={ok} playwright_fail={fail}"
        }
        Msg::ErrWikiNoScenarios => {
            "нет UI-сценариев wiki - сначала `tester wiki analyze`"
        }
        Msg::ErrWikiRunNoItems => "wiki run: нет UI plan items после analyze",
        Msg::ErrWikiRunNeedsWrite => "--run требует --write",
        Msg::ErrWikiCacheEmpty => "кэш wiki пуст - сначала sync (--url или --path)",
        Msg::ErrWikiNeedUrlOrPath => {
            "укажите --url или --path (в .tester/wiki/source.json ещё ничего нет)"
        }
        Msg::ErrWikiHtmlNoHost => "у url нет host",
        Msg::ErrWikiHtmlZeroPages => "HTML crawl скачал 0 страниц с {url}",
        Msg::ErrWikiMwZeroPages => "MediaWiki не вернул страниц с {api}",
        Msg::ErrWikiMwAllpagesHttp => "allpages HTTP {status}",
        Msg::ErrWikiMwExtractsHttp => "extracts HTTP {status}",
        Msg::ErrWikiMdNotDir => "путь markdown не директория: {path}",
        Msg::ErrWikiMdZeroPages => "markdown-страниц не найдено в {path}",
        Msg::ErrWikiLlmNoJson => "в ответе llm нет JSON-массива",
        Msg::WikiScenarioReason => "wiki сценарий: {title}",
        Msg::WikiPreconditionsHeader => "Предусловия:",
        Msg::WikiScenarioLabel => "Wiki сценарий: {title}",
        Msg::WikiCoverageLabel => "Покрытие: {coverage}",
        Msg::WikiSourcesLabel => "Источники: {sources}",
        Msg::ToolWikiSync => {
            "Скачать wiki в .tester/wiki (MediaWiki/HTML по url или локальный markdown)"
        }
        Msg::ToolWikiSearch => "Поиск по кэшу wiki (title/body)",
        Msg::ToolWikiScenarioList => {
            "Список UI-сценариев из .tester/wiki/scenarios.json (после wiki_sync + analyze)"
        }
        Msg::ToolShowFile => {
            "Показать файл: outline для кода или извлечённый/OCR текст для PDF/office/изображений (кэш .tester/docs)"
        }
        Msg::ToolConvertDocument => {
            "Конвертировать документ в txt, md или pdf (LibreOffice/pandoc); по умолчанию в .tester/docs/converted"
        }
        Msg::ErrDocsMissingBin => "нужен `{bin}` в PATH (или путь в [documents] в tester.toml)",
        Msg::ErrDocsBinFailed => "инструмент `{bin}` ошибся: {err}",
        Msg::ErrDocsNotFound => "документ не найден: {path}",
        Msg::ErrDocsExtract => "не удалось извлечь текст из {path}: {err}",
        Msg::ErrDocsOcr => "OCR не удался для {path}: {err}",
        Msg::ErrDocsConvert => "конвертация не удалась для {path}: {err}",
        Msg::ErrDocsUnsupportedConvert => "нельзя конвертировать {kind} в {to}",
        Msg::ErrDocsUnsupportedKind => "неподдерживаемый тип документа: {path}",
        Msg::DocsShowOk => "show {path} kind={kind} source={source} engine={engine} cache_hit={hit} chars={chars}",
        Msg::DocsConvertOk => "сконвертировано {path} -> {out} через {engine}",
    }
}
