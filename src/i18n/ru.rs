use super::Msg;

pub fn t(msg: Msg) -> &'static str {
    match msg {
        Msg::CliAbout => "LLM-агент для генерации и починки тестов",
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
        Msg::NextHintRunUnit => "далее: tester run <path> --kind unit",

        Msg::TuiNeedsTty => {
            "tui требует интерактивный терминал; используйте `tester chat <PATH>` или запустите в TTY"
        }
        Msg::FooterHome => " Enter сессия    Ctrl+O проект    q выход ",
        Msg::FooterPicker => {
            " ↑↓  Enter открыть    Ctrl+Enter/Ctrl+S выбрать    -> worktrees    Esc назад    фильтр "
        }
        Msg::FooterSession => {
            " Enter отправить    @файл    /команды    y/N approve    Esc отмена/выход    Ctrl+O проект "
        }
        Msg::TitleHome => " главная ",
        Msg::TitleFolders => " папки ",
        Msg::TitleStatus => " статус ",
        Msg::TitleTranscript => " диалог ",
        Msg::TitleComposer => " ввод ",
        Msg::TitleCommands => " /команды ",
        Msg::TitleSession => " сессия ",
        Msg::TitleOpenProject => " открыть проект ",
        Msg::TitleAttach => " вложения ",
        Msg::TitleFiles => " @файлы ",
        Msg::TitleWorktrees => " worktrees ",
        Msg::NoProject => "(проект не выбран)",
        Msg::ProjectLabel => "Проект",
        Msg::HomeHint => "Откройте проект, чтобы начать сессию.",
        Msg::HomeStatusIdle => "Ctrl+O открыть проект  q выход",
        Msg::HomeStatusSelected => "проект выбран  Ctrl+O сменить  q выход",
        Msg::HomeStatusWarn => "путь с предупреждениями  Ctrl+O сменить  q выход",
        Msg::DiscoverFailed => "discover не удался",
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
        Msg::SessionStatusBlocked => {
            "write/shell требуют y/N  Enter отправить  @файл  /help"
        }
        Msg::SessionCleared => "сессия очищена",
        Msg::SessionRunning => "выполняется... Esc отмена",
        Msg::SessionCancelReq => "запрошена отмена...",
        Msg::SessionHelp => {
            "команды: /scan /plan [kind] /run <prompt> /config /clear /audit /exit  вложение @path или @path#L20-45"
        }
        Msg::SessionUnknownCmd => "неизвестная команда; /help",
        Msg::SessionUsageRun => "usage: /run <prompt>",
        Msg::SessionNoGoals => "нет целей в плане",
        Msg::SessionBadKind => "неверный kind",
        Msg::SessionEmptyResponse => "(пустой ответ)",
        Msg::SessionError => "ошибка - смотрите диалог",
        Msg::SessionThreadExit => "поток агента завершился",
        Msg::RoleYou => "вы",
        Msg::RoleAssistant => "ассистент",
        Msg::RoleTool => "tool",
        Msg::RoleSystem => "система",
        Msg::SupportedWord => "поддерживается",
        Msg::UnsupportedWord => "не поддерживается",
        Msg::PermTitle => " разрешить tool? ",
        Msg::PermHint => "y / Enter = да    n / Esc = нет",
        Msg::PermPrompt => "нужно разрешение - y/n",
        Msg::DoctorTitle => "# tester doctor\n\n",
        Msg::DoctorOk => "ok",
        Msg::DoctorFail => "FAIL",
        Msg::ReportTitle => "# Отчёт tester\n\n",
        Msg::ReportProject => "проект",
        Msg::ReportLanguage => "язык",
        Msg::ReportPackage => "пакет",
        Msg::ReportWarnings => "## Предупреждения\n\n",
        Msg::ReportFailures => "## Сводка failures\n\n",
        Msg::ReportResults => "## Результаты\n\n| символ | вид | файл | попытки | статус | first_pass | density | полезность | команда |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
        Msg::ReportTrace => "Трассировка",
        Msg::ReportStatusPlanned => "запланирован",
        Msg::ReportStatusDryRun => "сухой_прогон",
        Msg::ReportStatusWritten => "записан",
        Msg::ReportStatusPassed => "пройден",
        Msg::ReportStatusUnfixed => "не_исправлен",
        Msg::ReportStatusSkipped => "пропущен",
        Msg::ReportStatusFailedPath => "отказ_пути",
    }
}
