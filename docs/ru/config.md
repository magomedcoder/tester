# Конфиг `tester.toml`

```toml
locale = "ru"

[llm]
base_url = "http://127.0.0.1:11434/v1"
model = ""
api_key = ""

[run]
agent_auto_approve = false
allow_docker = false

[analysis]
embeddings = false
```

| Секция / ключ            | По умолчанию                | Описание                                        |
| ------------------------ | --------------------------- | ----------------------------------------------- |
| `locale`                 | `en`                        | Язык UI: `en` или `ru`                          |
| `llm.base_url`           | `http://127.0.0.1:11434/v1` | OpenAI-совместимый API base                     |
| `llm.model`              | - (нужен для LLM-команд)    | Имя модели                                      |
| `llm.api_key`            | нет                         | Ключ API (только из файла)                      |
| `run.agent_auto_approve` | `false`                     | Auto-approve write/shell                        |
| `run.allow_docker`       | `false`                     | Tool `docker_info` у агента                     |
| `analysis.embeddings`    | `false`                     | Opt-in semantic `/search` (вызов `/embeddings`) |

Опционально (UI-тесты): `[tests.ui]` - `framework`, `base_url`, `start`.  
Продвинутые ключи (`max_tokens`, лимиты агента, ...) по-прежнему читаются, если задать, но не обязательны.

Нужен для: TUI по умолчанию / `test generate`, `test run`, `test repair`, `agent`, `chat`.  
Не нужен для: `test scan`, `test plan`, `test eval`, `ui init`, `sessions`.
