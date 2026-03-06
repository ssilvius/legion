use crate::db::Database;
use crate::error::Result;
use crate::search::{SearchIndex, SearchResult};

/// A set of recalled reflections matching a query within a repo.
#[derive(Debug, serde::Serialize)]
pub struct RecallResult {
    pub reflections: Vec<RecalledReflection>,
    pub query: String,
    pub repo: String,
}

/// A single recalled reflection with its BM25 relevance score.
#[derive(Debug, serde::Serialize)]
pub struct RecalledReflection {
    pub id: String,
    pub repo: String,
    pub text: String,
    pub score: f32,
    pub created_at: String,
}

/// Join search results with the database to produce full reflections.
///
/// Looks up each search hit in SQLite to retrieve the full reflection
/// data (text, repo, created_at). Missing entries (index/DB desync)
/// are skipped silently.
fn join_search_results(
    db: &Database,
    search_results: &[SearchResult],
) -> Result<Vec<RecalledReflection>> {
    let mut reflections = Vec::with_capacity(search_results.len());

    for sr in search_results {
        if let Some(reflection) = db.get_reflection_by_id(&sr.id)? {
            reflections.push(RecalledReflection {
                id: reflection.id,
                repo: reflection.repo,
                text: reflection.text,
                score: sr.score,
                created_at: reflection.created_at,
            });
        }
    }

    Ok(reflections)
}

/// Query reflections relevant to the given context.
///
/// Searches the Tantivy index filtered by `repo` and ranked by BM25,
/// then joins each result with the SQLite database to retrieve full
/// reflection data (text, created_at). Missing reflections in the DB
/// (index/DB desync) are skipped silently.
///
/// Returns results ordered by descending relevance score.
pub fn recall(
    db: &Database,
    index: &SearchIndex,
    repo: &str,
    context: &str,
    limit: usize,
) -> Result<RecallResult> {
    let search_results = index.search(repo, context, limit)?;
    let reflections = join_search_results(db, &search_results)?;

    Ok(RecallResult {
        reflections,
        query: context.to_owned(),
        repo: repo.to_owned(),
    })
}

/// Return the most recent reflections for a repo, bypassing BM25 search.
///
/// Useful for session-start hooks where no meaningful search context
/// is available yet. Returns results ordered newest first.
pub fn recall_latest(db: &Database, repo: &str, limit: usize) -> Result<RecallResult> {
    let all = db.get_reflections_by_repo(repo)?;

    let reflections: Vec<RecalledReflection> = all
        .into_iter()
        .take(limit)
        .map(|r| RecalledReflection {
            id: r.id,
            repo: r.repo,
            text: r.text,
            score: 0.0,
            created_at: r.created_at,
        })
        .collect();

    Ok(RecallResult {
        reflections,
        query: "(latest)".to_owned(),
        repo: repo.to_owned(),
    })
}

/// Search reflections across all repositories for cross-agent consultation.
///
/// Uses `index.search_all()` (no repo filter) and joins with the database
/// to retrieve full reflection data including the originating repo.
/// Returns a `RecallResult` with `repo` set to "(all)".
pub fn consult(
    db: &Database,
    index: &SearchIndex,
    context: &str,
    limit: usize,
) -> Result<RecallResult> {
    let search_results = index.search_all(context, limit)?;
    let reflections = join_search_results(db, &search_results)?;

    Ok(RecallResult {
        reflections,
        query: context.to_owned(),
        repo: "(all)".to_owned(),
    })
}

/// Format recall results for Claude Code hook injection.
///
/// Produces concise, human-readable output. Returns an empty string
/// when there are no results.
pub fn format_for_hook(result: &RecallResult) -> String {
    if result.reflections.is_empty() {
        return String::new();
    }

    let mut output = format!("[Legion] Relevant reflections for {}:\n", result.repo);

    for r in &result.reflections {
        output.push_str(&format!("- {} (score: {:.2})\n", r.text, r.score));
    }

    output
}

