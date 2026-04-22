use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use super::{AssetPaths, default_asset_dir_for_home};

#[test]
fn builds_default_asset_dir_under_home() {
    let dir = default_asset_dir_for_home("/tmp/test-home".as_ref());

    assert_eq!(
        dir,
        PathBuf::from("/tmp/test-home/.local/share/codex-commit")
    );
}

#[test]
fn validates_existing_assets() {
    let home = tempdir().expect("tempdir");
    let assets = AssetPaths::from_home(home.path());

    fs::create_dir_all(&assets.base_dir).expect("asset dir");
    fs::write(&assets.schema_path, "{}").expect("schema");

    assets.validate().expect("assets should validate");
}

#[test]
fn missing_asset_error_reports_path() {
    let home = tempdir().expect("tempdir");
    let assets = AssetPaths::from_home(home.path());
    fs::create_dir_all(&assets.base_dir).expect("asset dir");

    let report = assets.validate().expect_err("validation should fail");
    let rendered = format!("{report:?}");

    assert!(rendered.contains("Schema file not found"));
    assert!(rendered.contains(&assets.schema_path.display().to_string()));
}
