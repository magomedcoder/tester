# Tester

LLM-агент для генерации и починки тестов: unit, integration, UI и contract.

Поддерживаемые языки: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

```bash
cargo build
cargo run -- run /путь-проекту
cargo run -- agent /путь-проекту "покрой unit-тестами multiply"
cargo run -- chat /путь-проекту
```

### Типовые сценарии

```bash
# Написать unit-тесты
cargo run -- run /проект --kind unit

# Починить баг по traceback
cargo run -- debug /проект --bug-file /tmp/panic.txt --reproduce-only

# Проверить полезность тестов (мутации)
cargo run -- eval /проект --max-symbols 5
cargo run -- eval ./tests/fixtures --suite

# UI e2e
cargo run -- ui init /web-app
cargo run -- ui doctor /web-app
cargo run -- run /web-app --kind ui

# Здоровье окружения
cargo run -- doctor /проект
```

Конфиг: `<корень>/tester.toml`. По умолчанию `--kind all`; `run` сразу пишет файлы (отключить: `--dry-run`).

## Глобальные опции

| Опция                 | Описание                               |
| --------------------- | -------------------------------------- |
| `--format text\|json` | Формат вывода (`text` по умолчанию)    |
| `--debug`             | Подробный лог (промпты, HTTP к модели) |
| `-h`, `--help`        | Справка                                |
| `-V`, `--version`     | Версия                                 |

Модель, `base_url`, `api_key` и `agent_auto_approve` задаются только в `tester.toml` (`[llm]` / `[run]`).

## Команды

Общий аргумент у большинства команд: `<PATH>` - путь к проекту или любому файлу внутри него (корень ищется по манифесту).

### `scan`

Профиль проекта без обращения к модели: язык, пакет, команда тестов, фреймворки, capabilities, заметки.

```bash
cargo run -- scan /путь-проекту
cargo run -- scan /путь-проекту --format json
```

### `plan`

Список целей для покрытия. Модель не вызывается.

```bash
cargo run -- plan /путь-проекту --kind unit
cargo run -- plan /путь-проекту --kind all --base main
cargo run -- plan /путь-проекту --kind unit --bug "panic in multiply"
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
cargo run -- generate /путь-проекту --kind unit
cargo run -- generate /путь-проекту --kind unit --write
cargo run -- generate /путь-проекту --kind unit --write --update
cargo run -- generate /путь-проекту --kind unit --append --write
```

| Флаг                                                               | Описание                                                                           |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `--kind`, `--base`, `--bug`, `--bug-file`, `--append`, `--package` | Как у `plan`                                                                       |
| `--write`                                                          | Записать файлы на диск                                                             |
| `--update`                                                         | Разрешить перезапись уже существующих тестовых файлов (без флага - отказ, код `5`) |

### `run`

Полный цикл: план -> генерация -> запись -> прогон -> repair при падении (до `repair_attempts`).

```bash
cargo run -- run /путь-проекту
cargo run -- run /путь-проекту --kind ui
cargo run -- run /путь-проекту --dry-run
```

Флаги как у `generate`, но запись включена по умолчанию; `--dry-run` - только diff без записи и прогона.

### `repair`

Продолжить починку последнего неуспешного черновика (состояние в `.tester/` проекта).

```bash
cargo run -- repair /путь-проекту
```

### `eval`

Мутации покрытых символов (arithmetic / boolean / off-by-one / branch / null). Модель не нужна.

```bash
cargo run -- eval /путь-проекту --max-symbols 5
cargo run -- eval /путь-к/fixtures --suite
cargo run -- eval /путь-проекту --json
```

### `debug`

От traceback -> минимальный воспроизводящий тест -> validate/repair.

```bash
cargo run -- debug /проект --bug "panic in multiply at src/lib.rs:10"
cargo run -- debug /проект --bug-file ./fail.log --reproduce-only
echo "$TRACE" | cargo run -- debug /проект --stdin --reproduce-only
```

### `doctor` / `ui doctor`

Проверка LLM, toolchain, UI runner, android/docker.

