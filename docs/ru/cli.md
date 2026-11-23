# CLI-команды

Внешние команды терминала (`tester test ...`, `agent`, `security`, ...).  
Slash и клавиши интерфейса: [ui.md](ui.md).

Общий аргумент у большинства команд: `<PATH>` - путь к проекту или любому файлу внутри него (корень ищется по манифесту).

## Глобальные опции

| Опция                 | Описание                                                        |
| --------------------- | --------------------------------------------------------------- |
| `--format text\|json` | Формат вывода (`text` по умолчанию)                             |
| `--debug`             | Подробный лог (промпты, HTTP к модели)                          |
| `--locale en\|ru`     | Язык UI (перебивает `TESTER_LOCALE` / `LANG` / `locale` в toml) |
| `--model <ID>`        | Выбрать model на этот запуск                                    |
| `-h`, `--help`        | Справка                                                         |
| `-V`, `--version`     | Версия                                                          |

`[[providers]]` и `agent_auto_approve` - в `~/.tester/tester.toml` (`[[providers]]` / `[run]`).
Model `--model`, `tester models` или TUI `/model` (auto, если на endpoint ровно одна модель; последний выбор в `~/.tester/model`).
Локаль: `--locale` -> `TESTER_LOCALE` -> `LANG` -> `locale` в toml -> `en`.

## Тестовый пайплайн (`tester test ...`)

Подкоманды: `scan` / `plan` / `generate` / `run` / `repair` / `eval`.

### `test scan`

Профиль проекта без обращения к модели: язык, пакет, команда тестов, фреймворки, capabilities, заметки.

```bash
tester test scan /путь-проекту
tester test scan /путь-проекту --format json
```

### `test plan`

Список целей для покрытия. Модель не вызывается.

```bash
tester test plan /путь-проекту --kind unit
tester test plan /путь-проекту --kind all --base main
tester test plan /путь-проекту --kind unit --bug "panic in multiply"
tester test plan /путь-проекту --kind unit --max-items 10 --coverage lcov.info
tester test plan /путь-проекту --show
tester test plan /путь-проекту --apply
```

| Флаг                | Описание                                                      |
| ------------------- | ------------------------------------------------------------- |
| `--kind <KIND>`     | `all` (по умолчанию), `unit`, `integration`, `ui`, `contract` |
| `--base <REF>`      | Только символы из `git diff REF...HEAD`                       |
| `--bug <TEXT>`      | Приоритет символам из traceback / текста бага                 |
| `--bug-file <PATH>` | То же, но текст из файла                                      |
| `--append`          | Планировать дописывание в существующие тестовые файлы         |
| `--package <NAME>`  | Фильтр пакета (Cargo workspace и аналоги)                     |
| `--show`            | Показать сохранённый план из `.tester/plan.json`              |
| `--apply`           | Generate/write по сохранённому плану                          |

### `test generate`

Сгенерировать черновики через LLM. По умолчанию файлы не пишутся - печатается diff. Для записи нужен `--write`.

```bash
tester test generate /путь-проекту --kind unit
tester test generate /путь-проекту --kind unit --write
tester test generate /путь-проекту --kind unit --write --update
tester test generate /путь-проекту --kind unit --append --write
```

| Флаг                                                               | Описание                                                                           |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `--kind`, `--base`, `--bug`, `--bug-file`, `--append`, `--package` | Как у `test plan`                                                                  |
| `--write`                                                          | Записать файлы на диск                                                             |
| `--update`                                                         | Разрешить перезапись уже существующих тестовых файлов (без флага - отказ, код `5`) |

### `test run`

Полный цикл: план -> генерация -> запись -> прогон -> repair при падении (до `repair_attempts`).

```bash
tester test run /путь-проекту
tester test run /путь-проекту --kind ui
tester test run /путь-проекту --dry-run
```

Флаги как у `test generate`, но запись включена по умолчанию; `--dry-run` - только diff без записи и прогона.

### `test repair`

Продолжить починку последнего неуспешного черновика (состояние в `.tester/` проекта).

```bash
tester test repair /путь-проекту
```

### `test eval`

Мутации покрытых символов (arithmetic / boolean / off-by-one / branch / null). Модель не нужна.

```bash
tester test eval /путь-проекту --max-symbols 5
tester test eval /каталог-с-проектами --suite
tester test eval /путь-проекту --json
```

## `debug`

От traceback -> минимальный воспроизводящий тест -> validate/repair.

```bash
tester debug /проект --bug "panic in multiply at src/lib.rs:10"
tester debug /проект --bug-file ./fail.log --reproduce-only
echo "$TRACE" | tester debug /проект --stdin --reproduce-only
```

## `doctor` / `ui doctor`

