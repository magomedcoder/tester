# Tester

LLM-агент для генерации и починки тестов: unit, integration, UI и contract.

Поддерживаемые языки: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

### Сборка из исходников

```bash
cargo build --release
# бинарник: target/release/tester
```

## Быстрый старт

```bash
tester run /путь-проекту
tester agent /путь-проекту "покрой unit-тестами multiply"
tester chat /путь-проекту
tester tui
tester tui /путь-проекту
```

### Типовые сценарии

```bash
# Написать unit-тесты
tester run /проект --kind unit

# Починить баг по traceback
tester debug /проект --bug-file /tmp/panic.txt --reproduce-only

# Проверить полезность тестов (мутации)
tester eval /проект --max-symbols 5
tester eval ./tests/fixtures --suite

# UI e2e
tester ui init /web-app
tester ui doctor /web-app
tester run /web-app --kind ui

# Здоровье окружения
tester doctor /проект
```

Конфиг: `<корень>/tester.toml`. По умолчанию `--kind all`; `run` сразу пишет файлы (отключить: `--dry-run`).

## Глобальные опции

| Опция                 | Описание                                                        |
| --------------------- | --------------------------------------------------------------- |
| `--format text\|json` | Формат вывода (`text` по умолчанию)                             |
| `--debug`             | Подробный лог (промпты, HTTP к модели)                          |
| `--locale en\|ru`     | Язык UI (перебивает `TESTER_LOCALE` / `LANG` / `locale` в toml) |
| `-h`, `--help`        | Справка                                                         |
| `-V`, `--version`     | Версия                                                          |

Модель, `base_url`, `api_key` и `agent_auto_approve` задаются только в `tester.toml` (`[llm]` / `[run]`). Локаль: `--locale` -> `TESTER_LOCALE` -> `LANG` -> `locale` в `tester.toml` -> `en`.

## Команды

Общий аргумент у большинства команд: `<PATH>` - путь к проекту или любому файлу внутри него (корень ищется по манифесту).

### `scan`

Профиль проекта без обращения к модели: язык, пакет, команда тестов, фреймворки, capabilities, заметки.

```bash
tester scan /путь-проекту
tester scan /путь-проекту --format json
```

### `plan`

Список целей для покрытия. Модель не вызывается.

```bash
tester plan /путь-проекту --kind unit
tester plan /путь-проекту --kind all --base main
tester plan /путь-проекту --kind unit --bug "panic in multiply"
```

| Флаг                | Описание                                                      |
| ------------------- | ------------------------------------------------------------- |
| `--kind <KIND>`     | `all` (по умолчанию), `unit`, `integration`, `ui`, `contract` |
| `--base <REF>`      | Только символы из `git diff REF...HEAD`                       |
| `--bug <TEXT>`      | Приоритет символам из traceback / текста бага                 |
| `--bug-file <PATH>` | То же, но текст из файла                                      |
| `--append`          | Планировать дописывание в существующие тестовые файлы         |
| `--package <NAME>`  | Фильтр пакета (Cargo workspace и аналоги)                     |

### `generate`

Сгенерировать черновики через LLM. По умолчанию файлы не пишутся - печатается diff. Для записи нужен `--write`.

```bash
tester generate /путь-проекту --kind unit
tester generate /путь-проекту --kind unit --write
tester generate /путь-проекту --kind unit --write --update
tester generate /путь-проекту --kind unit --append --write
```

| Флаг                                                               | Описание                                                                           |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `--kind`, `--base`, `--bug`, `--bug-file`, `--append`, `--package` | Как у `plan`                                                                       |
| `--write`                                                          | Записать файлы на диск                                                             |
| `--update`                                                         | Разрешить перезапись уже существующих тестовых файлов (без флага - отказ, код `5`) |

### `run`

Полный цикл: план -> генерация -> запись -> прогон -> repair при падении (до `repair_attempts`).

```bash
tester run /путь-проекту
tester run /путь-проекту --kind ui
tester run /путь-проекту --dry-run
```

Флаги как у `generate`, но запись включена по умолчанию; `--dry-run` - только diff без записи и прогона.

### `repair`

Продолжить починку последнего неуспешного черновика (состояние в `.tester/` проекта).

```bash
tester repair /путь-проекту
```

### `eval`

Мутации покрытых символов (arithmetic / boolean / off-by-one / branch / null). Модель не нужна.

```bash
tester eval /путь-проекту --max-symbols 5
tester eval /путь-к/fixtures --suite
tester eval /путь-проекту --json
```

### `debug`

От traceback -> минимальный воспроизводящий тест -> validate/repair.

```bash
tester debug /проект --bug "panic in multiply at src/lib.rs:10"
tester debug /проект --bug-file ./fail.log --reproduce-only
echo "$TRACE" | tester debug /проект --stdin --reproduce-only
```

### `doctor` / `ui doctor`

Проверка LLM, toolchain, UI runner, android/docker.

```bash
tester doctor /проект
tester ui doctor /web-app
```

### `plan --show` / `plan --apply`

```bash
tester plan /проект --kind unit --max-items 10 --coverage lcov.info
tester plan /проект --show
tester plan /проект --apply
```

### `agent` / `chat`

Консольный агент с tool-calling loop (scan/plan/write_test/run_tests/repair + fs/git).

```bash
tester agent /путь-проекту "добавь unit-тесты на multiply"
tester agent /путь-проекту "исправь упавшие тесты" --resume --dry-run
tester chat /путь-проекту
```

