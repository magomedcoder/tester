# Interactive UI (default)

Running `tester` with **no subcommand** opens the TUI (requires a TTY). Without `PATH`, opens the project picker, then a chat session. Fallback: `--plain` -> `chat` (PATH required).

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

**Picker:** `Ctrl+O``Up`/`Down``Enter` folder`Right` worktrees`Ctrl+Enter` / `Ctrl+S` opentype to filter`Esc` back. Recents: `~/.local/share/.tester/recents.json` (`tester` without PATH resumes `last_root` when set).

**Keys:** `Enter` send`Shift+Enter` / `Ctrl+Enter` / `Ctrl+J` newline`Esc Esc` cancel`Ctrl+C Ctrl+C` quit`Ctrl+R` history search`Ctrl+X Ctrl+E` external editor`Up`/`Down` / `PgUp`/`PgDn` scroll`y`/`n` tool permission.

**Attach:** `@path``@path#L20-45``@Symbol` (Tab completes).

**Slash:** `/help` `/scan` `/plan` `/run` `/clear` `/audit` `/grep` `/symbol` `/refs` `/search` `/index` `/map` `/list_files` `/show_file` `/manual` `/recent` `/session` `/prune` `/diff` `/branch` `/propose_plan` `/exit`.

Without `agent_auto_approve` (and not `--dry-run`), write/shell open a **y/N** modal.
