use crate::model::{Draft, TestKind};
use anyhow::{Result, bail};

pub fn check_draft(draft: &Draft) -> Result<()> {
    let c = &draft.content;
    if !has_assertion(c) {
        bail!("gate: нет утверждения (assert/expect/REQUIRE)");
    }
    if is_weak_test(c) {
        bail!("gate: слабый тест (только snapshot/sleep без проверки поведения)");
    }
    if has_forbidden_network(c, draft.kind) {
        bail!("gate: запрещённая сеть/Docker во внешнем сервисе");
    }
    if draft.kind == TestKind::Unit && has_heavy_io(c) {
        bail!("gate: unit-тест тянет сеть/БД - используйте integration");
    }
    Ok(())
}

pub fn has_assertion(content: &str) -> bool {
    const MARKERS: &[&str] = &[
        "assert",
        "expect(",
        "assert_",
        "toHaveText",
        "toBe",
        "REQUIRE",
        "assertEquals",
        "XCTAssert",
        "self.assert",
        "EXPECT_",
        "ASSERT_",
        "should(",
        "toEqual",
        "toStrictEqual",
    ];
    MARKERS.iter().any(|m| content.contains(m))
}

fn is_weak_test(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let only_snapshot = (lower.contains("toMatchSnapshot") || lower.contains("assert_snapshot"))
        && !lower.contains("assert_eq")
        && !lower.contains("expect(")
        && !lower.contains("toBe")
        && !lower.contains("toEqual");
    let flaky = (lower.contains("sleep(")
        || lower.contains("time.sleep")
        || lower.contains("thread::sleep"))
        && !has_assertion(content);
    only_snapshot || flaky
}

fn has_forbidden_network(content: &str, kind: TestKind) -> bool {
    if kind == TestKind::Ui {
        return false;
    }
    let markers = [
        "docker compose",
        "docker-compose",
        "Docker::",
        "localhost:5432",
        "mongodb://",
        "amqp://",
        "redis://",
    ];
    markers.iter().any(|m| content.contains(m))
}

fn has_heavy_io(content: &str) -> bool {
    let markers = [
        "reqwest::",
        "ureq::",
        "fetch(",
        "axios.",
        "http.Get",
        "http.Post",
        "requests.get",
        "requests.post",
        "HttpClient",
        "OkHttp",
        "curl_exec",
        "sqlx::",
        "diesel::",
        "postgres://",
    ];
    markers.iter().any(|m| content.contains(m))
}

pub fn assert_density(content: &str) -> f64 {
    let lines = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
        .max(1) as f64;
    let asserts = content
        .lines()
        .filter(|l| {
            let t = l.to_ascii_lowercase();
            t.contains("assert") || t.contains("expect(") || t.contains("require")
        })
        .count() as f64;
    (asserts / lines).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TestKind;
    use std::path::PathBuf;

    fn draft(content: &str) -> Draft {
        Draft {
            path: PathBuf::from("tests/t.rs"),
            content: content.into(),
            kind: TestKind::Unit,
            symbol_name: "x".into(),
            filter_name: "x".into(),
            append: false,
        }
    }

    #[test]
    fn rejects_no_assert() {
        assert!(check_draft(&draft("fn t() {}")).is_err());
    }

    #[test]
    fn accepts_assert() {
        assert!(check_draft(&draft("#[test] fn t(){ assert_eq!(1,1); }")).is_ok());
    }
}
