use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

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
    let sha = harness.git(["rev-parse", "HEAD"]).expect("git rev-parse");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_commit_summary_output(&stdout, sha.trim(), "feat: add src file", "src.txt");

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
    let sha = harness.git(["rev-parse", "HEAD"]).expect("git rev-parse");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_commit_summary_output(
        &stdout,
        sha.trim(),
        "feat: commit staged file",
        "staged.txt",
    );
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

    let captured_prompt =
        fs::read_to_string(harness.prompt_capture_path()).expect("prompt capture");
    assert!(captured_prompt.contains("# Git Commit Proposal"));
    assert!(captured_prompt.contains("Run `git status --short --branch`"));
    assert!(captured_prompt.contains("package-lock.json"));
    assert!(!captured_prompt.contains("name: git-commit-proposal"));
    assert!(!captured_prompt.contains("\n---\n"));
}

#[test]
fn pseudo_tty_run_falls_back_to_plain_text_and_still_commits() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("tty.txt", "tty\n").expect("file");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit with the inline UI.","stage_paths":["tty.txt"],"commit":{"subject":"feat: commit from tui","body_paragraphs":["Rendered through ratatui."]},"alternatives":[]}"#,
    );

    let output = harness.run_tty("y\n", Some("Commit with this message? [Y/n] "), false, &[]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("codex-commit: terminal UI unavailable; falling back to plain text"));
    assert!(stdout.contains("Commit with this message? [Y/n]"));
    let sha = harness.git(["rev-parse", "HEAD"]).expect("git rev-parse");
    assert_commit_summary_output(&stdout, sha.trim(), "feat: commit from tui", "tty.txt");

    let subject = harness.git(["log", "-1", "--pretty=%s"]).expect("git log");
    assert_eq!(subject.trim(), "feat: commit from tui");
}

#[test]
fn codex_output_is_streamed_before_codex_commit_finishes() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("stream.txt", "stream\n").expect("file");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to commit streamed output.","stage_paths":["stream.txt"],"commit":{"subject":"feat: stream codex output","body_paragraphs":[]},"alternatives":[]}"#,
    );

    let mut running = harness.spawn(
        ["y\n"],
        &[
            ("STUB_CODEX_STREAM_MODE", "1"),
            ("STUB_CODEX_STREAM_DELAY", "0.5"),
        ],
    );

    let mut saw_progress = false;
    for _ in 0..20 {
        let output_so_far = running.combined_string();
        if output_so_far.contains("codex stub: progress 1") {
            saw_progress = true;
            assert!(!output_so_far.contains("Commit with this message? [Y/n]"));
            assert!(running.child.try_wait().expect("try_wait").is_none());
            break;
        }

        if running.child.try_wait().expect("try_wait").is_some() {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    assert!(
        saw_progress,
        "expected live codex output before process completed"
    );

    let output = running.wait();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ctrl_d_from_tui_cancels_without_creating_a_commit() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness.write_file("cancel.txt", "cancel\n").expect("file");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to cancel from the inline UI.","stage_paths":["cancel.txt"],"commit":{"subject":"feat: should not commit","body_paragraphs":[]},"alternatives":[]}"#,
    );

    let output = harness.run_tty(
        "\x04",
        None,
        true,
        &[
            ("STUB_CODEX_STREAM_MODE", "1"),
            ("STUB_CODEX_STREAM_DELAY", "0.1"),
        ],
    );
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let head_subject = harness.git(["log", "-1", "--pretty=%s"]).expect("git log");
    assert_eq!(head_subject.trim(), "chore: initial commit");
}