```bash
cargo run -- doctor /проект
cargo run -- ui doctor /web-app
```

### `plan --show` / `plan --apply`

```bash
cargo run -- plan /проект --kind unit --max-items 10 --coverage lcov.info
cargo run -- plan /проект --show
cargo run -- plan /проект --apply
```

### `agent` / `chat`

Консольный агент с tool-calling loop (scan/plan/write_test/run_tests/repair + fs/git).

```bash
cargo run -- agent /путь-проекту "добавь unit-тесты на multiply"
cargo run -- agent /путь-проекту "исправь упавшие тесты" --resume --dry-run
cargo run -- chat /путь-проекту
```

| Флаг          | Описание                                    |
| ------------- | ------------------------------------------- |
| `--dry-run`   | Не писать файлы / не гонять shell на запись |
| `--resume`    | Продолжить `.tester/session.json`           |
| `--stream`    | Streaming токенов (по умолчанию вкл.)       |
| `--no-stream` | Отключить streaming                         |

В `chat`: `/scan`, `/plan [kind]`, `/run <prompt>`, `/config`, `/clear`, `/audit`, `/exit`.

### `tui`

Интерактивный терминальный UI (каркас). Нужен TTY. `q` / Esc - выход. Fallback: `--plain` -> `chat` (нужен PATH).

```bash
cargo run -- tui
cargo run -- tui /путь-проекту --dry-run
cargo run -- tui /путь-проекту --plain
```

| Флаг        | Описание                            |
| ----------- | ----------------------------------- |
| `--dry-run` | Проброс в будущую session / agent   |
| `--resume`  | Зарезервировано под resume сессии   |
| `--plain`   | Классический `chat` REPL вместо TUI |

### `ui init`

Минимальный Playwright: `playwright.config.ts`, каталог `e2e/`, запись секции UI в `tester.toml` (если файла ещё нет), `npm install` + chromium по возможности.

```bash
cargo run -- ui init /path/to/web-app
cargo run -- run /path/to/web-app --kind ui
```

### `cpp init`

Каркас CMake + GoogleTest (`tests/`), минимальный `src/add.cpp`.

```bash
cargo run -- cpp init /path/to/cpp-app
cargo run -- run /path/to/cpp-app --kind unit
```

## Конфиг `tester.toml`

```toml
[llm]
base_url = "http://127.0.0.1:8080/v1"
model = ""
api_key = ""
max_tokens = 4096
timeout_secs = 120

[tests.unit]
enabled = true

[tests.integration]
enabled = true

[tests.contract]
enabled = true

[tests.ui]
enabled = false
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

### `[llm]`

| Ключ           | По умолчанию               | Описание                              |
| -------------- | -------------------------- | ------------------------------------- |
| `base_url`     | `http://127.0.0.1:8080/v1` | OpenAI-совместимый api base URL       |
| `model`        | - (обязателен)             | Имя модели                            |
| `api_key`      | нет                        | Ключ API (только из файла, не из env) |
| `max_tokens`   | `4096`                     | Лимит токенов ответа                  |
| `timeout_secs` | `120`                      | Таймаут запроса к модели              |

### `[tests.*]`

| Ключ                        | По умолчанию            | Описание                                                  |
| --------------------------- | ----------------------- | --------------------------------------------------------- |
| `tests.unit.enabled`        | `true`                  | Учитывать unit-цели                                       |
| `tests.integration.enabled` | `true`                  | Учитывать integration-цели                                |
| `tests.contract.enabled`    | `true`                  | Учитывать contract-цели                                   |
| `tests.ui.enabled`          | `false`                 | UI-цели (для `run --kind ui` включается автоматически)    |
| `tests.ui.framework`        | `playwright`            | UI-фреймворк                                              |
| `tests.ui.base_url`         | `http://127.0.0.1:3000` | baseURL для Playwright                                    |
| `tests.ui.start`            | нет                     | Shell-команда старта локального сервера перед UI-прогоном |

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
Не нужен для: `scan`, `plan`, `eval`, `ui init`, `cpp init`.

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
