# Конфиг `tester.toml`

Один файл: **`~/.tester/tester.toml`** (или `$TESTER_HOME/tester.toml`).  
Создаётся автоматически при первом запуске, если файла нет. Per-project overlay нет.

Выбор: `/model`, `--model`, `/provider` (план), `--provider`.
Persist: `~/.tester/model`, `~/.tester/provider`.

```toml
locale = "ru"

[[providers]]
id = "llamacpp"
kind = "openai_compat"
base_url = "http://127.0.0.1:8080/v1"
# api_key = "" # только если сервер запущен с --api-key

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

| Секция / ключ               | По умолчанию    | Описание                                                                                                 |
| --------------------------- | --------------- | -------------------------------------------------------------------------------------------------------- |
| `locale`                    | `en`            | Язык UI: `en` или `ru`                                                                                   |
| `[[providers]]`             | один `llamacpp` | Провайдеры: `id`, `kind` (`openai_compat`), `base_url`, опционально `api_key`                            |
| `run.agent_auto_approve`    | `true`          | Auto-approve write/shell в корне проекта (`false` -> y/N); выход за папку / write policy всё равно режут |
| `run.allow_docker`          | `false`         | Tool `docker_info` у агента                                                                              |
| `analysis.embeddings`       | `false`         | Opt-in semantic `/search` (вызов `/embeddings`)                                                          |
| `security.enabled`          | `true`          | Включить `tester security scan`                                                                          |
| `security.deep`             | `false`         | Intra-file taint                                                                                         |
| `security.fetch_advisories` | `false`         | Opt-in запросы OSV                                                                                       |

Опционально UI: `[tests.ui]`.

Нужен для LLM-работы: выбранная model (+ active provider).  
Не нужен для: `test scan`, `test plan`, `test eval`, `security scan`, `ui init`, `sessions`, `models`.
