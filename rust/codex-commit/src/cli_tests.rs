use clap::Parser;

use super::Cli;

#[test]
fn parses_without_additional_context() {
    let cli = Cli::try_parse_from(["codex-commit"]).expect("cli should parse");

    assert!(cli.additional_context.is_empty());
    assert_eq!(cli.extra_context(), "");
}

#[test]
fn joins_additional_context_with_spaces() {
    let cli =
        Cli::try_parse_from(["codex-commit", "focus", "security", "fixes"]).expect("cli parse");

    assert_eq!(cli.extra_context(), "focus security fixes");
}
