# Config `tester.toml`

```toml
locale = "en"

[llm]
base_url = "http://127.0.0.1:11434/v1"
model = ""
api_key = ""

[run]
agent_auto_approve = false
allow_docker = false

[analysis]
# semantic /search (needs an embeddings-capable endpoint)
embeddings = false
```

| Section / key            | Default                     | Description                                     |
| ------------------------ | --------------------------- | ----------------------------------------------- |
| `locale`                 | `en`                        | UI language: `en` or `ru`                       |
| `llm.base_url`           | `http://127.0.0.1:11434/v1` | OpenAI-compatible API base                      |
| `llm.model`              | - (required for LLM cmds)   | Model name                                      |
| `llm.api_key`            | none                        | API key (file only, not env)                    |
| `run.agent_auto_approve` | `false`                     | Auto-approve write/shell                        |
| `run.allow_docker`       | `false`                     | Enable agent `docker_info` tool                 |
| `analysis.embeddings`    | `false`                     | Opt-in semantic `/search` (calls `/embeddings`) |

Optional (UI tests): `[tests.ui]` - `framework`, `base_url`, `start`.  
Advanced overrides (`max_tokens`, repair budget, agent limits, ...) still parse if set, but are not required.

Required for: default TUI / `test generate`, `test run`, `test repair`, `agent`, `chat`.  
Not required for: `test scan`, `test plan`, `test eval`, `ui init`, `sessions`.
