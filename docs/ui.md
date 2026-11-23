# Interactive UI

Slash commands, keybinds, and attachments inside the interactive UI.

```bash
tester
tester /path/to/project
tester /path/to/project --dry-run
tester /path/to/project --session <id>
tester /path/to/project --plain
```

| Flag        | Description                                            |
| ----------- | ------------------------------------------------------ |
| `PATH`      | Optional project root (or any file inside it)          |
| `--dry-run` | Agent tools do not write / do not run write shell cmds |
| `--session` | Open a stored chat by id (or unique prefix)            |
| `--plain`   | Use classic `chat` REPL instead of TUI                 |

## Session keys

| Key                                     | Action                                |
| --------------------------------------- | ------------------------------------- |
| `Up` / `Down`                           | Navigate the list                     |
| `Enter`                                 | Enter folder                          |
| `Right`                                 | Worktrees of the current repo         |
| `Left`                                  | Back to folders (from worktrees)      |
| `Ctrl+Enter` / `Ctrl+S`                 | Open selected project                 |
| `Esc`                                   | Back / cancel                         |
| `Ctrl+O` (from session)                 | Open picker again                     |
| `Enter`                                 | Send message                          |
| `Shift+Enter` / `Ctrl+Enter` / `Ctrl+J` | Newline in the composer               |
| `Esc` `Esc`                             | Cancel job / close overlay            |
| `Ctrl+C` `Ctrl+C`                       | Quit                                  |
| `Ctrl+O`                                | Open project (picker)                 |
| `Ctrl+R`                                | Search input history                  |
| `Ctrl+X` `Ctrl+E`                       | External editor                       |
| `Up` / `Down` / `PgUp` / `PgDn`         | Scroll the transcript                 |
| `y` / `Enter`                           | Allow tool (when confirm is required) |
| `n` / `Esc`                             | Deny tool                             |
| `Ctrl+A` / `Ctrl+E`                     | Start / end of line                   |
| `Ctrl+Home` / `Ctrl+End`                | Start / end of text                   |
| `Ctrl+W` / `Ctrl+U` / `Ctrl+K`          | Delete word / to start / to end       |
| `Ctrl+Y`                                | Yank deleted text                     |
| `Ctrl+P` / `Ctrl+N`                     | Input history                         |
| `Ctrl+←` / `Ctrl+→`                     | Move by word                          |
| `Ctrl+V` / `Shift+Insert`               | Paste from clipboard                  |

## Attachments

| Syntax         | Description             |
| -------------- | ----------------------- |
| `@path`        | Attach a file or folder |
| `@path#L10-40` | Line range              |
| `@Symbol`      | Jump / attach by symbol |

`Tab` - completion in the `@` / `/` palette.

## Slash commands

Typing `/` opens the palette (`↑↓` `Tab` `Enter`). Full list:

| Command                | Description                                                   |
| ---------------------- | ------------------------------------------------------------- |
| `/help`                | Help: commands and keybinds                                   |
| `/scan`                | Project language, package, test command                       |
| `/plan [kind]`         | Build and save a test plan (`unit`, `integration`, `ui`, ...) |
| `/propose_plan [kind]` | Plan preview overlay (a approve / r reject / e edit)          |
| `/run <prompt>`        | Run the agent with a prompt                                   |
| `/clear`               | Clear messages in the current chat session                    |
| `/audit`               | Audit log / capabilities summary                              |
| `/grep <pattern>`      | Regex search across the repo                                  |
| `/symbol <name>`       | Symbol definition                                             |
| `/refs <name>`         | References to a symbol                                        |
| `/search <query>`      | Semantic search (needs `analysis.embeddings=true`)            |
| `/index`               | Index status                                                  |
| `/index rebuild`       | Rebuild the symbol index                                      |
| `/map`                 | Project structure map                                         |
| `/list_files`          | Project file list (deeper map)                                |
| `/show_file <path>`    | Source outline or PDF/office/image text (OCR cached)          |
| `/convert <path> [to]` | Convert document to txt/md/pdf                                |
| `/wiki ...`            | Wiki sync/analyze/run/status                                  |
| `/manual`              | Built-in manual (overlay)                                     |
| `/recent`              | Recent projects                                               |
| `/session`             | List / switch chat sessions (overlay)                         |
| `/session new`         | New chat session                                              |
| `/session <id>`        | Switch to a session by id / prefix                            |
| `/prune <id>`          | Delete a stored session by id                                 |
| `/prune -w`            | Delete sessions for the current workspace                     |
| `/prune -o <days>`     | Delete sessions older than N days (default 30)                |
| `/prune all`           | Delete all stored sessions                                    |
| `/diff`                | View `git diff`                                               |
| `/branch`              | Switch git branch                                             |
| `/model` [filter]      | Pick LLM model                                                |
| `/security`            | Security scan (overlay)                                       |
| `/security deep`       | Deep scan (taint)                                             |
| `/exit`                | Leave the session                                             |
