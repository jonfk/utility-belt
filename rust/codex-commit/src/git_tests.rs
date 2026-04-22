use super::{sorted_paths, staged_sets_match};

#[test]
fn sorts_paths_for_stable_comparisons() {
    let sorted = sorted_paths(&["b.rs".into(), "a.rs".into()]);

    assert_eq!(sorted, vec!["a.rs", "b.rs"]);
}

#[test]
fn staged_set_comparison_ignores_order() {
    assert!(staged_sets_match(
        &["src/main.rs".into(), "README.md".into()],
        &["README.md".into(), "src/main.rs".into()]
    ));
}

#[test]
fn staged_set_comparison_detects_mismatch() {
    assert!(!staged_sets_match(
        &["src/main.rs".into()],
        &["README.md".into()]
    ));
}
