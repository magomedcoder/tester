# Интерактивный UI (по умолчанию)

Запуск `tester` **без подкоманды** открывает TUI (нужен TTY). Без `PATH` - picker, затем chat-сессия. Fallback: `--plain` -> `chat` (нужен PATH).

```bash
tester
tester /путь-проекту
tester /путь-проекту --dry-run
tester /путь-проекту --session <id>
tester /путь-проекту --plain
```

| Флаг        | Описание                                      |
| ----------- | --------------------------------------------- |
| `PATH`      | Опциональный корень проекта (или файл внутри) |
| `--dry-run` | Agent tools не пишут и не гоняют write shell  |
| `--session` | Открыть сохранённый чат по id (или префиксу)  |
| `--plain`   | Классический `chat` REPL вместо TUI           |

**Picker:** `Ctrl+O``Up`/`Down``Enter` папка`Right` worktrees`Ctrl+Enter` / `Ctrl+S` открытьфильтр`Esc` назад. Recents: `~/.local/share/.tester/recents.json` (`tester` без PATH открывает `last_root`, если есть).

**Клавиши:** `Enter` отправить`Shift+Enter` / `Ctrl+Enter` / `Ctrl+J` новая строка`Esc Esc` отмена`Ctrl+C Ctrl+C` выход`Ctrl+R` поиск истории`Ctrl+X Ctrl+E` внешний редактор`Up`/`Down` / `PgUp`/`PgDn` скролл`y`/`n` разрешение tool.

**Вложения:** `@path``@path#L20-45``@Symbol` (Tab - completion).

**Slash:** `/help` `/scan` `/plan` `/run` `/clear` `/audit` `/grep` `/symbol` `/refs` `/search` `/index` `/map` `/list_files` `/show_file` `/manual` `/recent` `/session` `/prune` `/diff` `/branch` `/propose_plan` `/exit`.

Без `agent_auto_approve` (и не `--dry-run`) write/shell открывают modal **y/N**.