#[test]
fn ctrl_c_from_tui_exits_with_code_130() {
    let harness = TestHarness::new().expect("harness");
    harness.commit_initial_state().expect("initial commit");
    harness
        .write_file("interrupt.txt", "interrupt\n")
        .expect("file");

    harness.set_stub_proposal(
        r#"{"status":"ready","summary":"Ready to interrupt from the inline UI.","stage_paths":["interrupt.txt"],"commit":{"subject":"feat: should not commit","body_paragraphs":[]},"alternatives":[]}"#,
    );

    let output = harness.run_tty(
        "\x03",
        None,
        true,
        &[
            ("STUB_CODEX_STREAM_MODE", "1"),
            ("STUB_CODEX_STREAM_DELAY", "0.1"),
        ],
    );
    assert_eq!(output.status.code(), Some(130));

    let head_subject = harness.git(["log", "-1", "--pretty=%s"]).expect("git log");
    assert_eq!(head_subject.trim(), "chore: initial commit");
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
        let child = self.spawn(stdin_chunks, extra_env);
        child.wait()
    }

    fn spawn<'a>(
        &self,
        stdin_chunks: impl IntoIterator<Item = &'a str>,
        extra_env: &[(&str, &str)],
    ) -> RunningCommand {
        let mut command = self.base_command(BINARY_PATH, extra_env);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().expect("spawn binary");
        if let Some(stdin) = child.stdin.as_mut() {
            for chunk in stdin_chunks {
                stdin.write_all(chunk.as_bytes()).expect("write stdin");
            }
        }

        let stdout = child.stdout.take().expect("stdout pipe");
        let stderr = child.stderr.take().expect("stderr pipe");
        RunningCommand::new(child, stdout, stderr)
    }

    fn run_tty(
        &self,
        input: &str,
        wait_for: Option<&str>,
        respond_to_cursor_query: bool,
        extra_env: &[(&str, &str)],
    ) -> Output {
        let python = r#"import os, pty, select, signal, sys, time
cmd = sys.argv[1:]
pid, fd = pty.fork()
if pid == 0:
    os.execvpe(cmd[0], cmd, os.environ)
payload = os.environ.get("PTY_INPUT", "").encode()
trigger = os.environ.get("PTY_INPUT_TRIGGER", "")
reply_cursor = os.environ.get("PTY_REPLY_CURSOR", "") == "1"
captured = bytearray()
sent_input = False
cursor_replied_at = None
start = time.time()
deadline = start + 10
exit_code = 1
while True:
    if payload and not sent_input and time.time() - start >= 0.25:
        if trigger and trigger not in captured.decode(errors="ignore"):
            pass
        elif reply_cursor and cursor_replied_at is None:
            pass
        elif reply_cursor and time.time() - cursor_replied_at < 0.25:
            pass
        else:
            os.write(fd, payload)
            sent_input = True
    try:
        ready, _, _ = select.select([fd], [], [], 0.1)
        if ready:
            chunk = os.read(fd, 4096)
            if not chunk:
                break
            captured.extend(chunk)
            if reply_cursor and b'\x1b[6n' in chunk:
                os.write(fd, b'\x1b[1;1R')
                cursor_replied_at = time.time()
    except OSError:
        break
    waited_pid, status = os.waitpid(pid, os.WNOHANG)
    if waited_pid == pid:
        exit_code = os.waitstatus_to_exitcode(status)
        break
    if time.time() >= deadline:
        os.kill(pid, signal.SIGTERM)
        _, status = os.waitpid(pid, 0)
        exit_code = os.waitstatus_to_exitcode(status)
        sys.stderr.write("PTY timeout while waiting for codex-commit\n")
        break
sys.stdout.buffer.write(captured)
try:
    _, status = os.waitpid(pid, 0)
    exit_code = os.waitstatus_to_exitcode(status)
except ChildProcessError:
    pass
sys.exit(exit_code)
"#;

        let mut command = self.base_command("python3", extra_env);
        command.arg("-c").arg(python).arg(BINARY_PATH);
        command.env("PTY_INPUT", input);
        if let Some(marker) = wait_for {
            command.env("PTY_INPUT_TRIGGER", marker);
        }
        if respond_to_cursor_query {
            command.env("PTY_REPLY_CURSOR", "1");
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        command.output().expect("run in pty")
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
if [ "${STUB_CODEX_STREAM_MODE:-}" = "1" ]; then
  echo "codex stub: progress 1"
  sleep "${STUB_CODEX_STREAM_DELAY:-0.2}"
  echo "codex stub: progress 2" >&2
  sleep "${STUB_CODEX_STREAM_DELAY:-0.2}"
fi
cp "$STUB_PROPOSAL_FILE" "$output"
printf '%s\n' "$schema" > "$STUB_SCHEMA_CAPTURE"
printf '%s\n' "$prompt" > "${STUB_SCHEMA_CAPTURE}.prompt"
echo "codex stub ran"
"#,
        )?;
        Ok(())
    }

    fn base_command(&self, program: &str, extra_env: &[(&str, &str)]) -> Command {
        let mut command = Command::new(program);
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
        command
    }

    fn stub_proposal_path(&self) -> PathBuf {
        self.root.path().join("stub-proposal.json")
    }

    fn schema_capture_path(&self) -> PathBuf {
        self.root.path().join("schema-path.txt")
    }

    fn prompt_capture_path(&self) -> PathBuf {
        self.root.path().join("schema-path.txt.prompt")
    }
}

fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn assert_commit_summary_output(stdout: &str, sha: &str, subject: &str, path: &str) {
    assert!(
        stdout.contains(&format!("commit {sha}")),
        "stdout: {stdout}"
    );
    assert!(stdout.contains(subject), "stdout: {stdout}");
    assert!(stdout.contains(&format!("{path} |")), "stdout: {stdout}");
}

struct RunningCommand {
    child: Child,
    stdout: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stdout_handle: JoinHandle<()>,
    stderr_handle: JoinHandle<()>,
}

impl RunningCommand {
    fn new(child: Child, stdout: ChildStdout, stderr: ChildStderr) -> Self {
        let stdout_buffer = Arc::new(Mutex::new(Vec::new()));
        let stderr_buffer = Arc::new(Mutex::new(Vec::new()));

        let stdout_handle = spawn_reader(stdout, Arc::clone(&stdout_buffer));
        let stderr_handle = spawn_reader(stderr, Arc::clone(&stderr_buffer));

        Self {
            child,
            stdout: stdout_buffer,
            stderr: stderr_buffer,
            stdout_handle,
            stderr_handle,
        }
    }

    fn stdout_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout.lock().expect("stdout lock")).to_string()
    }

    fn combined_string(&self) -> String {
        format!(
            "{}{}",
            self.stdout_string(),
            String::from_utf8_lossy(&self.stderr.lock().expect("stderr lock"))
        )
    }

    fn wait(mut self) -> Output {
        let status = self.child.wait().expect("wait child");
        self.stdout_handle.join().expect("join stdout");
        self.stderr_handle.join().expect("join stderr");

        Output {
            status,
            stdout: self.stdout.lock().expect("stdout lock").clone(),
            stderr: self.stderr.lock().expect("stderr lock").clone(),
        }
    }
}

fn spawn_reader<R>(mut reader: R, target: Arc<Mutex<Vec<u8>>>) -> JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => target
                    .lock()
                    .expect("reader lock")
                    .extend_from_slice(&buffer[..bytes_read]),
                Err(_) => break,
            }
        }
    })
}
