use chrono::{DateTime, Utc};

use crate::db::{Database, Reflection};
use crate::error::Result;
use crate::signal;
use crate::task::Task;

/// A single item in a status section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatusItem {
    pub category: String,
    pub text: String,
    pub from: String,
    pub age: String,
}

/// Complete status output for an agent.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatusOutput {
    pub repo: String,
    pub your_work: Vec<StatusItem>,
    pub team_needs: Vec<StatusItem>,
    pub what_changed: Vec<StatusItem>,
}

/// Hours to look back for team posts.
const LOOKBACK_HOURS: i64 = 24;
/// Maximum items in the WHAT CHANGED section.
const MAX_CHANGED: usize = 10;

/// Gather the full status for a repo.
pub fn get_status(db: &Database, repo: &str) -> Result<StatusOutput> {
    let your_work = get_your_work(db, repo)?;
    let team_needs = get_team_needs(db, repo)?;
    let what_changed = get_what_changed(db, repo, &team_needs)?;

    Ok(StatusOutput {
        repo: repo.to_string(),
        your_work,
        team_needs,
        what_changed,
    })
}

/// Format status output for terminal display.
pub fn format_status(output: &StatusOutput) -> String {
    let has_work = !output.your_work.is_empty();
    let has_needs = !output.team_needs.is_empty();
    let has_changed = !output.what_changed.is_empty();

    if !has_work && !has_needs && !has_changed {
        return String::new();
    }

    let mut out = format!("[Legion] Status for {}:\n", output.repo);

    if has_work {
        out.push_str("\nYOUR WORK:\n");
        for item in &output.your_work {
            out.push_str(&format!(
                "  [{}] {}  (from: {}, {})\n",
                item.category, item.text, item.from, item.age
            ));
        }
    }

    if has_needs {
        out.push_str("\nTEAM NEEDS YOU:\n");
        for item in &output.team_needs {
            out.push_str(&format!(
                "  [{}] {}  (from: {}, {})\n",
                item.category, item.text, item.from, item.age
            ));
        }
    }

    if has_changed {
        out.push_str("\nWHAT CHANGED:\n");
        for item in &output.what_changed {
            out.push_str(&format!(
                "  [{}] {}  (from: {}, {})\n",
                item.category, item.text, item.from, item.age
            ));
        }
    }

    out
}

/// YOUR WORK: pending, accepted, and blocked tasks assigned to this repo.
fn get_your_work(db: &Database, repo: &str) -> Result<Vec<StatusItem>> {
    let tasks: Vec<Task> = db.get_active_tasks_for_repo(repo)?;
    let mut items: Vec<StatusItem> = Vec::with_capacity(tasks.len());

    for t in &tasks {
        let category = format!("TASK:{}", t.priority);
        let text = match t.status.as_str() {
            "blocked" => format!("{} [BLOCKED]", t.text),
            "accepted" => format!("{} [in progress]", t.text),
            _ => t.text.clone(),
        };
        items.push(StatusItem {
            category,
            text,
            from: t.from_repo.clone(),
            age: relative_time(&t.created_at),
        });
    }

    Ok(items)
}

/// TEAM NEEDS YOU: recent posts mentioning this repo or @all with action keywords.
fn get_team_needs(db: &Database, repo: &str) -> Result<Vec<StatusItem>> {
    let posts: Vec<Reflection> = db.get_recent_board_posts(LOOKBACK_HOURS)?;
    let repo_lower: String = repo.to_lowercase();
    let at_repo: String = format!("@{}", repo_lower);
    let mut items: Vec<StatusItem> = Vec::new();

    for p in &posts {
        // Skip own posts
        if p.repo.to_lowercase() == repo_lower {
            continue;
        }

        let text_lower: String = p.text.to_lowercase();

        // Check if this is a signal directed at this repo
        if let Some(sig) = signal::parse_signal(&p.text) {
            let directed: bool =
                sig.recipient.to_lowercase() == repo_lower || sig.recipient.to_lowercase() == "all";
            let is_actionable: bool = matches!(
                sig.verb.to_lowercase().as_str(),
                "review" | "question" | "request" | "blocker"
            );

            if directed && is_actionable {
                let category: String = categorize_signal(&sig.verb);
                items.push(StatusItem {
                    category,
                    text: truncate(&p.text, 120),
                    from: p.repo.clone(),
                    age: relative_time(&p.created_at),
                });
                continue;
            }
        }

        // Check for direct @mention
        if text_lower.contains(&at_repo) {
            let category: String = categorize_post_text(&text_lower);
            items.push(StatusItem {
                category,
                text: truncate(&p.text, 120),
                from: p.repo.clone(),
                age: relative_time(&p.created_at),
            });
            continue;
        }

        // Check for @all with action keywords
        if text_lower.contains("@all") && has_action_keyword(&text_lower) {
            let category: String = categorize_post_text(&text_lower);
            items.push(StatusItem {
                category,
                text: truncate(&p.text, 120),
                from: p.repo.clone(),
                age: relative_time(&p.created_at),
            });
        }
    }

    Ok(items)
}

