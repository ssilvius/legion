use std::path::Path;

use crate::db::{self, Database, Reflection, ReflectionMeta};
use crate::error::{LegionError, Result};
use crate::reflect;
use crate::search::SearchIndex;
use crate::signal;

/// Store a bullpen post from direct text input.
///
/// Like `reflect_from_text` but sets audience to "team" so the post
/// appears on the shared bullpen visible to all agents.
#[allow(dead_code)]
pub fn post_from_text(db: &Database, index: &SearchIndex, repo: &str, text: &str) -> Result<()> {
    post_from_text_with_meta(db, index, repo, text, &ReflectionMeta::default())
}

/// Extract and store a bullpen post from a transcript JSONL file.
#[allow(dead_code)]
pub fn post_from_transcript(
    db: &Database,
    index: &SearchIndex,
    repo: &str,
    transcript_path: &Path,
) -> Result<()> {
    post_from_transcript_with_meta(db, index, repo, transcript_path, &ReflectionMeta::default())
}

/// Store a bullpen post from text with Synapse metadata.
pub fn post_from_text_with_meta(
    db: &Database,
    index: &SearchIndex,
    repo: &str,
    text: &str,
    meta: &ReflectionMeta,
) -> Result<()> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(LegionError::NoReflectionInput);
    }

    let reflection = db.insert_reflection_with_meta(repo, trimmed, "team", meta)?;
    index.add(&reflection.id, repo, trimmed)?;

    eprintln!("posted to bullpen for {} ({})", repo, reflection.id);

    Ok(())
}

/// Extract and store a bullpen post from a transcript with Synapse metadata.
pub fn post_from_transcript_with_meta(
    db: &Database,
    index: &SearchIndex,
    repo: &str,
    transcript_path: &Path,
    meta: &ReflectionMeta,
) -> Result<()> {
    let content = reflect::extract_last_assistant_message(transcript_path)?;
    post_from_text_with_meta(db, index, repo, &content, meta)
}

/// Bullpen post filter mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BullpenFilter {
    All,
    SignalsOnly,
    MusingsOnly,
}

/// Retrieve all bullpen posts and mark them as read for the given reader repo.
///
/// Returns the posts before marking, so the caller sees the full bullpen
/// including any previously unread posts.
#[allow(dead_code)]
pub fn bullpen(db: &Database, reader_repo: &str) -> Result<Vec<Reflection>> {
    let posts = db.get_board_posts()?;
    db.mark_board_read(reader_repo)?;
    Ok(posts)
}

/// Retrieve bullpen posts filtered by type.
///
/// Only marks posts as read when viewing All (unfiltered). Filtered views
/// (signals-only, musings-only) do not mark anything as read, since the
/// reader has not seen the full bullpen.
pub fn bullpen_filtered(
    db: &Database,
    reader_repo: &str,
    filter: BullpenFilter,
) -> Result<Vec<Reflection>> {
    let posts = db.get_board_posts()?;

    match filter {
        BullpenFilter::All => {
            db.mark_board_read(reader_repo)?;
            Ok(posts)
        }
        BullpenFilter::SignalsOnly => Ok(posts
            .into_iter()
            .filter(|p| signal::is_signal(&p.text))
            .collect()),
        BullpenFilter::MusingsOnly => Ok(posts
            .into_iter()
            .filter(|p| !signal::is_signal(&p.text))
            .collect()),
    }
}

/// Return the count of unread bullpen posts for the given reader repo.
pub fn bullpen_count(db: &Database, reader_repo: &str) -> Result<u64> {
    db.get_unread_count(reader_repo)
}

/// Format bullpen posts for display.
///
/// Signals are rendered as compact one-liners. Musings get full text.
/// Returns an empty string when there are no posts.
pub fn format_bullpen(posts: &[Reflection]) -> String {
    if posts.is_empty() {
        return String::new();
    }

    let mut output = format!("[Legion] Bullpen ({} posts):\n", posts.len());

    for p in posts {
        let date = db::format_date(&p.created_at);
        if let Some(sig) = signal::parse_signal(&p.text) {
            output.push_str(&format!(
                "- {}\n",
                signal::format_signal_compact(&sig, &p.repo, date)
            ));
        } else {
            output.push_str(&format!("- [{}] {} ({})\n", p.repo, p.text, date));
        }
    }

    output
}

