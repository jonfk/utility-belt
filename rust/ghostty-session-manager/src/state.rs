use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::info_span;

use crate::error::AppError;

const STATE_FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default = "state_file_version")]
    pub version: u32,
    #[serde(default)]
    pub projects: BTreeMap<String, ProjectStateRecord>,
}

impl Default for StateFile {
    fn default() -> Self {
        Self::empty()
    }
}

impl StateFile {
    pub fn empty() -> Self {
        Self {
            version: STATE_FILE_VERSION,
            projects: BTreeMap::new(),
        }
    }

    pub fn record_project_access(
        &mut self,
        project_path: &Path,
        accessed_at: Timestamp,
    ) -> Result<(), Report<AppError>> {
        let canonical_key = StateStore::canonical_project_key(project_path)?;
        self.projects.insert(
            canonical_key,
            ProjectStateRecord {
                last_accessed_at: accessed_at,
            },
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectStateRecord {
    pub last_accessed_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn from_default_path() -> Result<Self, Report<AppError>> {
        Ok(Self {
            path: default_state_file_path()?,
        })
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<StateFile, Report<AppError>> {
        let span = info_span!("state.load", path = self.path.display().to_string());
        let _enter = span.enter();
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(StateFile::empty());
            }
            Err(error) => {
                return Err(Report::new(AppError::State)
                    .attach(format!(
                        "Failed to read state file at {}",
                        self.path.display()
                    ))
                    .attach(error.to_string()));
            }
        };

        if contents.trim().is_empty() {
            return Ok(StateFile::empty());
        }

        serde_json::from_str(&contents)
            .change_context(AppError::State)
            .attach_with(|| format!("Failed to parse state file at {}", self.path.display()))
    }

    pub fn save(&self, state: &StateFile) -> Result<(), Report<AppError>> {
        let span = info_span!(
            "state.save",
            path = self.path.display().to_string(),
            projects = state.projects.len()
        );
        let _enter = span.enter();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .change_context(AppError::State)
                .attach_with(|| {
                    format!("Failed to create state file directory {}", parent.display())
                })?;
        }

        let rendered = serde_json::to_string_pretty(state)
            .change_context(AppError::State)
            .attach("Failed to serialize state file as JSON")?;

        fs::write(&self.path, format!("{rendered}\n"))
            .change_context(AppError::State)
            .attach_with(|| format!("Failed to write state file at {}", self.path.display()))?;

        Ok(())
    }

    pub fn canonical_project_key(path: &Path) -> Result<String, Report<AppError>> {
        let canonical_path = path
            .canonicalize()
            .change_context(AppError::State)
            .attach_with(|| format!("Failed to canonicalize project path {}", path.display()))?;

        canonical_path.to_str().map(str::to_owned).ok_or_else(|| {
            Report::new(AppError::State).attach(format!(
                "Canonical project path is not valid UTF-8: {}",
                canonical_path.display()
            ))
        })
    }
}

fn state_file_version() -> u32 {
    STATE_FILE_VERSION
}

fn default_state_file_path() -> Result<PathBuf, Report<AppError>> {
    let home = env::var_os("HOME").ok_or_else(|| {
        Report::new(AppError::State).attach("HOME is not set; cannot resolve state file path")
    })?;

    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("ghostty-session-manager")
        .join("state.json"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use jiff::Timestamp;

    use super::{ProjectStateRecord, StateFile, StateStore};

    #[test]
    fn missing_file_loads_as_empty_state() {
        let temp_dir = unique_test_dir();
        let store = StateStore::from_path(temp_dir.join("state.json"));

        let state = store.load().expect("missing file should be empty");

        assert_eq!(state, StateFile::empty());
    }

    #[test]
    fn empty_file_loads_as_empty_state() {
        let temp_dir = unique_test_dir();
        let path = temp_dir.join("state.json");
        fs::create_dir_all(&temp_dir).expect("temp dir should exist");
        fs::write(&path, "").expect("empty file should be written");
        let store = StateStore::from_path(path);

        let state = store.load().expect("empty file should be empty");

        assert_eq!(state, StateFile::empty());
    }

    #[test]
    fn whitespace_only_file_loads_as_empty_state() {
        let temp_dir = unique_test_dir();
        let path = temp_dir.join("state.json");
        fs::create_dir_all(&temp_dir).expect("temp dir should exist");
        fs::write(&path, "  \n\t").expect("whitespace file should be written");
        let store = StateStore::from_path(path);

        let state = store.load().expect("whitespace file should be empty");

        assert_eq!(state, StateFile::empty());
    }

    #[test]
    fn malformed_json_returns_state_error() {
        let temp_dir = unique_test_dir();
        let path = temp_dir.join("state.json");
        fs::create_dir_all(&temp_dir).expect("temp dir should exist");
        fs::write(&path, "{not json").expect("malformed file should be written");
        let store = StateStore::from_path(path);

        let report = store.load().expect_err("malformed json should fail");

        assert!(format!("{report:?}").contains("Failed to parse state file"));
    }

    #[test]
    fn save_creates_parent_directories() {
        let temp_dir = unique_test_dir();
        let path = temp_dir.join("nested").join("state.json");
        let store = StateStore::from_path(&path);

        assert_eq!(store.path(), path.as_path());

        store
            .save(&sample_state_file())
            .expect("save should create parent dirs");

        assert!(path.exists());
    }

    #[test]
    fn save_and_load_round_trip_projects_and_timestamps() {
        let temp_dir = unique_test_dir();
        let path = temp_dir.join("state.json");
        let store = StateStore::from_path(path);
        let expected = sample_state_file();

        store.save(&expected).expect("state should save");
        let loaded = store.load().expect("state should load");

        assert_eq!(loaded, expected);
    }

    #[test]
    fn save_writes_pretty_printed_json_with_version_one() {
        let temp_dir = unique_test_dir();
        let path = temp_dir.join("state.json");
        let store = StateStore::from_path(&path);

        store
            .save(&sample_state_file())
            .expect("state should save cleanly");

        let rendered = fs::read_to_string(path).expect("state file should be readable");
        assert!(rendered.contains("\"version\": 1"));
        assert!(rendered.contains("\n  \"projects\": {"));
        assert!(rendered.contains("\"last_accessed_at\": \"2026-04-15T12:00:00Z\""));
    }

    #[test]
    fn record_project_access_creates_new_project_record() {
        let temp_dir = unique_test_dir();
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");
        let accessed_at = parse_timestamp("2026-04-16T09:30:00Z");

        let mut state = StateFile::empty();
        state
            .record_project_access(&project_dir, accessed_at)
            .expect("recording access should succeed");

        let key = project_dir
            .canonicalize()
            .expect("project dir should canonicalize")
            .display()
            .to_string();
        assert_eq!(
            state.projects.get(&key),
            Some(&ProjectStateRecord {
                last_accessed_at: accessed_at,
            })
        );
    }

    #[test]
    fn record_project_access_updates_existing_project_record() {
        let temp_dir = unique_test_dir();
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");
        let key = project_dir
            .canonicalize()
            .expect("project dir should canonicalize")
            .display()
            .to_string();
        let mut state = StateFile::empty();
        state.projects.insert(
            key.clone(),
            ProjectStateRecord {
                last_accessed_at: parse_timestamp("2026-04-15T12:00:00Z"),
            },
        );

        let updated_at = parse_timestamp("2026-04-16T09:30:00Z");
        state
            .record_project_access(&project_dir, updated_at)
            .expect("recording access should update existing project");

        assert_eq!(
            state.projects.get(&key),
            Some(&ProjectStateRecord {
                last_accessed_at: updated_at,
            })
        );
    }

    #[test]
    fn canonical_project_key_uses_canonical_absolute_path() {
        let temp_dir = unique_test_dir();
        let project_dir = temp_dir.join("project");
        fs::create_dir_all(&project_dir).expect("project dir should exist");
        let nested = project_dir.join(".").join("subdir").join("..");
        fs::create_dir_all(project_dir.join("subdir")).expect("subdir should exist");

        let key = StateStore::canonical_project_key(&nested).expect("path should canonicalize");
        let expected = project_dir
            .canonicalize()
            .expect("expected path should canonicalize");

        assert_eq!(key, expected.display().to_string());
    }

    fn sample_state_file() -> StateFile {
        let mut state = StateFile::empty();
        state.projects.insert(
            "/Users/example/src/project-a".to_owned(),
            ProjectStateRecord {
                last_accessed_at: parse_timestamp("2026-04-15T12:00:00Z"),
            },
        );
        state
    }

    fn parse_timestamp(input: &str) -> Timestamp {
        input.parse().expect("timestamp fixture should parse")
    }

    fn unique_test_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "ghostty-session-manager-tests-{}-{}",
            timestamp, counter
        ));

        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale temp dir should be removable");
        }

        dir
    }
}
