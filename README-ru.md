# Tester

LLM-агент для генерации и починки тестов: unit, integration, UI и contract.

Поддерживаемые языки: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

### Сборка из исходников

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
tester test eval /каталог-с-проектами --suite

# UI e2e
tester ui init /web-app
tester ui doctor /web-app
tester test run /web-app --kind ui

# Здоровье окружения
tester doctor /проект

# Защитный security-скан + harden-патч
tester security scan /проект
tester security patch /проект --finding F-0001

# Мульти-чат сессии
tester sessions list
tester /проект --session <id>
```

Конфиг: `~/.tester/tester.toml`.
По умолчанию `--kind all`; `test run` сразу пишет файлы (отключить: `--dry-run`).

## Документация

- [CLI-команды и глобальные опции](docs/ru/cli.md)
- [TUI (slash / клавиши)](docs/ru/ui.md)
- [Конфиг `tester.toml`](docs/ru/config.md)
- [Коды выхода и угрозы](docs/ru/reference.md)
- [Security (`tester security`)](docs/ru/cli.md#security)

[English](README.md)
