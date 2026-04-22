use super::build_commit_message;
use crate::proposal::ProposalCommit;

#[test]
fn renders_subject_only_message() {
    let message = build_commit_message(&ProposalCommit {
        subject: "feat: add cli".into(),
        body_paragraphs: vec![],
    });

    assert_eq!(message, "feat: add cli\n");
}

#[test]
fn renders_body_paragraphs_with_spacing() {
    let message = build_commit_message(&ProposalCommit {
        subject: "feat: add cli".into(),
        body_paragraphs: vec![
            "First paragraph.".into(),
            "".into(),
            "Second paragraph.".into(),
        ],
    });

    assert_eq!(
        message,
        "feat: add cli\n\nFirst paragraph.\n\nSecond paragraph.\n\n"
    );
}
