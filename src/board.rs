use std::path::Path;

use crate::db::{Database, Reflection};
use crate::error::{LegionError, Result};
use crate::search::SearchIndex;

/// Store a board post from direct text input.
///
/// Like `reflect_from_text` but sets audience to "team" so the post
/// appears on the shared board visible to all agents.
pub fn post_from_text(db: &Database, index: &SearchIndex, repo: &str, text: &str) -> Result<()> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(LegionError::NoReflectionInput);
    }

    let reflection = db.insert_reflection(repo, trimmed, "team")?;
    index.add(&reflection.id, repo, trimmed)?;

    eprintln!("posted to board for {} ({})", repo, reflection.id);

    Ok(())
}

/// Extract and store a board post from a transcript JSONL file.
///
/// Reads the file and uses the last assistant message as the post text.
/// Sets audience to "team" for board visibility.
pub fn post_from_transcript(
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

    use std::io::BufRead;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        #[derive(serde::Deserialize)]
        struct TranscriptLine {
            role: String,
            content: String,
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptLine>(trimmed)
            && entry.role == "assistant"
        {
            last_assistant_content = Some(entry.content);
        }
    }

    match last_assistant_content {
        Some(content) => post_from_text(db, index, repo, &content),
        None => Err(LegionError::NoReflectionInput),
    }
}

/// Retrieve all board posts and mark them as read for the given reader repo.
///
/// Returns the posts before marking, so the caller sees the full board
/// including any previously unread posts.
pub fn board(db: &Database, reader_repo: &str) -> Result<Vec<Reflection>> {
    let posts = db.get_board_posts()?;
    db.mark_board_read(reader_repo)?;
    Ok(posts)
}

/// Return the count of unread board posts for the given reader repo.
pub fn board_count(db: &Database, reader_repo: &str) -> Result<u64> {
    db.get_unread_count(reader_repo)
}

/// Format board posts for display.
///
/// Produces output like:
/// ```text
/// [Legion] Board (2 posts):
/// - [kelex] some insight (2026-03-05)
/// - [rafters] another thought (2026-03-04)
/// ```
///
/// Returns an empty string when there are no posts.
pub fn format_board(posts: &[Reflection]) -> String {
    if posts.is_empty() {
        return String::new();
    }

    let mut output = format!("[Legion] Board ({} posts):\n", posts.len());

    for p in posts {
        let date = format_date(&p.created_at);
        output.push_str(&format!("- [{}] {} ({})\n", p.repo, p.text, date));
    }

    output
}

/// Format unread board count for display.
///
/// Returns a message like "3 unread posts on the board" when count > 0.
/// Returns an empty string when count is 0 (no noise for hooks).
pub fn format_board_count(count: u64) -> String {
    if count == 0 {
        return String::new();
    }

    format!("{} unread posts on the board", count)
}