/// Format recall results for cross-repo consultation output.
///
/// Includes repository attribution per line so agents can see where
/// each reflection originated. Returns an empty string when there
/// are no results.
pub fn format_for_consult(result: &RecallResult) -> String {
    if result.reflections.is_empty() {
        return String::new();
    }

    let mut output = String::from("[Legion] Cross-repo reflections:\n");

    for r in &result.reflections {
        output.push_str(&format!(
            "- [{}] {} (score: {:.2})\n",
            r.repo, r.text, r.score
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflect::reflect_from_text;
    use crate::testutil::test_storage;

    #[test]
    fn recall_returns_ranked_results() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(
            &db,
            &index,
            "kelex",
            "mapping rules are fragile with Zod types",
        )
        .expect("reflect 1");
        reflect_from_text(&db, &index, "kelex", "the CLI argument parser works fine")
            .expect("reflect 2");
        reflect_from_text(
            &db,
            &index,
            "kelex",
            "Zod schema introspection handles unions",
        )
        .expect("reflect 3");

        let result = recall(&db, &index, "kelex", "Zod type mapping", 5).expect("recall");
        assert!(result.reflections.len() >= 2);
        assert!(result.reflections[0].score >= result.reflections[1].score);
    }

    #[test]
    fn recall_empty_context_returns_empty() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "some reflection").expect("reflect");

        let result = recall(&db, &index, "kelex", "", 5).expect("recall");
        assert!(result.reflections.is_empty());
    }

    #[test]
    fn recall_respects_limit() {
        let (db, index, _dir) = test_storage();
        for i in 0..10 {
            reflect_from_text(&db, &index, "test", &format!("testing reflection {i}"))
                .expect("reflect");
        }

        let result = recall(&db, &index, "test", "testing", 3).expect("recall");
        assert_eq!(result.reflections.len(), 3);
    }

    #[test]
    fn recall_skips_missing_db_entries() {
        let (db, index, _dir) = test_storage();

        // Add directly to index without DB entry to simulate desync
        index
            .add("orphan-id", "kelex", "orphan reflection text")
            .expect("add to index");

        // Add a proper entry through reflect_from_text
        reflect_from_text(&db, &index, "kelex", "properly stored reflection").expect("reflect");

        let result = recall(&db, &index, "kelex", "reflection", 10).expect("recall");

        // Only the properly stored one should appear
        for r in &result.reflections {
            assert_ne!(r.id, "orphan-id");
        }
    }

    #[test]
    fn recall_filters_by_repo() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "Zod schema mapping").expect("reflect kelex");
        reflect_from_text(&db, &index, "rafters", "Zod token generation").expect("reflect rafters");

        let result = recall(&db, &index, "kelex", "Zod", 10).expect("recall");
        assert_eq!(result.reflections.len(), 1);
        assert!(result.reflections[0].text.contains("mapping"));
    }

    #[test]
    fn recall_populates_metadata() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "test reflection").expect("reflect");

        let result = recall(&db, &index, "kelex", "test", 5).expect("recall");
        assert_eq!(result.repo, "kelex");
        assert_eq!(result.query, "test");
    }

    #[test]
    fn format_for_hook_produces_readable_output() {
        let result = RecallResult {
            query: "Zod mapping".into(),
            repo: "kelex".into(),
            reflections: vec![RecalledReflection {
                id: "test-id".into(),
                repo: "kelex".into(),
                text: "mapping rules are fragile".into(),
                score: 0.87,
                created_at: "2026-03-05T00:00:00Z".into(),
            }],
        };
        let output = format_for_hook(&result);
        assert!(output.contains("mapping rules are fragile"));
        assert!(output.contains("kelex"));
        assert!(output.contains("0.87"));
    }

    #[test]
    fn format_for_hook_multiple_results() {
        let result = RecallResult {
            query: "Zod mapping".into(),
            repo: "kelex".into(),
            reflections: vec![
                RecalledReflection {
                    id: "id-1".into(),
                    repo: "kelex".into(),
                    text: "mapping rules are fragile".into(),
                    score: 0.87,
                    created_at: "2026-03-05T00:00:00Z".into(),
                },
                RecalledReflection {
                    id: "id-2".into(),
                    repo: "kelex".into(),
                    text: "discriminated unions hide complexity".into(),
                    score: 0.62,
                    created_at: "2026-03-05T00:00:00Z".into(),
                },
            ],
        };
        let output = format_for_hook(&result);
        assert!(output.contains("mapping rules are fragile"));
        assert!(output.contains("discriminated unions hide complexity"));
        assert!(output.contains("[Legion]"));
    }

    #[test]
    fn format_for_hook_empty_results() {
        let result = RecallResult {
            query: "nothing".into(),
            repo: "kelex".into(),
            reflections: vec![],
        };
        let output = format_for_hook(&result);
        assert!(output.is_empty() || output.contains("No relevant reflections"));
    }

    #[test]
    fn consult_searches_across_repos() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "Zod schema mapping rules").expect("reflect kelex");
        reflect_from_text(&db, &index, "rafters", "token generation pipeline")
            .expect("reflect rafters");
        reflect_from_text(&db, &index, "platform", "Zod validation at the edge")
            .expect("reflect platform");

        let result = consult(&db, &index, "Zod", 10).expect("consult");
        // Should match kelex and platform but not rafters
        assert!(result.reflections.len() >= 2);
        let repos: Vec<&str> = result.reflections.iter().map(|r| r.repo.as_str()).collect();
        assert!(repos.contains(&"kelex"));
        assert!(repos.contains(&"platform"));
    }

    #[test]
    fn consult_includes_repo_attribution() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "schema introspection logic").expect("reflect");

        let result = consult(&db, &index, "schema", 5).expect("consult");
        assert_eq!(result.reflections.len(), 1);
        assert_eq!(result.reflections[0].repo, "kelex");
        assert_eq!(result.repo, "(all)");
    }

    #[test]
    fn consult_empty_context_returns_empty() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "some reflection text").expect("reflect");

        let result = consult(&db, &index, "", 5).expect("consult");
        assert!(result.reflections.is_empty());
    }

    #[test]
    fn format_for_consult_includes_repo_per_line() {
        let result = RecallResult {
            query: "schema".into(),
            repo: "(all)".into(),
            reflections: vec![
                RecalledReflection {
                    id: "id-1".into(),
                    repo: "kelex".into(),
                    text: "schema introspection".into(),
                    score: 0.90,
                    created_at: "2026-03-05T00:00:00Z".into(),
                },
                RecalledReflection {
                    id: "id-2".into(),
                    repo: "platform".into(),
                    text: "schema validation".into(),
                    score: 0.75,
                    created_at: "2026-03-05T00:00:00Z".into(),
                },
            ],
        };
        let output = format_for_consult(&result);
        assert!(output.contains("[Legion] Cross-repo reflections:"));
        assert!(output.contains("[kelex]"));
        assert!(output.contains("[platform]"));
        assert!(output.contains("schema introspection"));
        assert!(output.contains("schema validation"));
        assert!(output.contains("0.90"));
        assert!(output.contains("0.75"));
    }

    #[test]
    fn format_for_consult_empty_results() {
        let result = RecallResult {
            query: "nothing".into(),
            repo: "(all)".into(),
            reflections: vec![],
        };
        let output = format_for_consult(&result);
        assert!(output.is_empty());
    }
}
