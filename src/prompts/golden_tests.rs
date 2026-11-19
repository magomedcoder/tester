use super::system_prompt;
use crate::model::TestKind;

#[test]
fn unit_system_prompt_stable() {
    let p = system_prompt(TestKind::Unit);
    assert!(p.contains("юнит"));
    assert!(p.contains("assert"));
    assert!(p.contains("```path"));
}

#[test]
fn all_kinds_have_prompts() {
    for kind in [
        TestKind::Unit,
        TestKind::Integration,
        TestKind::Ui,
        TestKind::Contract,
    ] {
        let p = system_prompt(kind);
        assert!(!p.trim().is_empty(), "{kind:?}");
        assert!(p.contains("path") || p.contains("файл"), "{kind:?}");
    }
}
