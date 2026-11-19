use crate::failure::ErrorClass;
use crate::model::{Language, PlanItem, ProjectProfile, TestKind};

#[cfg(test)]
mod golden_tests;

pub fn system_prompt(kind: TestKind) -> String {
    match kind {
        TestKind::Unit => include_str!("unit.md").to_string(),
        TestKind::Integration => include_str!("integration.md").to_string(),
        TestKind::Ui => include_str!("ui.md").to_string(),
        TestKind::Contract => include_str!("contract.md").to_string(),
        TestKind::All => include_str!("unit.md").to_string(),
    }
}

pub fn user_prompt(
    profile: &ProjectProfile,
    item: &PlanItem,
    style_sample: Option<&str>,
    page_context: Option<&str>,
) -> String {
    let cases = item
        .cases
        .iter()
        .map(|c| {
            format!(
                "- {}: {} | вход: {} | ожидание: {}",
                c.name, c.description, c.input, c.expectation
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut parts = vec![
        format!("language: {}", profile.language.as_str()),
        format!("package: {}", profile.package_name),
        format!("test_kind: {}", item.kind.as_str()),
        format!("target_path: {}", item.target_path.display()),
        format!("symbol: {}", item.gap.symbol.name),
        format!("signature: {}", item.gap.symbol.signature),
        format!("source_file: {}", item.gap.symbol.file.display()),
        format!("body:\n{}", item.gap.symbol.body),
        format!("planned_cases:\n{cases}"),
        "Не изменяй продакшен-код. Верни ровно один файл.".into(),
        "Комментарии в коде теста - только на русском языке.".into(),
        "Обязательные кейсы: happy path, error path (если применимо), boundary.".into(),
    ];

    if let Some(sample) = style_sample {
        parts.push(format!(
            "style_sample (скопируй assert-идиомы, имена тестов, fixtures/helpers):\n{sample}"
        ));
    }
    if let Some(page) = page_context {
        parts.push(format!(
            "page_context (DOM/роли/формы/навигация - строй осмысленный UI-сценарий):\n{page}"
        ));
    }
    if let Some(bug) = &item.bug_context {
        parts.push(format!(
            "bug_context (приоритет / traceback; покрой воспроизведение):\n{bug}"
        ));
    }
    if item.kind == TestKind::Integration || item.kind == TestKind::Contract {
        parts.push(integration_helpers(profile.language));
        parts.push(
            "Используй in-memory фейки, temp dir или testdata. Не поднимай Docker, сеть и внешние сервисы. Только stub/wiremock/in-process."
                .into(),
        );
    }
    if item.kind == TestKind::Contract {
        parts.push(
            "Контракт: запрос -> статус/схема ответа. Consumer или provider test без реальных HTTP-зависимостей."
                .into(),
        );
    }
    if item.kind == TestKind::Unit {
        parts.push("Unit: без сети, БД и браузера. Если нужен I/O - это integration.".into());
    }
    if profile.language == Language::Rust {
        parts.push(format!(
            "Импортируй из крейта `{}`. Предпочитай утверждения, а не println.",
            profile.package_name
        ));
    }
    parts.join("\n\n")
}

fn integration_helpers(lang: Language) -> String {
    let tip = match lang {
        Language::Rust => "Rust: tempfile::tempdir / std::env::temp_dir",
        Language::Python => "Python: tmp_path / tempfile / monkeypatch",
        Language::Go => concat!("Go: t.TempDir() / net/http/", "httptest"),
        Language::TypeScript | Language::JavaScript => {
            "JS/TS: memfs / tmp dir / msw для HTTP stubs"
        }
        Language::Kotlin => "Kotlin: TemporaryFolder / MockWebServer",
        Language::Php => "PHP: vfsStream / temp dir",
        Language::Dart | Language::Flutter => "Dart: Directory.systemTemp",
        Language::Cpp => "C++: временная директория в test fixture",
        Language::Unknown => "temp dir / in-memory fakes",
    };
    format!("integration_helpers: {tip}")
}

pub fn repair_prompt(previous: &str, error: &str, path: &str) -> String {
    repair_prompt_classified(previous, error, path, ErrorClass::Other)
}

pub fn repair_prompt_classified(
    previous: &str,
    error: &str,
    path: &str,
    class: ErrorClass,
) -> String {
    let focus = match class {
        ErrorClass::Compile => {
            "Фокус: исправь ошибки компиляции/импортов/типов. Не меняй смысл assert без нужды."
        }
        ErrorClass::Assert => {
            "Фокус: ожидания assert неверны или кейс хрупкий - поправь ожидание/ввод, сохрани покрытие."
        }
        ErrorClass::Timeout => "Фокус: убери зависания, sleeps, бесконечные ожидания; ускорь тест.",
        ErrorClass::EnvMissing => {
            "Фокус: не требуй отсутствующий toolchain; используй уже доступные API проекта."
        }
        ErrorClass::Other => "Исправь падение по тексту ошибки.",
    };
    format!(
        "Предыдущий тестовый файл упал (класс={}). {focus}\npath: {path}\n\nошибка:\n{error}\n\nпредыдущий файл:\n{previous}\n\nВерни полный исправленный файл в формате path/code fence. Не меняй продакшен-код. Комментарии - на русском.",
        class.as_str()
    )
}
