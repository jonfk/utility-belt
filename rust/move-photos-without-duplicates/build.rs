use std::env;
use std::process::Command;

fn main() {
    // Ensure changes in this crate rerun the build script.
    println!("cargo:rerun-if-changed=build.rs");

    let hash = latest_directory_hash().unwrap_or_else(|| "unavailable".to_string());
    println!("cargo:rustc-env=GIT_HASH={hash}");

    // If Cargo is invoked outside of a git repository (e.g., from a source archive),
    // we still succeed with the fallback value above.
}

fn latest_directory_hash() -> Option<String> {
    let current_dir = env::var("CARGO_MANIFEST_DIR").ok()?;

    let output = Command::new("git")
        .args(["log", "-1", "--pretty=format:%h", "--", "."])
        .current_dir(current_dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let hash = String::from_utf8(output.stdout).ok()?;
    let trimmed = hash.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
