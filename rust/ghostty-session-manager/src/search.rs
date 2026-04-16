use std::collections::BTreeMap;
use std::path::Path;

use frizbee::{Config, Match, Matcher};

use crate::state::ProjectStateRecord;

pub fn rank_project_keys(
    query: &str,
    projects: &BTreeMap<String, ProjectStateRecord>,
) -> Vec<String> {
    let normalized_query = normalize(query);
    if normalized_query.is_empty() {
        return rank_all_projects(projects);
    }

    let config = Config::default();
    let mut basename_matcher = Matcher::new(&normalized_query, &config);
    let mut full_path_matcher = Matcher::new(&normalized_query, &config);

    let mut ranked = projects
        .iter()
        .filter_map(|(project_key, record)| {
            let normalized_full_path = normalize(project_key);
            let normalized_basename = normalize(&basename_for_project_key(project_key));
            let basename_match = fuzzy_match(&mut basename_matcher, &normalized_basename);
            let full_path_match = fuzzy_match(&mut full_path_matcher, &normalized_full_path);

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
                exact_basename_match: normalized_basename == normalized_query,
                exact_full_path_match: normalized_full_path == normalized_query,
                basename_prefix_match: normalized_basename.starts_with(&normalized_query),
                best_hit_from_basename: best_hit_from_basename(
                    basename_match.as_ref(),
                    full_path_match.as_ref(),
                ),
                best_fuzzy_score,
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(compare_ranked_projects);
    ranked
        .into_iter()
        .map(|ranked_project| ranked_project.project_key)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedProject {
    project_key: String,
    last_accessed_at: jiff::Timestamp,
    exact_basename_match: bool,
    exact_full_path_match: bool,
    basename_prefix_match: bool,
    best_hit_from_basename: bool,
    best_fuzzy_score: u16,
}

fn rank_all_projects(projects: &BTreeMap<String, ProjectStateRecord>) -> Vec<String> {
    let mut project_keys = projects.keys().cloned().collect::<Vec<_>>();
    project_keys.sort_by(|left_key, right_key| {
        let left_record = &projects[left_key];
        let right_record = &projects[right_key];

        right_record
            .last_accessed_at
            .cmp(&left_record.last_accessed_at)
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

fn fuzzy_match(matcher: &mut Matcher, candidate: &str) -> Option<Match> {
    matcher.match_iter(&[candidate]).next()
}

fn best_hit_from_basename(basename_match: Option<&Match>, full_path_match: Option<&Match>) -> bool {
    match (basename_match, full_path_match) {
        (Some(basename_match), Some(full_path_match)) => {
            basename_match.score > full_path_match.score
        }
        (Some(_), None) => true,
        _ => false,
    }
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
        .then_with(|| right.last_accessed_at.cmp(&left.last_accessed_at))
        .then_with(|| left.project_key.cmp(&right.project_key))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jiff::Timestamp;

    use super::rank_project_keys;
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
            let actual = rank_project_keys(case.query, &projects);
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

        let ranked = rank_project_keys("project", &projects);

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

        let ranked = rank_project_keys("project", &projects);

        assert_eq!(
            ranked,
            vec![
                "/work/apps/a/project".to_owned(),
                "/work/apps/b/project".to_owned(),
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
            rank_project_keys("api", &projects),
            vec![
                "/Users/example/src/services/api".to_owned(),
                "/Users/example/src/services/api-gateway".to_owned(),
                "/Users/example/src/shared/api-clients".to_owned(),
            ]
        );
        assert_eq!(
            rank_project_keys("ghost", &projects),
            vec![
                "/Users/example/src/platform/ghostty-session-manager".to_owned(),
                "/Users/example/src/platform/ghostty-tools".to_owned(),
            ]
        );
        assert_eq!(
            rank_project_keys("platform/ghostty", &projects),
            vec![
                "/Users/example/src/platform/ghostty-session-manager".to_owned(),
                "/Users/example/src/platform/ghostty-tools".to_owned(),
            ]
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

    fn parse_timestamp(input: &str) -> Timestamp {
        input.parse().expect("timestamp fixture should parse")
    }
}
