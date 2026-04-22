use super::build_prompt;

#[test]
fn builds_prompt_without_extra_context() {
    let prompt = build_prompt("");

    assert!(prompt.starts_with("Follow these instructions exactly"));
    assert!(prompt.contains("# Git Commit Proposal"));
    assert!(prompt.contains("Run `git status --short --branch`"));
    assert!(prompt.contains("git log -n 15 --pretty=format:'%h %ad %s' --date=short"));
    assert!(
        prompt
            .contains("git log -n 8 --pretty=format:'%h %ad %s' --date=short -- <candidate paths>")
    );
    assert!(prompt.contains("git log --skip=40 -n 8"));
    assert!(prompt.contains("package-lock.json"));
    assert!(prompt.contains("Cargo.lock"));
    assert!(prompt.contains("Avoid fully reading lockfiles and generated artifacts"));
    assert!(prompt.contains(
        "Use the user context to choose emphasis, wording, and likely commit scope"
    ));
    assert!(!prompt.contains("\n---\n"));
    assert!(!prompt.contains("name: git-commit-proposal"));
    assert!(!prompt.contains("Additional user context"));
    assert!(!prompt.contains("## User-Provided Commit Context"));
    assert!(!prompt.contains("<<<USER_CONTEXT>>>"));
}

#[test]
fn inserts_trimmed_extra_context_before_rules_and_workflow() {
    let prompt = build_prompt("  focus on tests  ");

    assert!(prompt.contains("# Git Commit Proposal"));
    assert!(prompt.contains("## User-Provided Commit Context"));
    assert!(prompt.contains("<<<USER_CONTEXT>>>\nfocus on tests\n<<<END_USER_CONTEXT>>>"));

    let context_index = prompt.find("## User-Provided Commit Context").unwrap();
    let rules_index = prompt.find("## Rules").unwrap();
    let workflow_index = prompt.find("## Workflow").unwrap();

    assert!(context_index < rules_index);
    assert!(context_index < workflow_index);
}

#[test]
fn omits_context_section_for_whitespace_only_input() {
    let prompt = build_prompt("   ");

    assert!(!prompt.contains("## User-Provided Commit Context"));
    assert!(!prompt.contains("<<<USER_CONTEXT>>>"));
}
