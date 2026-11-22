#![cfg(test)]

use crate::{SyncOpts, analyze, sync};
use std::fs;

#[test]
fn markdown_sync_analyze_produces_scenarios() {
    let home = tempfile::tempdir().unwrap();
    let docs = home.path().join("wiki");
    let project = home.path().join("proj");
    fs::create_dir_all(docs.join("v1.0")).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::write(
        docs.join("v1.0/login.md"),
        r#"# Login guide

How to log in:

1. Open the login page
2. Enter email and password
3. Click Sign in
4. Expect dashboard
"#,
    )
    .unwrap();

    let (source, index) = sync(
        &project,
        SyncOpts {
            url: None,
            path: Some(docs),
            max_pages: 50,
        },
    )
    .unwrap();
    assert_eq!(index.pages.len(), 1);
    assert!(matches!(source.kind, crate::WikiKind::Markdown));

    let (_analysis, scenarios) = analyze(&project, None).unwrap();
    assert!(
        !scenarios.scenarios.is_empty(),
        "expected howto steps to become scenarios"
    );
    let items = crate::plan_items_from_wiki(&project, Some("1.0"), 10).unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0].kind, tester_core::model::TestKind::Ui);
    assert!(
        scenarios.scenarios.iter().all(|s| s.coverage == "uncovered"
            || s.coverage == "partial"
            || s.coverage == "covered"
            || s.coverage == "unmapped"),
        "align should set coverage"
    );
}

#[test]
fn align_marks_covered_when_e2e_mentions_scenario() {
    let home = tempfile::tempdir().unwrap();
    let docs = home.path().join("wiki");
    let project = home.path().join("proj");
    fs::create_dir_all(docs.join("v1.0")).unwrap();
    fs::create_dir_all(project.join("e2e")).unwrap();
    fs::write(
        docs.join("v1.0/login.md"),
        r#"# Login guide

How to log in:

1. Open the login page
2. Enter email and password
3. Click Sign in
4. Expect dashboard
"#,
    )
    .unwrap();
    sync(
        &project,
        SyncOpts {
            url: None,
            path: Some(docs),
            max_pages: 50,
        },
    )
    .unwrap();
    let (_a, scenarios) = analyze(&project, None).unwrap();
    let id = scenarios.scenarios[0].id.clone();
    fs::write(
        project.join("e2e/login.spec.ts"),
        format!("test('wiki_{id}', async ({{ page }}) => {{ await page.goto('/login'); }});\n"),
    )
    .unwrap();
    let (_a2, scenarios2) = analyze(&project, None).unwrap();
    let sc = scenarios2
        .scenarios
        .iter()
        .find(|s| s.id == id)
        .expect("scenario");
    assert_eq!(sc.coverage, "covered");
    let items = crate::plan_items_from_wiki(&project, None, 10).unwrap();
    assert!(
        items.iter().all(|i| i.gap.symbol.name != id),
        "covered scenarios skipped in plan"
    );
}

#[test]
fn empty_dir_wiki_sync_and_plan() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("empty_proj");
    let docs = home.path().join("wiki");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(docs.join("v1.0")).unwrap();
    std::fs::write(
        docs.join("v1.0/login.md"),
        r#"# Login guide

How to log in:

1. Open the login page
2. Enter email
3. Click Sign in
4. Expect dashboard
"#,
    )
    .unwrap();
    assert!(!project.join("Cargo.toml").exists());
    assert!(!project.join("package.json").exists());

    sync(
        &project,
        SyncOpts {
            url: None,
            path: Some(docs),
            max_pages: 50,
        },
    )
    .unwrap();
    let (_a, scenarios) = analyze(&project, None).unwrap();
    assert!(!scenarios.scenarios.is_empty());
    let items = crate::plan_items_from_wiki(&project, None, 10).unwrap();
    assert!(!items.is_empty());
    assert!(items[0].target_path.starts_with("e2e"));
}