/// Format unread bullpen count for display.
///
/// Returns a combined message when posts or tasks are present.
/// Returns an empty string when both are 0 (no noise for hooks).
pub fn format_bullpen_count(post_count: u64, task_count: u64) -> String {
    if post_count == 0 && task_count == 0 {
        return String::new();
    }

    let mut parts: Vec<String> = Vec::new();
    if post_count > 0 {
        parts.push(format!("{} unread posts", post_count));
    }
    if task_count > 0 {
        parts.push(format!("{} pending tasks", task_count));
    }

    format!("{} on the bullpen", parts.join(", "))
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

        let result = recall::consult_bm25(&db, &index, "token generation", 5).expect("consult");
        assert_eq!(result.reflections.len(), 1);
        assert!(result.reflections[0].text.contains("token generation"));
    }

    #[test]
    fn bullpen_returns_only_posts_not_reflections() {
        let (db, index, _dir) = test_storage();

        // Store a private reflection
        crate::reflect::reflect_from_text(&db, &index, "kelex", "private thought")
            .expect("reflect");

        // Store a bullpen post
        post_from_text(&db, &index, "rafters", "shared thought").expect("post");

        let posts = bullpen(&db, "platform").expect("bullpen");
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].text, "shared thought");
        assert_eq!(posts[0].audience, "team");
    }

    #[test]
    fn bullpen_marks_as_read() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "kelex", "a post").expect("post");

        assert_eq!(db.get_unread_count("platform").expect("count"), 1);

        let _posts = bullpen(&db, "platform").expect("bullpen");

        assert_eq!(
            db.get_unread_count("platform").expect("count after read"),
            0
        );
    }

    #[test]
    fn bullpen_count_returns_unread_count() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "kelex", "post one").expect("post 1");
        post_from_text(&db, &index, "rafters", "post two").expect("post 2");

        let count = bullpen_count(&db, "platform").expect("count");
        assert_eq!(count, 2);
    }

    #[test]
    fn format_bullpen_shows_repo_attribution() {
        let posts = vec![
            Reflection {
                id: "id-1".into(),
                repo: "kelex".into(),
                text: "shared insight".into(),
                created_at: "2026-03-05T12:00:00Z".into(),
                audience: "team".into(),
                domain: None,
                tags: None,
                recall_count: 0,
                last_recalled_at: None,
                parent_id: None,
            },
            Reflection {
                id: "id-2".into(),
                repo: "rafters".into(),
                text: "another thought".into(),
                created_at: "2026-03-04T08:00:00Z".into(),
                audience: "team".into(),
                domain: None,
                tags: None,
                recall_count: 0,
                last_recalled_at: None,
                parent_id: None,
            },
        ];

        let output = format_bullpen(&posts);
        assert!(output.contains("[Legion] Bullpen (2 posts):"));
        assert!(output.contains("[kelex]"));
        assert!(output.contains("[rafters]"));
        assert!(output.contains("shared insight"));
        assert!(output.contains("another thought"));
        assert!(output.contains("2026-03-05"));
        assert!(output.contains("2026-03-04"));
    }

    #[test]
    fn format_bullpen_empty_returns_empty_string() {
        let output = format_bullpen(&[]);
        assert!(output.is_empty());
    }

    #[test]
    fn format_bullpen_count_zero_is_empty_string() {
        let output = format_bullpen_count(0, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn format_bullpen_count_posts_only() {
        let output = format_bullpen_count(3, 0);
        assert_eq!(output, "3 unread posts on the bullpen");
    }

    #[test]
    fn format_bullpen_count_tasks_only() {
        let output = format_bullpen_count(0, 2);
        assert_eq!(output, "2 pending tasks on the bullpen");
    }

    #[test]
    fn format_bullpen_count_posts_and_tasks() {
        let output = format_bullpen_count(3, 2);
        assert_eq!(output, "3 unread posts, 2 pending tasks on the bullpen");
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
    fn filtered_view_does_not_mark_as_read() {
        let (db, index, _dir) = test_storage();
        post_from_text(&db, &index, "kelex", "@legion review:approved").expect("signal");
        post_from_text(&db, &index, "rafters", "casual musing").expect("musing");

        assert_eq!(db.get_unread_count("platform").expect("count"), 2);

        // Filtered view should NOT mark as read
        let _signals =
            bullpen_filtered(&db, "platform", BullpenFilter::SignalsOnly).expect("signals");
        assert_eq!(
            db.get_unread_count("platform").expect("still unread"),
            2,
            "filtered view should not mark posts as read"
        );

        // Unfiltered view SHOULD mark as read
        let _all = bullpen_filtered(&db, "platform", BullpenFilter::All).expect("all");
        assert_eq!(
            db.get_unread_count("platform").expect("now read"),
            0,
            "unfiltered view should mark all as read"
        );
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