/// Format an ISO 8601 timestamp to a date-only string (YYYY-MM-DD).
fn format_date(iso_timestamp: &str) -> String {
    match iso_timestamp.split_once('T') {
        Some((date, _)) => date.to_owned(),
        None => iso_timestamp.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recall;
    use crate::testutil::test_storage;

    #[test]
    fn post_from_text_stores_with_team_audience() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "kelex", "shared insight for the team").expect("post");

        let posts = db.get_board_posts().expect("get_board_posts");
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].audience, "team");
        assert_eq!(posts[0].text, "shared insight for the team");
        assert_eq!(posts[0].repo, "kelex");
    }

    #[test]
    fn post_is_discoverable_via_consult() {
        let (db, index, _dir) = test_storage();
        post_from_text(
            &db,
            &index,
            "rafters",
            "token generation pipeline optimization",
        )
        .expect("post");

        let result = recall::consult(&db, &index, "token generation", 5).expect("consult");
        assert_eq!(result.reflections.len(), 1);
        assert!(result.reflections[0].text.contains("token generation"));
    }

    #[test]
    fn board_returns_only_posts_not_reflections() {
        let (db, index, _dir) = test_storage();

        // Store a private reflection
        crate::reflect::reflect_from_text(&db, &index, "kelex", "private thought")
            .expect("reflect");

        // Store a board post
        post_from_text(&db, &index, "rafters", "shared thought").expect("post");

        let posts = board(&db, "platform").expect("board");
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].text, "shared thought");
        assert_eq!(posts[0].audience, "team");
    }

    #[test]
    fn board_marks_as_read() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "kelex", "a post").expect("post");

        assert_eq!(db.get_unread_count("platform").expect("count"), 1);

        let _posts = board(&db, "platform").expect("board");

        assert_eq!(
            db.get_unread_count("platform").expect("count after read"),
            0
        );
    }

    #[test]
    fn board_count_returns_unread_count() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "kelex", "post one").expect("post 1");
        post_from_text(&db, &index, "rafters", "post two").expect("post 2");

        let count = board_count(&db, "platform").expect("count");
        assert_eq!(count, 2);
    }

    #[test]
    fn format_board_shows_repo_attribution() {
        let posts = vec![
            Reflection {
                id: "id-1".into(),
                repo: "kelex".into(),
                text: "shared insight".into(),
                created_at: "2026-03-05T12:00:00Z".into(),
                audience: "team".into(),
            },
            Reflection {
                id: "id-2".into(),
                repo: "rafters".into(),
                text: "another thought".into(),
                created_at: "2026-03-04T08:00:00Z".into(),
                audience: "team".into(),
            },
        ];

        let output = format_board(&posts);
        assert!(output.contains("[Legion] Board (2 posts):"));
        assert!(output.contains("[kelex]"));
        assert!(output.contains("[rafters]"));
        assert!(output.contains("shared insight"));
        assert!(output.contains("another thought"));
        assert!(output.contains("2026-03-05"));
        assert!(output.contains("2026-03-04"));
    }

    #[test]
    fn format_board_empty_returns_empty_string() {
        let output = format_board(&[]);
        assert!(output.is_empty());
    }

    #[test]
    fn format_board_count_zero_is_empty_string() {
        let output = format_board_count(0);
        assert!(output.is_empty());
    }

    #[test]
    fn format_board_count_nonzero_shows_message() {
        let output = format_board_count(3);
        assert_eq!(output, "3 unread posts on the board");
    }

    #[test]
    fn compound_repo_post_works() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "platform", "cross-team knowledge").expect("post platform");
        post_from_text(&db, &index, "legion", "cross-team knowledge").expect("post legion");

        let posts = db.get_board_posts().expect("get_board_posts");
        assert_eq!(posts.len(), 2);

        let repos: Vec<&str> = posts.iter().map(|p| p.repo.as_str()).collect();
        assert!(repos.contains(&"platform"));
        assert!(repos.contains(&"legion"));
    }

    #[test]
    fn post_from_text_rejects_empty() {
        let (db, index, _dir) = test_storage();
        let err = post_from_text(&db, &index, "kelex", "").unwrap_err();
        assert!(matches!(err, LegionError::NoReflectionInput));
    }

    #[test]
    fn post_from_transcript_extracts_last_assistant() {
        let dir = tempfile::tempdir().expect("tempdir");
        let transcript = dir.path().join("transcript.jsonl");
        std::fs::write(
            &transcript,
            r#"{"role":"user","content":"hello"}
{"role":"assistant","content":"first response"}
{"role":"assistant","content":"the board post"}
"#,
        )
        .expect("write transcript");

        let (db, index, _idx_dir) = test_storage();
        post_from_transcript(&db, &index, "kelex", &transcript).expect("post from transcript");

        let posts = db.get_board_posts().expect("get_board_posts");
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].text, "the board post");
        assert_eq!(posts[0].audience, "team");
    }
}
