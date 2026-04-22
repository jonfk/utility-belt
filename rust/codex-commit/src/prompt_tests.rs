use super::build_prompt;

#[test]
fn builds_prompt_without_extra_context() {
    let prompt = build_prompt("Skill body", "");

    assert!(prompt.starts_with("Follow these instructions exactly"));
    assert!(prompt.ends_with("Skill body"));
    assert!(!prompt.contains("Additional user context"));
}

#[test]
fn appends_trimmed_extra_context() {
    let prompt = build_prompt("Skill body", "  focus on tests  ");

    assert!(prompt.contains("Skill body"));
    assert!(prompt.contains("Additional user context:\nfocus on tests"));
}
