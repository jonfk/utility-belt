use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::tempdir;

const BINARY_PATH: &str = env!("CARGO_BIN_EXE_codex-commit");

#[test]
fn unstaged_ready_flow_stages_and_commits_expected_files() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("src.txt", "hello\n").expect("file");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit the change.","stage_paths":["src.txt"],"commit":{"subject":"feat: add src file","body_paragraphs":["Document the change."]},"alternatives":[]}"#,
    );

    let output = harness.run(["y\n"], &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let subject = harness.git(["log", "-1", "--pretty=%s"]).expect("git log");
    assert_eq!(subject.trim(), "feat: add src file");

    let committed_files = harness
        .git(["show", "--pretty=", "--name-only", "HEAD"])
        .expect("show files");
    assert_eq!(committed_files.trim(), "src.txt");
}

#[test]
fn staged_ready_flow_commits_existing_staging_without_restaging() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("staged.txt", "staged\n").expect("file");
    harness.git(["add", "--", "staged.txt"]).expect("stage");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit staged work.","stage_paths":["staged.txt"],"commit":{"subject":"feat: commit staged file","body_paragraphs":[]},"alternatives":[]}"#,
    );

    let output = harness.run(["y\n"], &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let subject = harness.git(["log", "-1", "--pretty=%s"]).expect("git log");
    assert_eq!(subject.trim(), "feat: commit staged file");
}

#[test]
fn staged_mismatch_refuses_to_proceed() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness
        .write_file("current.txt", "current\n")
        .expect("file");
    harness
        .write_file("proposal.txt", "proposal\n")
        .expect("file");
    harness.git(["add", "--", "current.txt"]).expect("stage");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit staged work.","stage_paths":["proposal.txt"],"commit":{"subject":"feat: wrong file","body_paragraphs":[]},"alternatives":[]}"#,
    );

    let output = harness.run(["y\n"], &[]);
    assert!(!output.status.success(), "process should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Proposal does not match the current staged set"));

    let staged = harness
        .git(["diff", "--cached", "--name-only"])
        .expect("cached diff");
    assert_eq!(staged.trim(), "current.txt");
}

#[test]
fn split_required_exits_without_mutation() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("a.txt", "a\n").expect("file");
    harness.write_file("b.txt", "b\n").expect("file");

    harness.set_stub_proposal(
        r#"{"status":"split_required","summary":"Changes should be split.","stage_paths":[],"commit":null,"alternatives":[{"summary":"Commit file a separately.","commit_subject":"feat: add a","stage_paths":["a.txt"]},{"summary":"Commit file b separately.","commit_subject":"feat: add b","stage_paths":["b.txt"]}]}"#,
    );

    let output = harness.run([""], &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Changes should be split."));
    assert!(stdout.contains("Suggested split commits"));
    assert_eq!(
        harness
            .git(["status", "--short"])
            .expect("git status")
            .trim(),
        "?? a.txt\n?? b.txt"
    );
}

#[test]
fn nothing_to_commit_exits_cleanly_without_mutation() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");

    harness.set_stub_proposal(
        r#"{"status":"nothing_to_commit","summary":"No commit created.","stage_paths":[],"commit":null,"alternatives":[]}"#,
    );

    let output = harness.run([""], &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("No commit created."));
}

#[test]
fn editor_loop_can_modify_commit_message_before_commit() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("edit.txt", "edit\n").expect("file");

    let editor_script = harness.stub_dir.join("editor.sh");
    write_executable(
        &editor_script,
        r#"#!/bin/sh
printf 'fix: edited subject\n\nEdited body from fake editor.\n' > "$1"
"#,
    )
    .expect("editor");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit after editing.","stage_paths":["edit.txt"],"commit":{"subject":"feat: initial subject","body_paragraphs":["Initial body."]},"alternatives":[]}"#,
    );

    let output = harness.run(
        ["n\ny\n"],
        &[("GIT_EDITOR", editor_script.to_string_lossy().as_ref())],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let subject = harness.git(["log", "-1", "--pretty=%s"]).expect("git log");
    assert_eq!(subject.trim(), "fix: edited subject");

    let body = harness
        .git(["log", "-1", "--pretty=%b"])
        .expect("git log body");
    assert!(body.contains("Edited body from fake editor."));
}

