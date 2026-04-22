use super::build_prompt;

#[test]
fn builds_prompt_without_extra_context() {
    let prompt = build_prompt("");

    assert!(prompt.starts_with("Follow these instructions exactly"));
    assert!(prompt.contains("# Git Commit Proposal"));
    assert!(prompt.contains("Run `git status --short`."));
    assert!(!prompt.contains("\n---\n"));
    assert!(!prompt.contains("name: git-commit-proposal"));
    assert!(!prompt.contains("Additional user context"));
}

#[test]
fn appends_trimmed_extra_context() {
    let prompt = build_prompt("  focus on tests  ");

    assert!(prompt.contains("# Git Commit Proposal"));
    assert!(prompt.contains("Additional user context:\nfocus on tests"));
}
