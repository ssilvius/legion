use crate::db::Database;
use crate::error::Result;
use crate::search::SearchIndex;

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
    pub text: String,
    pub score: f32,
    pub created_at: String,
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

    let mut reflections = Vec::with_capacity(search_results.len());

    for sr in &search_results {
        // Skip silently if the reflection exists in the index but not in the DB
        if let Some(reflection) = db.get_reflection_by_id(&sr.id)? {
            reflections.push(RecalledReflection {
                id: reflection.id,
                text: reflection.text,
                score: sr.score,
                created_at: reflection.created_at,
            });
        }
    }

    Ok(RecallResult {
        reflections,
        query: context.to_owned(),
        repo: repo.to_owned(),
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
                    text: "mapping rules are fragile".into(),
                    score: 0.87,
                    created_at: "2026-03-05T00:00:00Z".into(),
                },
                RecalledReflection {
                    id: "id-2".into(),
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
}