#[test]
fn installed_schema_path_is_passed_to_codex_exec() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("schema.txt", "schema\n").expect("file");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit.","stage_paths":["schema.txt"],"commit":{"subject":"feat: capture schema","body_paragraphs":[]},"alternatives":[]}"#,
    );

    let output = harness.run(["y\n"], &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected_schema = harness
        .home_dir
        .join(".local/share/codex-commit/commit-proposal.schema.json");
    let captured_schema =
        fs::read_to_string(harness.schema_capture_path()).expect("schema capture");
    assert_eq!(
        captured_schema.trim(),
        expected_schema.display().to_string()
    );
}

struct TestHarness {
    root: tempfile::TempDir,
    repo_dir: PathBuf,
    home_dir: PathBuf,
    stub_dir: PathBuf,
}

impl TestHarness {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let root = tempdir()?;
        let repo_dir = root.path().join("repo");
        let home_dir = root.path().join("home");
        let stub_dir = root.path().join("bin");

        fs::create_dir_all(&repo_dir)?;
        fs::create_dir_all(&stub_dir)?;
        fs::create_dir_all(home_dir.join(".local/share/codex-commit"))?;

        fs::write(
            home_dir.join(".local/share/codex-commit/SKILL.md"),
            "skill body",
        )?;
        fs::write(
            home_dir.join(".local/share/codex-commit/commit-proposal.schema.json"),
            r#"{"type":"object"}"#,
        )?;

        let harness = Self {
            root,
            repo_dir,
            home_dir,
            stub_dir,
        };

        harness.write_codex_stub()?;
        harness.git(["init"])?;
        harness.git(["config", "user.name", "Test User"])?;
        harness.git(["config", "user.email", "test@example.com"])?;

        Ok(harness)
    }

    fn commit_initial_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.write_file("README.md", "initial\n")?;
        self.git(["add", "--", "README.md"])?;
        self.git(["commit", "-m", "chore: initial commit"])?;
        Ok(())
    }

    fn write_file(
        &self,
        relative_path: &str,
        contents: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.repo_dir.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    fn set_stub_proposal(&self, proposal_json: &str) {
        fs::write(self.stub_proposal_path(), proposal_json).expect("stub proposal");
    }

    fn run<'a>(
        &self,
        stdin_chunks: impl IntoIterator<Item = &'a str>,
        extra_env: &[(&str, &str)],
    ) -> Output {
        let mut command = Command::new(BINARY_PATH);
        command.current_dir(&self.repo_dir);
        command.env("HOME", &self.home_dir);
        command.env(
            "PATH",
            format!(
                "{}:{}",
                self.stub_dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
        command.env("STUB_PROPOSAL_FILE", self.stub_proposal_path());
        command.env("STUB_SCHEMA_CAPTURE", self.schema_capture_path());
        command.env("GIT_AUTHOR_NAME", "Test User");
        command.env("GIT_AUTHOR_EMAIL", "test@example.com");
        command.env("GIT_COMMITTER_NAME", "Test User");
        command.env("GIT_COMMITTER_EMAIL", "test@example.com");
        for (key, value) in extra_env {
            command.env(key, value);
        }
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().expect("spawn binary");
        if let Some(stdin) = child.stdin.as_mut() {
            for chunk in stdin_chunks {
                stdin.write_all(chunk.as_bytes()).expect("write stdin");
            }
        }
        child.wait_with_output().expect("wait output")
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn write_codex_stub(&self) -> Result<(), Box<dyn std::error::Error>> {
        let script = self.stub_dir.join("codex");
        write_executable(
            &script,
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "codex-stub 1.0.0"
  exit 0
fi
if [ "$1" != "exec" ]; then
  echo "unexpected command: $1" >&2
  exit 1
fi
shift
schema=""
output=""
prompt=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --ephemeral)
      shift
      ;;
    --sandbox)
      shift 2
      ;;
    -c)
      shift 2
      ;;
    --output-schema)
      schema="$2"
      shift 2
      ;;
    -o)
      output="$2"
      shift 2
      ;;
    *)
      prompt="$1"
      shift
      ;;
  esac
done
if [ ! -f "$schema" ]; then
  echo "schema not found: $schema" >&2
  exit 7
fi
cp "$STUB_PROPOSAL_FILE" "$output"
printf '%s\n' "$schema" > "$STUB_SCHEMA_CAPTURE"
printf '%s\n' "$prompt" > "${STUB_SCHEMA_CAPTURE}.prompt"
echo "codex stub ran"
"#,
        )?;
        Ok(())
    }

    fn stub_proposal_path(&self) -> PathBuf {
        self.root.path().join("stub-proposal.json")
    }

    fn schema_capture_path(&self) -> PathBuf {
        self.root.path().join("schema-path.txt")
    }
}

fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
