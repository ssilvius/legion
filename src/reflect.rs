use std::io::BufRead;
use std::path::Path;

use serde::Deserialize;

use crate::db::Database;
use crate::error::{LegionError, Result};
use crate::search::SearchIndex;

/// A single line from a Claude Code transcript JSONL file.
#[derive(Deserialize)]
struct TranscriptLine {
    role: String,
    content: String,
}

/// Store a reflection from direct text input.
///
/// Validates that text is non-empty, inserts into SQLite via
/// `db.insert_reflection()`, and adds to the Tantivy search index
/// via `index.add()`. Prints a confirmation message to stdout.
pub fn reflect_from_text(db: &Database, index: &SearchIndex, repo: &str, text: &str) -> Result<()> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(LegionError::NoReflectionInput);
    }

    let reflection = db.insert_reflection(repo, trimmed)?;
    index.add(&reflection.id, repo, trimmed)?;

    eprintln!("stored reflection for {} ({})", repo, reflection.id);

    Ok(())
}

/// Extract and store a reflection from a transcript JSONL file.
///
/// Reads the file line by line. Each line is expected to be JSON with
/// "role" and "content" fields. Malformed lines are silently skipped.
/// The last line where `role == "assistant"` is used as the reflection
/// text.
///
/// Returns `LegionError::TranscriptNotFound` if the file does not exist.
/// Returns `LegionError::NoReflectionInput` if no assistant message is found.
pub fn reflect_from_transcript(
    db: &Database,
    index: &SearchIndex,
    repo: &str,
    transcript_path: &Path,
) -> Result<()> {
    if !transcript_path.exists() {
        return Err(LegionError::TranscriptNotFound(
            transcript_path.to_path_buf(),
        ));
    }

    let file = std::fs::File::open(transcript_path)?;
    let reader = std::io::BufReader::new(file);

    let mut last_assistant_content: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Malformed lines are skipped
        if let Ok(entry) = serde_json::from_str::<TranscriptLine>(trimmed)
            && entry.role == "assistant"
        {
            last_assistant_content = Some(entry.content);
        }
    }

    match last_assistant_content {
        Some(content) => reflect_from_text(db, index, repo, &content),
        None => Err(LegionError::NoReflectionInput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::test_storage;

    #[test]
    fn reflect_from_text_stores_in_db_and_index() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "kelex", "mapping rules are fragile").unwrap();

        let reflections = db.get_reflections_by_repo("kelex").unwrap();
        assert_eq!(reflections.len(), 1);
        assert_eq!(reflections[0].text, "mapping rules are fragile");

        let results = index.search("kelex", "mapping", 5).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn reflect_from_text_rejects_empty() {
        let (db, index, _dir) = test_storage();
        let err = reflect_from_text(&db, &index, "kelex", "").unwrap_err();
        assert!(matches!(err, LegionError::NoReflectionInput));
    }

    #[test]
    fn reflect_from_text_rejects_whitespace_only() {
        let (db, index, _dir) = test_storage();
        let err = reflect_from_text(&db, &index, "kelex", "   \n\t  ").unwrap_err();
        assert!(matches!(err, LegionError::NoReflectionInput));
    }

    #[test]
    fn reflect_from_transcript_extracts_last_assistant() {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("transcript.jsonl");
        std::fs::write(
            &transcript,
            r#"{"role":"user","content":"hello"}
{"role":"assistant","content":"first response"}
{"role":"user","content":"thanks"}
{"role":"assistant","content":"the important reflection"}
"#,
        )
        .unwrap();

        let (db, index, _idx_dir) = test_storage();
        reflect_from_transcript(&db, &index, "kelex", &transcript).unwrap();

        let reflections = db.get_reflections_by_repo("kelex").unwrap();
        assert_eq!(reflections[0].text, "the important reflection");
    }

    #[test]
    fn reflect_from_transcript_missing_file() {
        let (db, index, _dir) = test_storage();
        let err = reflect_from_transcript(
            &db,
            &index,
            "kelex",
            Path::new("/nonexistent/transcript.jsonl"),
        )
        .unwrap_err();
        assert!(matches!(err, LegionError::TranscriptNotFound(_)));
    }

    #[test]
    fn reflect_from_transcript_skips_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("transcript.jsonl");
        std::fs::write(
            &transcript,
            r#"not json at all
{"role":"user","content":"hello"}
{"broken json
{"role":"assistant","content":"survived malformed lines"}
"#,
        )
        .unwrap();

        let (db, index, _idx_dir) = test_storage();
        reflect_from_transcript(&db, &index, "kelex", &transcript).unwrap();

        let reflections = db.get_reflections_by_repo("kelex").unwrap();
        assert_eq!(reflections.len(), 1);
        assert_eq!(reflections[0].text, "survived malformed lines");
    }

    #[test]
    fn reflect_from_transcript_no_assistant_messages() {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("transcript.jsonl");
        std::fs::write(
            &transcript,
            r#"{"role":"user","content":"hello"}
{"role":"user","content":"anyone there?"}
"#,
        )
        .unwrap();

        let (db, index, _idx_dir) = test_storage();
        let err = reflect_from_transcript(&db, &index, "kelex", &transcript).unwrap_err();
        assert!(matches!(err, LegionError::NoReflectionInput));
    }

    #[test]
    fn compound_repo_stores_in_both() {
        let (db, index, _dir) = test_storage();
        reflect_from_text(&db, &index, "platform", "cross-repo knowledge").unwrap();
        reflect_from_text(&db, &index, "legion", "cross-repo knowledge").unwrap();

        let platform = db.get_reflections_by_repo("platform").unwrap();
        let legion = db.get_reflections_by_repo("legion").unwrap();

        assert_eq!(platform.len(), 1);
        assert_eq!(legion.len(), 1);
        assert_eq!(platform[0].text, legion[0].text);
        assert_ne!(platform[0].id, legion[0].id);

        let platform_search = index.search("platform", "cross-repo", 5).unwrap();
        let legion_search = index.search("legion", "cross-repo", 5).unwrap();
        assert_eq!(platform_search.len(), 1);
        assert_eq!(legion_search.len(), 1);
    }

    #[test]
    fn reflect_from_transcript_also_indexes_for_search() {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("transcript.jsonl");
        std::fs::write(
            &transcript,
            r#"{"role":"assistant","content":"binary parsing requires careful offset tracking"}
"#,
        )
        .unwrap();

        let (db, index, _idx_dir) = test_storage();
        reflect_from_transcript(&db, &index, "kelex", &transcript).unwrap();

        let results = index.search("kelex", "binary parsing", 5).unwrap();
        assert_eq!(results.len(), 1);

        let reflections = db.get_reflections_by_repo("kelex").unwrap();
        assert_eq!(results[0].id, reflections[0].id);
    }
}
