# Команды

Общий аргумент у большинства команд: `<PATH>` - путь к проекту или любому файлу внутри него (корень ищется по манифесту).

## Глобальные опции

| Опция                 | Описание                                                        |
| --------------------- | --------------------------------------------------------------- |
| `--format text\|json` | Формат вывода (`text` по умолчанию)                             |
| `--debug`             | Подробный лог (промпты, HTTP к модели)                          |
| `--locale en\|ru`     | Язык UI (перебивает `TESTER_LOCALE` / `LANG` / `locale` в toml) |
| `-h`, `--help`        | Справка                                                         |
| `-V`, `--version`     | Версия                                                          |

Модель, `base_url`, `api_key` и `agent_auto_approve` задаются только в `tester.toml` (`[llm]` / `[run]`). Локаль: `--locale` -> `TESTER_LOCALE` -> `LANG` -> `locale` в `tester.toml` -> `en`.

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
tester test eval /путь-к/fixtures --suite
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

В `chat`: `/scan`, `/plan [kind]`, `/run <prompt>`, `/clear`, `/audit`, `/grep`, `/symbol`, `/refs`, `/search`, `/index`, `/map`, `/list_files`, `/show_file`, `/exit`.

Полноэкранный UI: [tui.md](tui.md).

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

В TUI: `/session` picker`/session new``/session <id>``/prune ...`. `/recent` - **проекты**, не chat-сессии.

## `ui init`

Минимальный Playwright: `playwright.config.ts`, каталог `e2e/`, секция UI в `tester.toml` (если файла ещё нет), `npm install` + chromium по возможности.

```bash
tester ui init /path/to/web-app
tester test run /path/to/web-app --kind ui
```