Проверка LLM, toolchain, UI runner, android/docker.

```bash
tester doctor /проект
tester ui doctor /web-app
```

## `models`

Список id моделей с `GET {base_url}/models`. Текущая модель (если задана) помечена `*`.

```bash
tester models
tester models /путь-проекту
```

## `agent` / `chat`

Консольный агент с tool-calling loop (scan/plan/write_test/run_tests/repair + fs/git/analysis).

```bash
tester agent /путь-проекту "добавь unit-тесты на multiply"
tester agent /путь-проекту "исправь упавшие тесты" --resume --dry-run
tester chat /путь-проекту
```

| Флаг          | Описание                                        |
| ------------- | ----------------------------------------------- |
| `--dry-run`   | Не писать файлы / не гонять shell на запись     |
| `--resume`    | Продолжить активный чат в `~/.tester/sessions/` |
| `--stream`    | Streaming токенов (по умолчанию вкл.)           |
| `--no-stream` | Отключить streaming                             |

В `chat`: `/scan`, `/plan [kind]`, `/run <prompt>`, `/clear`, `/audit`, `/grep`, `/symbol`, `/refs`, `/search`, `/index`, `/map`, `/list_files`, `/show_file`, `/convert`, `/exit`.

### Документы (`/show_file`, `/convert`)

- `/show_file <path>` - outline для кода; извлечённый текст для PDF/office; OCR для изображений/сканов (кэш: `.tester/docs/cache/`).
- `/convert <path> [txt|md|pdf]` - конвертация через LibreOffice/`pandoc`; по умолчанию в `.tester/docs/converted/`.

Нужны утилиты в `PATH` (или пути в `~/.tester/tester.toml` `[documents]`): `pdftotext`, `pdftoppm`, `tesseract`, `soffice`/`libreoffice`, опционально `pandoc`. Agent tools: `show_file`, `convert_document`.

Команды интерфейса (slash / клавиши): [ui.md](ui.md).

## `sessions`

Список / prune мульти-чат сессий в `~/.tester/sessions/<id>/` (meta + transcript; указатель workspace - `.tester/session_id`).

```bash
tester sessions list
tester sessions list --workspace /путь-проекту
tester sessions prune <id>
tester sessions prune --workspace /путь-проекту
tester sessions prune --older-than 30
tester sessions prune --all
```

В TUI: `/session` picker, `/session new`, `/session <id>`, `/prune ...`. `/recent` - **проекты**, не chat-сессии.

## `security`

Только защита: detect -> explain -> harden. Без генерации эксплойтов.

```bash
tester security scan /путь/к/проекту
tester security scan /путь/к/проекту --deep --format json
tester security scan /путь/к/проекту --md --fail-on high
tester security explain /путь/к/проекту F-0001
tester security patch /путь/к/проекту --finding F-0001          # dry-run
tester security patch /путь/к/проекту --finding F-0001 --apply
tester security patch /путь/к/проекту --finding F-0001 --llm --apply
```

Код выхода `2`, если есть findings ≥ `--fail-on` (по умолчанию `high`).

### Конфиг (`[security]` в `tester.toml`)

```toml
[security]
enabled = true
deep = false
fetch_advisories = false
severities = ["critical", "high", "medium", "low", "info"]
auto_patch = false
disabled_rules = []
max_files = 4000
```

Подробнее по всем ключам: [config.md](config.md).

### Agent / TUI

| Канал       | Как                                                                       |
| ----------- | ------------------------------------------------------------------------- |
| Agent tools | `security_scan` (read-only), `security_explain`, `security_patch` (write) |
| TUI         | `/security` - overlay с отчётом; `/security deep` - taint                 |

### Что сканируется

- Секреты / high-entropy / `.env`
- Опасные sinks (exec, eval, pickle, yaml.load, SQL concat, ...)
- CI/Docker (`curl|sh`, privileged, docker.sock)
- Опционально OSV (`--fetch-advisories`)
- Deep: source->sink в одном файле

## `ui init`

Минимальный Playwright: `playwright.config.ts`, каталог `e2e/`, `npm install` + chromium по возможности. UI (`[tests.ui]`) - в `~/.tester/tester.toml`.

```bash
tester ui init /path/to/web-app
tester test run /path/to/web-app --kind ui
```

## Wiki -> UI-тесты (`tester wiki ...`)

Только ссылка или путь к md:

```bash
tester wiki run /path/to/project --url https://wiki.example.org/ --write
tester wiki run /path/to/project --wiki-path ./docs/wiki --write

tester wiki sync /project --url https://wiki.example.org/
tester wiki analyze /project # LLM-батчи по умолчанию
tester wiki analyze /project --no-llm # только эвристики
tester wiki plan /project [--version 1.2]
```
