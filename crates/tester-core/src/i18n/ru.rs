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
        Msg::SlashShowFile => "outline файла",
        Msg::SlashManual => "открыть manual",
        Msg::SlashRecent => "недавние проекты",
        Msg::SlashSession => "список / смена / новая сессия чата",
        Msg::SlashPrune => "удалить сохранённые сессии",
        Msg::SlashDiff => "просмотр git diff",
        Msg::SlashBranch => "переключение ветки",
        Msg::SlashProposePlan => "превью плана перед run",
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
    }
}
