use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use error_stack::{Report, ResultExt};

use crate::error::{AppError, AppResult};

pub fn ensure_codex_available() -> AppResult<()> {
    let output = Command::new("codex")
        .arg("--version")
        .output()
        .change_context(AppError::RepoEnvironment)
        .attach("Failed to locate `codex` on PATH")?;

    if output.status.success() {
        return Ok(());
    }

    Err(Report::new(AppError::RepoEnvironment).attach(format!(
        "`codex --version` failed: {}",
        stderr_text(&output.stderr)
    )))
}

pub fn run_codex(
    prompt: &str,
    schema_path: &Path,
    output_path: &Path,
    log_path: &Path,
) -> AppResult<()> {
    let mut child = Command::new("codex")
        .arg("exec")
        .arg("--ephemeral")
        .arg("--sandbox")
        .arg("read-only")
        .arg("-c")
        .arg("model_reasoning_effort=\"low\"")
        .arg("--output-schema")
        .arg(schema_path)
        .arg("-o")
        .arg(output_path)
        .arg(prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .change_context(AppError::Codex)
        .attach("Failed to start `codex exec`")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Report::new(AppError::Codex).attach("Failed to capture codex stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Report::new(AppError::Codex).attach("Failed to capture codex stderr"))?;

    let log_file = File::create(log_path)
        .change_context(AppError::Codex)
        .attach(format!(
            "Failed to write log file at {}",
            log_path.display()
        ))?;
    let log_writer = Arc::new(Mutex::new(log_file));

    let stdout_capture = spawn_stream_thread(stdout, Arc::clone(&log_writer), StreamTarget::Stdout);
    let stderr_capture = spawn_stream_thread(stderr, Arc::clone(&log_writer), StreamTarget::Stderr);

    let status = child
        .wait()
        .change_context(AppError::Codex)
        .attach("Failed while waiting for `codex exec` to finish")?;

    let _stdout = join_stream_thread(stdout_capture, StreamTarget::Stdout)?;
    let stderr = join_stream_thread(stderr_capture, StreamTarget::Stderr)?;

    io::stdout()
        .flush()
        .change_context(AppError::Codex)
        .attach("Failed to flush forwarded codex stdout")?;
    io::stderr()
        .flush()
        .change_context(AppError::Codex)
        .attach("Failed to flush forwarded codex stderr")?;

    if status.success() {
        return Ok(());
    }

    Err(Report::new(AppError::Codex).attach(format!(
        "`codex exec` failed. See log at {}. stderr: {}",
        log_path.display(),
        stderr_text(&stderr)
    )))
}

fn spawn_stream_thread<R>(
    mut reader: R,
    log_writer: Arc<Mutex<File>>,
    target: StreamTarget,
) -> thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut stream = target.writer();

        {
            let mut log = log_writer
                .lock()
                .map_err(|_| io::Error::other("failed to lock codex log writer"))?;
            log.write_all(target.header())?;
            log.flush()?;
        }

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];
            stream.write_all(chunk)?;
            stream.flush()?;
            captured.extend_from_slice(chunk);

            let mut log = log_writer
                .lock()
                .map_err(|_| io::Error::other("failed to lock codex log writer"))?;
            log.write_all(chunk)?;
            log.flush()?;
        }

        if !captured.ends_with(b"\n") {
            let mut log = log_writer
                .lock()
                .map_err(|_| io::Error::other("failed to lock codex log writer"))?;
            log.write_all(b"\n")?;
            log.flush()?;
        }

        Ok(captured)
    })
}

fn join_stream_thread(
    handle: thread::JoinHandle<io::Result<Vec<u8>>>,
    target: StreamTarget,
) -> AppResult<Vec<u8>> {
    let captured = handle.join().map_err(|_| {
        Report::new(AppError::Codex).attach(format!(
            "Failed while joining codex {} stream thread",
            target.label()
        ))
    })?;

    captured.change_context(AppError::Codex).attach(format!(
        "Failed while streaming codex {} output",
        target.label()
    ))
}

#[derive(Clone, Copy)]
enum StreamTarget {
    Stdout,
    Stderr,
}

impl StreamTarget {
    fn header(self) -> &'static [u8] {
        match self {
            Self::Stdout => b"[stdout]\n",
            Self::Stderr => b"[stderr]\n",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }

    fn writer(self) -> Box<dyn Write> {
        match self {
            Self::Stdout => Box::new(io::stdout()),
            Self::Stderr => Box::new(io::stderr()),
        }
    }
}

fn stderr_text(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        "codex returned a non-zero exit status".to_string()
    } else {
        text
    }
}
