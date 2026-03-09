use crate::db::Database;
use crate::error::Result;

/// Format an ISO 8601 timestamp to a date-only string (YYYY-MM-DD).
///
/// Falls back to the raw value if parsing fails, which keeps output
/// usable even with unexpected timestamp formats.
fn format_date(iso_timestamp: &str) -> String {
    // created_at is RFC 3339, e.g. "2026-03-05T12:34:56.789+00:00"
    // Extract the date portion before the 'T' separator.
    match iso_timestamp.split_once('T') {
        Some((date, _)) => date.to_owned(),
        None => iso_timestamp.to_owned(),
    }
}

/// Print stats for a specific repo or all repos.
///
/// With a repo filter, shows count, oldest, and newest reflection dates
/// for that single repo. Without a filter, shows a summary table of all
/// repos plus a total line.
///
/// Prints "no reflections stored yet" when the database is empty (or the
/// filtered repo has no reflections).
pub fn stats(db: &Database, repo: Option<&str>) -> Result<()> {
    let repo_stats = db.get_stats(repo)?;

    if repo_stats.is_empty() {
        println!("no reflections stored yet");
        return Ok(());
    }

    let mut total_count: u64 = 0;

    for s in &repo_stats {
        total_count += s.count;
        let oldest = format_date(&s.oldest);
        let newest = format_date(&s.newest);
        println!(
            "{}: {} reflections ({} to {})",
            s.repo, s.count, oldest, newest
        );
    }

    // Only print the total line when showing all repos (no filter)
    // and there is more than zero repos.
    if repo.is_none() {
        println!(
            "total: {} reflections across {} repos",
            total_count,
            repo_stats.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a test database and keep TempDir alive so SQLite
    /// does not lose the backing file.
    fn test_db() -> (Database, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        (db, dir)
    }

    #[test]
    fn stats_empty_database() {
        let (db, _dir) = test_db();
        // Should print "no reflections stored yet" and not error.
        stats(&db, None).unwrap();
    }

    #[test]
    fn stats_single_repo() {
        let (db, _dir) = test_db();
        db.insert_reflection("kelex", "one", "self").unwrap();
        db.insert_reflection("kelex", "two", "self").unwrap();

        let repo_stats = db.get_stats(Some("kelex")).unwrap();
        assert_eq!(repo_stats.len(), 1);
        assert_eq!(repo_stats[0].count, 2);

        // The command itself should succeed without error.
        stats(&db, Some("kelex")).unwrap();
    }

    #[test]
    fn stats_all_repos() {
        let (db, _dir) = test_db();
        db.insert_reflection("kelex", "one", "self").unwrap();
        db.insert_reflection("rafters", "two", "self").unwrap();

        let all_stats = db.get_stats(None).unwrap();
        assert_eq!(all_stats.len(), 2);

        // The command itself should succeed without error.
        stats(&db, None).unwrap();
    }

    #[test]
    fn stats_nonexistent_repo_shows_empty_message() {
        let (db, _dir) = test_db();
        db.insert_reflection("kelex", "one", "self").unwrap();

        // Filtering on a repo with no reflections should behave
        // the same as an empty database.
        stats(&db, Some("nonexistent")).unwrap();
    }

    #[test]
    fn format_date_extracts_date_portion() {
        let ts = "2026-03-05T12:34:56.789+00:00";
        assert_eq!(format_date(ts), "2026-03-05");
    }

    #[test]
    fn format_date_handles_no_time() {
        // If for some reason the value has no 'T', return it as-is.
        let ts = "2026-03-05";
        assert_eq!(format_date(ts), "2026-03-05");
    }

    #[test]
    fn format_date_handles_utc_z_suffix() {
        let ts = "2026-03-05T08:00:00Z";
        assert_eq!(format_date(ts), "2026-03-05");
    }
}