/// WHAT CHANGED: recent announcements and status updates, excluding items
/// already in team_needs.
fn get_what_changed(
    db: &Database,
    repo: &str,
    team_needs: &[StatusItem],
) -> Result<Vec<StatusItem>> {
    let posts: Vec<Reflection> = db.get_recent_board_posts(LOOKBACK_HOURS)?;
    let repo_lower: String = repo.to_lowercase();
    let mut items: Vec<StatusItem> = Vec::new();

    // Build a set of texts already shown in team_needs to avoid duplicates
    let needs_texts: Vec<String> = team_needs.iter().map(|n| n.text.clone()).collect();

    for p in &posts {
        // Skip own posts
        if p.repo.to_lowercase() == repo_lower {
            continue;
        }

        let preview: String = truncate(&p.text, 120);

        // Skip if already in team_needs
        if needs_texts.contains(&preview) {
            continue;
        }

        let text_lower: String = p.text.to_lowercase();

        // Signals with announce/status verbs
        if let Some(sig) = signal::parse_signal(&p.text)
            && matches!(
                sig.verb.to_lowercase().as_str(),
                "announce" | "status" | "update"
            )
        {
            items.push(StatusItem {
                category: "UPDATE".to_string(),
                text: preview,
                from: p.repo.clone(),
                age: relative_time(&p.created_at),
            });
            if items.len() >= MAX_CHANGED {
                break;
            }
            continue;
        }

        // Posts with update-like keywords
        if has_update_keyword(&text_lower) {
            items.push(StatusItem {
                category: "UPDATE".to_string(),
                text: preview,
                from: p.repo.clone(),
                age: relative_time(&p.created_at),
            });
            if items.len() >= MAX_CHANGED {
                break;
            }
        }
    }

    Ok(items)
}

/// Convert an ISO 8601 timestamp to a relative time string.
fn relative_time(iso_timestamp: &str) -> String {
    let parsed: std::result::Result<DateTime<Utc>, _> =
        DateTime::parse_from_rfc3339(iso_timestamp).map(|dt| dt.with_timezone(&Utc));

    let ts: DateTime<Utc> = match parsed {
        Ok(dt) => dt,
        Err(_) => return iso_timestamp.to_string(),
    };

    let now: DateTime<Utc> = Utc::now();
    let diff: chrono::TimeDelta = now.signed_duration_since(ts);

    let minutes: i64 = diff.num_minutes();
    if minutes < 1 {
        return "just now".to_string();
    }
    if minutes < 60 {
        return format!("{}m ago", minutes);
    }

    let hours: i64 = diff.num_hours();
    if hours < 24 {
        return format!("{}h ago", hours);
    }

    let days: i64 = diff.num_days();
    format!("{}d ago", days)
}

/// Categorize a signal verb into a display category.
fn categorize_signal(verb: &str) -> String {
    match verb.to_lowercase().as_str() {
        "review" => "REVIEW".to_string(),
        "question" => "QUESTION".to_string(),
        "request" => "REQUEST".to_string(),
        "blocker" => "BLOCKER".to_string(),
        _ => "SIGNAL".to_string(),
    }
}

/// Categorize a post based on text content keywords.
fn categorize_post_text(text_lower: &str) -> String {
    if text_lower.contains("review") || text_lower.contains("pr ") || text_lower.contains("pr#") {
        "REVIEW".to_string()
    } else if text_lower.contains("question") || text_lower.contains('?') {
        "QUESTION".to_string()
    } else if text_lower.contains("help") || text_lower.contains("blocked") {
        "REQUEST".to_string()
    } else {
        "MENTION".to_string()
    }
}

