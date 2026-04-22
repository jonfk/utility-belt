use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};

use crate::error::{AppError, AppResult};

const SKILL_FILE_NAME: &str = "SKILL.md";
const SCHEMA_FILE_NAME: &str = "commit-proposal.schema.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPaths {
    pub base_dir: PathBuf,
    pub skill_path: PathBuf,
    pub schema_path: PathBuf,
}

impl AssetPaths {
    pub fn resolve() -> AppResult<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| Report::new(AppError::Assets).attach("HOME is not set"))?;

        let assets = Self::from_home(&home);
        assets.validate()?;
        Ok(assets)
    }

    pub fn from_home(home: &Path) -> Self {
        let base_dir = default_asset_dir_for_home(home);

        Self {
            skill_path: base_dir.join(SKILL_FILE_NAME),
            schema_path: base_dir.join(SCHEMA_FILE_NAME),
            base_dir,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if !self.skill_path.is_file() {
            return Err(Report::new(AppError::Assets).attach(format!(
                "Skill file not found at {}",
                self.skill_path.display()
            )));
        }

        if !self.schema_path.is_file() {
            return Err(Report::new(AppError::Assets).attach(format!(
                "Schema file not found at {}",
                self.schema_path.display()
            )));
        }

        Ok(())
    }

    pub fn read_skill_text(&self) -> AppResult<String> {
        fs::read_to_string(&self.skill_path)
            .change_context(AppError::Assets)
            .attach(format!(
                "Failed to read skill file at {}",
                self.skill_path.display()
            ))
    }
}

pub fn default_asset_dir_for_home(home: &Path) -> PathBuf {
    home.join(".local").join("share").join("codex-commit")
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
