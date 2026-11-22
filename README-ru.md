# Tester

LLM-агент для генерации и починки тестов: unit, integration, UI и contract.

Поддерживаемые языки: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

### Сборка из исходников

Cargo workspace (`crates/*`). Из корня репозитория:

```bash
cargo build --release -p tester
# бинарник: target/release/tester
```

## Быстрый старт

```bash
tester
tester /путь-проекту
tester test run /путь-проекту
tester agent /путь-проекту "покрой unit-тестами multiply"
tester chat /путь-проекту
```

### Типовые сценарии

```bash
# Написать unit-тесты
tester test run /проект --kind unit

# Починить баг по traceback
tester debug /проект --bug-file /tmp/panic.txt --reproduce-only

# Проверить полезность тестов (мутации)
tester test eval /проект --max-symbols 5
tester test eval ./tests/fixtures --suite

# UI e2e
tester ui init /web-app
tester ui doctor /web-app
tester test run /web-app --kind ui

# Здоровье окружения
tester doctor /проект

# Мульти-чат сессии
tester sessions list
tester /проект --session <id>
```

Конфиг: `<корень>/tester.toml`. По умолчанию `--kind all`; `test run` сразу пишет файлы (отключить: `--dry-run`).

## Документация

| Тема                           | Ссылка                                       |
| ------------------------------ | -------------------------------------------- |
| CLI-команды и глобальные опции | [docs/ru/commands.md](docs/ru/commands.md)   |
| Интерактивный TUI              | [docs/ru/tui.md](docs/ru/tui.md)             |
| Конфиг `tester.toml`           | [docs/ru/config.md](docs/ru/config.md)       |
| Коды выхода и угрозы           | [docs/ru/reference.md](docs/ru/reference.md) |

[English](README.md)