/// Check if text contains action keywords (for @all filtering).
fn has_action_keyword(text_lower: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "review", "help", "question", "needs", "blocked", "pr ", "pr#",
    ];
    KEYWORDS.iter().any(|kw| text_lower.contains(kw))
}

/// Check if text contains update/announcement keywords.
fn has_update_keyword(text_lower: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "shipped", "merged", "complete", "released", "deployed", "done", "launched", "finished",
    ];
    KEYWORDS.iter().any(|kw| text_lower.contains(kw))
}

/// Truncate text to max_chars, using first line only.
fn truncate(text: &str, max_chars: usize) -> String {
    let first_line: &str = text.lines().next().unwrap_or(text);
    if first_line.chars().count() <= max_chars {
        first_line.to_string()
    } else {
        let truncated: String = first_line.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::test_storage;

    #[test]
    fn relative_time_just_now() {
        let now = Utc::now().to_rfc3339();
        let result = relative_time(&now);
        assert!(
            result == "just now" || result.ends_with("m ago"),
            "unexpected: {}",
            result
        );
    }

    #[test]
    fn relative_time_minutes_ago() {
        let past = (Utc::now() - chrono::Duration::minutes(15)).to_rfc3339();
        let result = relative_time(&past);
        assert!(result.contains("m ago"), "expected minutes ago: {}", result);
    }

    #[test]
    fn relative_time_hours_ago() {
        let past = (Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        let result = relative_time(&past);
        assert_eq!(result, "3h ago");
    }

    #[test]
    fn relative_time_days_ago() {
        let past = (Utc::now() - chrono::Duration::days(2)).to_rfc3339();
        let result = relative_time(&past);
        assert_eq!(result, "2d ago");
    }

    #[test]
    fn relative_time_invalid_falls_back() {
        let result = relative_time("not-a-timestamp");
        assert_eq!(result, "not-a-timestamp");
    }

    #[test]
    fn truncate_short_text() {
        assert_eq!(truncate("hello", 120), "hello");
    }

    #[test]
    fn truncate_long_text() {
        let long = "a".repeat(200);
        let result = truncate(&long, 120);
        assert!(result.ends_with("..."));
        // 120 chars + "..."
        assert_eq!(result.len(), 123);
    }

    #[test]
    fn truncate_multiline_uses_first_line() {
        let text = "first line\nsecond line\nthird line";
        assert_eq!(truncate(text, 120), "first line");
    }

    #[test]
    fn status_empty_database_returns_empty() {
        let (db, _index, _dir) = test_storage();
        let output = get_status(&db, "kelex").expect("get_status");
        assert!(output.your_work.is_empty());
        assert!(output.team_needs.is_empty());
        assert!(output.what_changed.is_empty());
    }

    #[test]
    fn format_status_empty_returns_empty_string() {
        let output = StatusOutput {
            repo: "kelex".to_string(),
            your_work: vec![],
            team_needs: vec![],
            what_changed: vec![],
        };
        assert!(format_status(&output).is_empty());
    }

    #[test]
    fn format_status_shows_sections() {
        let output = StatusOutput {
            repo: "kelex".to_string(),
            your_work: vec![StatusItem {
                category: "TASK:high".to_string(),
                text: "implement search".to_string(),
                from: "platform".to_string(),
                age: "3h ago".to_string(),
            }],
            team_needs: vec![StatusItem {
                category: "REVIEW".to_string(),
                text: "PR #36 needs review".to_string(),
                from: "mail".to_string(),
                age: "45m ago".to_string(),
            }],
            what_changed: vec![StatusItem {
                category: "UPDATE".to_string(),
                text: "shipped v1.0".to_string(),
                from: "eavesdrop".to_string(),
                age: "4h ago".to_string(),
            }],
        };
        let formatted = format_status(&output);
        assert!(formatted.contains("[Legion] Status for kelex:"));
        assert!(formatted.contains("YOUR WORK:"));
        assert!(formatted.contains("[TASK:high] implement search"));
        assert!(formatted.contains("TEAM NEEDS YOU:"));
        assert!(formatted.contains("[REVIEW] PR #36 needs review"));
        assert!(formatted.contains("WHAT CHANGED:"));
        assert!(formatted.contains("[UPDATE] shipped v1.0"));
    }

    #[test]
    fn your_work_shows_tasks() {
        let (db, _index, _dir) = test_storage();
        crate::task::create_task(&db, "platform", "kelex", "build the thing", None, "high")
            .expect("create");

        let items = get_your_work(&db, "kelex").expect("your_work");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, "TASK:high");
        assert!(items[0].text.contains("build the thing"));
        assert_eq!(items[0].from, "platform");
    }

    #[test]
    fn your_work_shows_blocked_tasks() {
        let (db, _index, _dir) = test_storage();
        let id = crate::task::create_task(&db, "platform", "kelex", "blocked work", None, "med")
            .expect("create");
        crate::task::accept_task(&db, &id).expect("accept");
        crate::task::block_task(&db, &id, Some("waiting")).expect("block");

        let items = get_your_work(&db, "kelex").expect("your_work");
        assert_eq!(items.len(), 1);
        assert!(items[0].text.contains("[BLOCKED]"));
    }

    #[test]
    fn your_work_excludes_done_tasks() {
        let (db, _index, _dir) = test_storage();
        let id = crate::task::create_task(&db, "platform", "kelex", "done work", None, "med")
            .expect("create");
        crate::task::accept_task(&db, &id).expect("accept");
        crate::task::complete_task(&db, &id, None).expect("complete");

        let items = get_your_work(&db, "kelex").expect("your_work");
        assert!(items.is_empty());
    }

    #[test]
    fn team_needs_picks_up_signals() {
        let (db, _index, _dir) = test_storage();
        db.insert_reflection("mail", "@kelex review:ready PR #36 needs your eyes", "team")
            .expect("insert");

        let items = get_team_needs(&db, "kelex").expect("team_needs");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, "REVIEW");
        assert_eq!(items[0].from, "mail");
    }

    #[test]
    fn team_needs_excludes_own_posts() {
        let (db, _index, _dir) = test_storage();
        db.insert_reflection("kelex", "@all review:ready something", "team")
            .expect("insert");

        let items = get_team_needs(&db, "kelex").expect("team_needs");
        assert!(items.is_empty());
    }

    #[test]
    fn what_changed_picks_up_announcements() {
        let (db, _index, _dir) = test_storage();
        db.insert_reflection("eavesdrop", "@all announce: shipped v1.0 pipeline", "team")
            .expect("insert");

        let items = get_what_changed(&db, "kelex", &[]).expect("what_changed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, "UPDATE");
        assert_eq!(items[0].from, "eavesdrop");
    }

    #[test]
    fn what_changed_picks_up_keyword_posts() {
        let (db, _index, _dir) = test_storage();
        db.insert_reflection("mail", "mail agent shipped core package", "team")
            .expect("insert");

        let items = get_what_changed(&db, "kelex", &[]).expect("what_changed");
        assert_eq!(items.len(), 1);
        assert!(items[0].text.contains("shipped"));
    }

    #[test]
    fn what_changed_excludes_own_posts() {
        let (db, _index, _dir) = test_storage();
        db.insert_reflection("kelex", "kelex shipped something", "team")
            .expect("insert");

        let items = get_what_changed(&db, "kelex", &[]).expect("what_changed");
        assert!(items.is_empty());
    }

    #[test]
    fn categorize_post_text_detects_review() {
        assert_eq!(categorize_post_text("please review this pr"), "REVIEW");
        assert_eq!(categorize_post_text("pr #36 ready"), "REVIEW");
    }

    #[test]
    fn categorize_post_text_detects_question() {
        assert_eq!(categorize_post_text("how does this work?"), "QUESTION");
        assert_eq!(categorize_post_text("question about search"), "QUESTION");
    }

    #[test]
    fn categorize_post_text_detects_help_request() {
        assert_eq!(categorize_post_text("need help with embeddings"), "REQUEST");
        assert_eq!(
            categorize_post_text("we are blocked on upstream"),
            "REQUEST"
        );
    }

    #[test]
    fn categorize_post_text_fallback() {
        assert_eq!(categorize_post_text("hey just fyi"), "MENTION");
    }

    #[test]
    fn has_action_keyword_matches() {
        assert!(has_action_keyword("needs review please"));
        assert!(has_action_keyword("can you help with this"));
        assert!(!has_action_keyword("just an announcement"));
    }

    #[test]
    fn has_update_keyword_matches() {
        assert!(has_update_keyword("shipped v1.0"));
        assert!(has_update_keyword("pr merged into main"));
        assert!(!has_update_keyword("working on it"));
    }
}
