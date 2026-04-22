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
