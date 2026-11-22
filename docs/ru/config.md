# Конфиг `tester.toml`

Два файла (deep merge, проект перекрывает глобальный):

1. **`~/.tester/tester.toml`** (или `$TESTER_HOME/tester.toml`) - `base_url`, `api_key`, locale, общие defaults. Создаётся автоматически при первом запуске, если файла нет.
2. **`<корень-проекта>/tester.toml`** - оверлей для этого репозитория

Выбор: TUI `/model`, CLI `--model`, `tester models` (auto, если на endpoint ровно одна). Последний выбор — `~/.tester/model`.

```toml
locale = "ru"

[llm]
base_url = "http://127.0.0.1:11434/v1"
# api_key = ""

[run]
agent_auto_approve = true
allow_docker = false

[analysis]
embeddings = false

[security]
enabled = true
deep = false
fetch_advisories = false
```

| Секция / ключ               | По умолчанию                | Описание                                                                                                 |
| --------------------------- | --------------------------- | -------------------------------------------------------------------------------------------------------- |
| `locale`                    | `en`                        | Язык UI: `en` или `ru`                                                                                   |
| `llm.base_url`              | `http://127.0.0.1:11434/v1` | OpenAI-совместимый API base                                                                              |
| `llm.api_key`               | нет                         | Ключ API (только из файла)                                                                               |
| `run.agent_auto_approve`    | `true`                      | Auto-approve write/shell в корне проекта (`false` -> y/N); выход за папку / write policy всё равно режут |
| `run.allow_docker`          | `false`                     | Tool `docker_info` у агента                                                                              |
| `analysis.embeddings`       | `false`                     | Opt-in semantic `/search` (вызов `/embeddings`)                                                          |
| `security.enabled`          | `true`                      | Включить `tester security scan`                                                                          |
| `security.deep`             | `false`                     | Intra-file taint                                                                                         |
| `security.fetch_advisories` | `false`                     | Opt-in запросы OSV                                                                                       |

Опционально (UI-тесты): `[tests.ui]` - `framework`, `base_url`, `start`.  
Продвинутые ключи (`max_tokens`, лимиты агента, ...) по-прежнему читаются, если задать, но не обязательны.

Типично: `[llm]` в глобальном файле, в проекте только отличия (`[tests.ui]`, `android_opt_in`, ...).

Нужен для LLM-работы: TUI chat / `test generate`, `test run`, `test repair`, `agent`, `chat` (плюс выбранная model).  
Не нужен для: `test scan`, `test plan`, `test eval`, `security scan`, `ui init`, `sessions`, `models`.
