use std::collections::BTreeMap;
use std::path::Path;

use frizbee::{Config, MatchIndices, Matcher};

use crate::state::ProjectStateRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMatchField {
    Basename,
    FullPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMatch {
    pub field: ProjectMatchField,
    pub char_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedProjectMatch {
    pub project_key: String,
    pub project_match: Option<ProjectMatch>,
}

pub fn rank_projects(
    query: &str,
    projects: &BTreeMap<String, ProjectStateRecord>,
    current_project_key: Option<&str>,
) -> Vec<RankedProjectMatch> {
    let normalized_query = normalize(query);
    if normalized_query.is_empty() {
        return rank_all_projects(projects, current_project_key)
            .into_iter()
            .map(|project_key| RankedProjectMatch {
                project_key,
                project_match: None,
            })
            .collect();
    }

    let config = Config::default();
    let mut basename_matcher = Matcher::new(&normalized_query, &config);
    let mut full_path_matcher = Matcher::new(&normalized_query, &config);

    let mut ranked = projects
        .iter()
        .filter_map(|(project_key, record)| {
            let normalized_full_path = normalize(project_key);
            let basename = basename_for_project_key(project_key);
            let normalized_basename = normalize(&basename);
            let basename_match = fuzzy_match_indices(&mut basename_matcher, &normalized_basename);
            let full_path_match =
                fuzzy_match_indices(&mut full_path_matcher, &normalized_full_path);

            let best_fuzzy_score = match (basename_match.as_ref(), full_path_match.as_ref()) {
                (Some(basename_match), Some(full_path_match)) => {
                    basename_match.score.max(full_path_match.score)
                }
                (Some(basename_match), None) => basename_match.score,
                (None, Some(full_path_match)) => full_path_match.score,
                (None, None) => return None,
            };

            Some(RankedProject {
                project_key: project_key.clone(),
                last_accessed_at: record.last_accessed_at,
                is_current_project: current_project_key == Some(project_key.as_str()),
                exact_basename_match: normalized_basename == normalized_query,
                exact_full_path_match: normalized_full_path == normalized_query,
                basename_prefix_match: normalized_basename.starts_with(&normalized_query),
                best_hit_from_basename: best_hit_from_basename(
                    basename_match.as_ref(),
                    full_path_match.as_ref(),
                ),
                best_fuzzy_score,
                basename_match,
                full_path_match,
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(compare_ranked_projects);
    ranked
        .into_iter()
        .map(|ranked_project| RankedProjectMatch {
            project_key: ranked_project.project_key.clone(),
            project_match: preferred_project_match(&ranked_project),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedProject {
    project_key: String,
    last_accessed_at: jiff::Timestamp,
    is_current_project: bool,
    exact_basename_match: bool,
    exact_full_path_match: bool,
    basename_prefix_match: bool,
    best_hit_from_basename: bool,
    best_fuzzy_score: u16,
    basename_match: Option<MatchIndices>,
    full_path_match: Option<MatchIndices>,
}

fn rank_all_projects(
    projects: &BTreeMap<String, ProjectStateRecord>,
    current_project_key: Option<&str>,
) -> Vec<String> {
    let mut project_keys = projects.keys().cloned().collect::<Vec<_>>();
    project_keys.sort_by(|left_key, right_key| {
        let left_record = &projects[left_key];
        let right_record = &projects[right_key];
        let left_is_current = current_project_key == Some(left_key.as_str());
        let right_is_current = current_project_key == Some(right_key.as_str());

        left_is_current
            .cmp(&right_is_current)
            .then_with(|| {
                right_record
                    .last_accessed_at
                    .cmp(&left_record.last_accessed_at)
            })
            .then_with(|| left_key.cmp(right_key))
    });
    project_keys
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn basename_for_project_key(project_key: &str) -> String {
    Path::new(project_key)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| project_key.to_owned())
}

fn fuzzy_match_indices(matcher: &mut Matcher, candidate: &str) -> Option<MatchIndices> {
    matcher.match_iter_indices(&[candidate]).next()
}

fn best_hit_from_basename(
    basename_match: Option<&MatchIndices>,
    full_path_match: Option<&MatchIndices>,
) -> bool {
    match (basename_match, full_path_match) {
        (Some(basename_match), Some(full_path_match)) => {
            basename_match.score > full_path_match.score
        }
        (Some(_), None) => true,
        _ => false,
    }
}

fn preferred_project_match(ranked_project: &RankedProject) -> Option<ProjectMatch> {
    let basename = basename_for_project_key(&ranked_project.project_key);

    match (
        ranked_project.basename_match.as_ref(),
        ranked_project.full_path_match.as_ref(),
    ) {
        (Some(basename_match), Some(full_path_match))
            if basename_match.score >= full_path_match.score =>
        {
            Some(ProjectMatch {
                field: ProjectMatchField::Basename,
                char_indices: matched_char_indices(&basename, &basename_match.indices),
            })
        }
        (Some(basename_match), None) => Some(ProjectMatch {
            field: ProjectMatchField::Basename,
            char_indices: matched_char_indices(&basename, &basename_match.indices),
        }),
        (_, Some(full_path_match)) => Some(ProjectMatch {
            field: ProjectMatchField::FullPath,
            char_indices: matched_char_indices(
                &ranked_project.project_key,
                &full_path_match.indices,
            ),
        }),
        (None, None) => None,
    }
}

fn matched_char_indices(candidate: &str, matched_bytes: &[usize]) -> Vec<usize> {
    let lowered_byte_to_char = lowered_byte_to_char_index(candidate);
    let mut matched_chars = matched_bytes
        .iter()
        .filter_map(|byte_index| lowered_byte_to_char.get(*byte_index).copied())
        .collect::<Vec<_>>();
    matched_chars.sort_unstable();
    matched_chars.dedup();
    matched_chars
}

fn lowered_byte_to_char_index(candidate: &str) -> Vec<usize> {
    let mut lowered_byte_to_char = Vec::new();

    for (char_index, character) in candidate.chars().enumerate() {
        for lowered in character.to_lowercase() {
            let mut buffer = [0; 4];
            let encoded = lowered.encode_utf8(&mut buffer);
            for _ in 0..encoded.len() {
                lowered_byte_to_char.push(char_index);
            }
        }
    }

    lowered_byte_to_char
}

fn compare_ranked_projects(left: &RankedProject, right: &RankedProject) -> std::cmp::Ordering {
    right
        .exact_basename_match
        .cmp(&left.exact_basename_match)
        .then_with(|| right.exact_full_path_match.cmp(&left.exact_full_path_match))
        .then_with(|| right.basename_prefix_match.cmp(&left.basename_prefix_match))
        .then_with(|| {
            right
                .best_hit_from_basename
                .cmp(&left.best_hit_from_basename)
        })
        .then_with(|| right.best_fuzzy_score.cmp(&left.best_fuzzy_score))
        .then_with(|| left.is_current_project.cmp(&right.is_current_project))
        .then_with(|| right.last_accessed_at.cmp(&left.last_accessed_at))
        .then_with(|| left.project_key.cmp(&right.project_key))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jiff::Timestamp;

    use super::{ProjectMatchField, rank_projects};
    use crate::state::ProjectStateRecord;

    #[test]
    fn table_driven_ranking_cases() {
        let projects = BTreeMap::from([
            (
                "/work/apps/platform/api".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/archive/legacy-api".to_owned(),
                project_record("2026-04-16T08:00:00Z"),
            ),
            (
                "/work/apps/ops/deploy".to_owned(),
                project_record("2026-04-16T10:30:00Z"),
            ),
            (
                "/work/libs/deployment-tools".to_owned(),
                project_record("2026-04-16T09:30:00Z"),
            ),
            (
                "/work/services/internal/billing-worker".to_owned(),
                project_record("2026-04-16T07:00:00Z"),
            ),
        ]);

        let cases = [
            RankingCase {
                name: "exact basename beats deeper full path fuzzy match",
                query: "api",
                expected: vec!["/work/apps/platform/api", "/work/archive/legacy-api"],
            },
            RankingCase {
                name: "basename prefix beats weaker full path match",
                query: "depl",
                expected: vec!["/work/apps/ops/deploy", "/work/libs/deployment-tools"],
            },
            RankingCase {
                name: "partial path query matches intermediate directories",
                query: "internal/bill",
                expected: vec!["/work/services/internal/billing-worker"],
            },
            RankingCase {
                name: "empty query returns mru ordering",
                query: "   ",
                expected: vec![
                    "/work/apps/platform/api",
                    "/work/apps/ops/deploy",
                    "/work/libs/deployment-tools",
                    "/work/archive/legacy-api",
                    "/work/services/internal/billing-worker",
                ],
            },
            RankingCase {
                name: "no match query returns empty results",
                query: "zzz",
                expected: vec![],
            },
        ];

        for case in cases {
            let actual = ranked_keys(case.query, &projects, None);
            assert_eq!(actual, case.expected, "{}", case.name);
        }
    }

    #[test]
    fn mru_breaks_ties_for_equally_ranked_matches() {
        let projects = BTreeMap::from([
            (
                "/work/apps/newer/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/apps/older/project".to_owned(),
                project_record("2026-04-16T09:00:00Z"),
            ),
        ]);

        let ranked = ranked_keys("project", &projects, None);

        assert_eq!(
            ranked,
            vec![
                "/work/apps/newer/project".to_owned(),
                "/work/apps/older/project".to_owned(),
            ]
        );
    }

    #[test]
    fn canonical_project_key_breaks_complete_ties() {
        let projects = BTreeMap::from([
            (
                "/work/apps/a/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/apps/b/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
        ]);

        let ranked = ranked_keys("project", &projects, None);

        assert_eq!(
            ranked,
            vec![
                "/work/apps/a/project".to_owned(),
                "/work/apps/b/project".to_owned(),
            ]
        );
    }

    #[test]
    fn empty_query_demotes_current_project_below_non_current_projects() {
        let projects = BTreeMap::from([
            (
                "/work/apps/current".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/apps/other".to_owned(),
                project_record("2026-04-16T10:00:00Z"),
            ),
        ]);

        let ranked = ranked_keys("", &projects, Some("/work/apps/current"));

        assert_eq!(
            ranked,
            vec![
                "/work/apps/other".to_owned(),
                "/work/apps/current".to_owned(),
            ]
        );
    }

    #[test]
    fn equally_relevant_filtered_matches_prefer_non_current_project() {
        let projects = BTreeMap::from([
            (
                "/work/apps/current/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/apps/other/project".to_owned(),
                project_record("2026-04-16T10:00:00Z"),
            ),
        ]);

        let ranked = ranked_keys("project", &projects, Some("/work/apps/current/project"));

        assert_eq!(
            ranked,
            vec![
                "/work/apps/other/project".to_owned(),
                "/work/apps/current/project".to_owned(),
            ]
        );
    }

    #[test]
    fn stronger_current_match_still_beats_weaker_non_current_match() {
        let projects = BTreeMap::from([
            (
                "/work/apps/current/api".to_owned(),
                project_record("2026-04-16T09:00:00Z"),
            ),
            (
                "/work/archive/legacy-api".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
        ]);

        let ranked = ranked_keys("api", &projects, Some("/work/apps/current/api"));

        assert_eq!(
            ranked,
            vec![
                "/work/apps/current/api".to_owned(),
                "/work/archive/legacy-api".to_owned(),
            ]
        );
    }

    #[test]
    fn canonical_key_breaks_ties_after_current_project_penalty() {
        let projects = BTreeMap::from([
            (
                "/work/apps/current/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/apps/a/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/work/apps/b/project".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
        ]);

        let ranked = ranked_keys("project", &projects, Some("/work/apps/current/project"));

        assert_eq!(
            ranked,
            vec![
                "/work/apps/a/project".to_owned(),
                "/work/apps/b/project".to_owned(),
                "/work/apps/current/project".to_owned(),
            ]
        );
    }

    #[test]
    fn realistic_fixture_covers_exact_prefix_and_path_queries() {
        let projects = BTreeMap::from([
            (
                "/Users/example/src/client/mobile-app".to_owned(),
                project_record("2026-04-16T08:00:00Z"),
            ),
            (
                "/Users/example/src/client/web-app".to_owned(),
                project_record("2026-04-16T12:00:00Z"),
            ),
            (
                "/Users/example/src/platform/ghostty-session-manager".to_owned(),
                project_record("2026-04-16T11:00:00Z"),
            ),
            (
                "/Users/example/src/platform/ghostty-tools".to_owned(),
                project_record("2026-04-16T07:00:00Z"),
            ),
            (
                "/Users/example/src/services/api".to_owned(),
                project_record("2026-04-16T10:00:00Z"),
            ),
            (
                "/Users/example/src/services/api-gateway".to_owned(),
                project_record("2026-04-16T09:00:00Z"),
            ),
            (
                "/Users/example/src/shared/api-clients".to_owned(),
                project_record("2026-04-16T06:00:00Z"),
            ),
            (
                "/Users/example/src/tools/console-kit".to_owned(),
                project_record("2026-04-16T05:00:00Z"),
            ),
        ]);

        assert_eq!(
            ranked_keys("api", &projects, None),
            vec![
                "/Users/example/src/services/api".to_owned(),
                "/Users/example/src/services/api-gateway".to_owned(),
                "/Users/example/src/shared/api-clients".to_owned(),
            ]
        );
        assert_eq!(
            ranked_keys("ghost", &projects, None),
            vec![
                "/Users/example/src/platform/ghostty-session-manager".to_owned(),
                "/Users/example/src/platform/ghostty-tools".to_owned(),
            ]
        );
        assert_eq!(
            ranked_keys("platform/ghostty", &projects, None),
            vec![
                "/Users/example/src/platform/ghostty-session-manager".to_owned(),
                "/Users/example/src/platform/ghostty-tools".to_owned(),
            ]
        );
    }

    #[test]
    fn ranking_metadata_tracks_basename_match_indices() {
        let projects = BTreeMap::from([(
            "/work/apps/platform/api".to_owned(),
            project_record("2026-04-16T11:00:00Z"),
        )]);

        let ranked = rank_projects("api", &projects, None);

        assert_eq!(ranked.len(), 1);
        assert_eq!(
            ranked[0]
                .project_match
                .as_ref()
                .map(|project_match| project_match.field),
            Some(ProjectMatchField::Basename)
        );
        assert_eq!(
            matched_text(
                "api",
                &ranked[0].project_match.clone().unwrap().char_indices
            ),
            "api"
        );
    }

    #[test]
    fn ranking_metadata_tracks_full_path_match_indices() {
        let projects = BTreeMap::from([(
            "/work/services/internal/billing-worker".to_owned(),
            project_record("2026-04-16T07:00:00Z"),
        )]);

        let ranked = rank_projects("internal/bill", &projects, None);

        assert_eq!(ranked.len(), 1);
        assert_eq!(
            ranked[0]
                .project_match
                .as_ref()
                .map(|project_match| project_match.field),
            Some(ProjectMatchField::FullPath)
        );
        assert_eq!(
            matched_text(
                "/work/services/internal/billing-worker",
                &ranked[0].project_match.clone().unwrap().char_indices,
            ),
            "internal/bill"
        );
    }

    #[derive(Debug)]
    struct RankingCase<'a> {
        name: &'a str,
        query: &'a str,
        expected: Vec<&'a str>,
    }

    fn project_record(last_accessed_at: &str) -> ProjectStateRecord {
        ProjectStateRecord {
            last_accessed_at: parse_timestamp(last_accessed_at),
            last_seen_at: parse_timestamp("2026-04-16T12:30:00Z"),
            last_window_id: "window-1".to_owned(),
            last_window_name: Some("Workspace".to_owned()),
        }
    }

    fn matched_text(text: &str, char_indices: &[usize]) -> String {
        text.chars()
            .enumerate()
            .filter(|(char_index, _)| char_indices.contains(char_index))
            .map(|(_, character)| character)
            .collect()
    }

    fn ranked_keys(
        query: &str,
        projects: &BTreeMap<String, ProjectStateRecord>,
        current_project_key: Option<&str>,
    ) -> Vec<String> {
        rank_projects(query, projects, current_project_key)
            .into_iter()
            .map(|project_match| project_match.project_key)
            .collect()
    }

    fn parse_timestamp(input: &str) -> Timestamp {
        input.parse().expect("timestamp fixture should parse")
    }
}