| Флаг          | Описание                                    |
| ------------- | ------------------------------------------- |
| `--dry-run`   | Не писать файлы / не гонять shell на запись |
| `--resume`    | Продолжить `.tester/session.json`           |
| `--stream`    | Streaming токенов (по умолчанию вкл.)       |
| `--no-stream` | Отключить streaming                         |

В `chat`: `/scan`, `/plan [kind]`, `/run <prompt>`, `/config`, `/clear`, `/audit`, `/exit`.

### `tui`

Интерактивный терминальный UI. Нужен TTY. Без `PATH` - picker, затем chat-сессия. Fallback: `--plain` -> `chat` (нужен PATH).

```bash
tester tui
tester tui /путь-проекту --dry-run
tester tui /путь-проекту --resume
tester tui /путь-проекту --plain
```

| Флаг        | Описание                                      |
| ----------- | --------------------------------------------- |
| `--dry-run` | Agent tools не пишут и не гоняют write shell  |
| `--resume`  | Загрузить `.tester/session.json` в transcript |
| `--plain`   | Классический `chat` REPL вместо TUI           |

Picker: `Ctrl+O`  `↑↓`  `Enter` папка  `Ctrl+Enter` / `Ctrl+S` выбрать  фильтр  `Esc` назад. Recents: `~/.local/share/.tester/recents.json`.

Session: `Enter` отправить  `Ctrl+J` новая строка  `/scan` `/plan` `/run` `/config` `/clear` `/audit` `/exit`  `Esc` отмена/выход. Без `agent_auto_approve` (и не `--dry-run`) write/shell блокируются до modal confirm.

### `ui init`

Минимальный Playwright: `playwright.config.ts`, каталог `e2e/`, запись секции UI в `tester.toml` (если файла ещё нет), `npm install` + chromium по возможности.

```bash
tester ui init /path/to/web-app
tester run /path/to/web-app --kind ui
```

## Конфиг `tester.toml`

```toml
locale = "ru"

[llm]
base_url = "http://127.0.0.1:8080/v1"
model = ""
api_key = ""
max_tokens = 4096
timeout_secs = 120

[tests.ui]
framework = "playwright"
base_url = "http://127.0.0.1:3000"
start = "python3 -m http.server 3000"

[run]
repair_attempts = 3
timeout_secs = 120
allow_docker = false
max_context_chars = 24000
android_opt_in = false
max_agent_turns = 20
max_tool_calls = 40
agent_deadline_secs = 600
agent_auto_approve = false
```

### Верхний уровень

| Ключ     | По умолчанию | Описание               |
| -------- | ------------ | ---------------------- |
| `locale` | `en`         | Язык UI: `en` или `ru` |

### `[llm]`

| Ключ           | По умолчанию               | Описание                              |
| -------------- | -------------------------- | ------------------------------------- |
| `base_url`     | `http://127.0.0.1:8080/v1` | OpenAI-совместимый api base URL       |
| `model`        | - (обязателен)             | Имя модели                            |
| `api_key`      | нет                        | Ключ API (только из файла, не из env) |
| `max_tokens`   | `4096`                     | Лимит токенов ответа                  |
| `timeout_secs` | `120`                      | Таймаут запроса к модели              |
### `[tests.ui]`

| Key                  | Default                 | Description                                             |
| -------------------- | ----------------------- | ------------------------------------------------------- |
| `tests.ui.framework` | `playwright`            | UI framework                                            |
| `tests.ui.base_url`  | `http://127.0.0.1:3000` | Playwright baseURL                                      |
| `tests.ui.start`     | none                    | Shell command to start a local server before the UI run |

All kinds (unit, integration, UI, contract) are always enabled; filter with `--kind`.
:3000` | baseURL для Playwright                                    |
| `tests.ui.start`     | нет                     | Shell-команда старта локального сервера перед UI-прогоном |

Все kinds (unit, integration, UI, contract) всегда включены; фильтр - через `--kind`.

### `[run]`

| Ключ                  | По умолчанию | Описание                                     |
| --------------------- | ------------ | -------------------------------------------- |
| `repair_attempts`     | `3`          | Бюджет попыток repair                        |
| `timeout_secs`        | `120`        | Таймаут validate (прогон тестов)             |
| `allow_docker`        | `false`      | Включает tool `docker_info` у агента         |
| `max_context_chars`   | `24000`      | Максимум символов контекста / compaction     |
| `android_opt_in`      | `false`      | Kotlin: `connectedAndroidTest` / androidTest |
| `max_agent_turns`     | `20`         | Лимит ходов agent loop                       |
| `max_tool_calls`      | `40`         | Лимит вызовов tools за сессию                |
| `agent_deadline_secs` | `600`        | Deadline агента                              |
| `agent_auto_approve`  | `false`      | Auto-approve write/shell в `agent`/`chat`    |

Нужен для: `generate`, `run`, `repair`, `agent`, `chat`.  
Не нужен для: `scan`, `plan`, `eval`, `ui init`.

## Коды выхода

| Код | Значение                                              |
| --- | ----------------------------------------------------- |
| `0` | успех                                                 |
| `2` | нет целей в плане                                     |
| `3` | модель недоступна                                     |
| `4` | тест не исправлен за бюджет repair (`unfixed`)        |
| `5` | отказ записи из-за пути / политики / нужен `--update` |

## Угрозы и меры

| Угроза               | Мера                                                 |
| -------------------- | ---------------------------------------------------- |
| Запись вне тестов    | Write policy адаптера языка; код `5`                 |
| Путь с `..`          | Нормализация относительно корня проекта              |
| Бесконечный repair   | `run.repair_attempts`                                |
| Произвольные команды | Только argv адаптера + опциональный `tests.ui.start` |
