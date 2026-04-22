use std::fs;

use tempfile::tempdir;

use super::{Proposal, ProposalStatus};

#[test]
fn parses_and_validates_ready_proposal() {
    let dir = tempdir().expect("tempdir");
    let proposal_path = dir.path().join("proposal.json");
    fs::write(
        &proposal_path,
        r#"{
  "status": "ready",
  "summary": "Ready to commit",
  "stage_paths": ["src/main.rs"],
  "commit": {
    "subject": "feat: add entrypoint",
    "body_paragraphs": ["Explain the new behavior."]
  },
  "alternatives": []
}"#,
    )
    .expect("proposal");

    let proposal = Proposal::from_path(&proposal_path).expect("proposal parse");
    proposal.validate().expect("proposal validate");

    assert_eq!(proposal.status, ProposalStatus::Ready);
    assert_eq!(
        proposal.commit.expect("commit").subject,
        "feat: add entrypoint"
    );
}

#[test]
fn validates_split_required_proposal() {
    let proposal: Proposal = serde_json::from_str(
        r#"{
  "status": "split_required",
  "summary": "Changes mix concerns.",
  "stage_paths": [],
  "commit": null,
  "alternatives": [
    {
      "summary": "Separate docs from code changes.",
      "commit_subject": "docs: update README",
      "stage_paths": ["README.md"]
    }
  ]
}"#,
    )
    .expect("deserialize");

    proposal.validate().expect("split_required should validate");
}

#[test]
fn validates_nothing_to_commit_proposal() {
    let proposal: Proposal = serde_json::from_str(
        r#"{
  "status": "nothing_to_commit",
  "summary": "No commit needed.",
  "stage_paths": [],
  "commit": null,
  "alternatives": []
}"#,
    )
    .expect("deserialize");

    proposal
        .validate()
        .expect("nothing_to_commit should validate");
}

#[test]
fn ready_proposal_requires_stage_paths() {
    let proposal: Proposal = serde_json::from_str(
        r#"{
  "status": "ready",
  "summary": "Ready to commit",
  "stage_paths": [],
  "commit": {
    "subject": "feat: add entrypoint",
    "body_paragraphs": []
  },
  "alternatives": []
}"#,
    )
    .expect("deserialize");

    let report = proposal.validate().expect_err("validation should fail");
    assert!(format!("{report:?}").contains("stage path"));
}
