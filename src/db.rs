use std::path::Path;

use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

use crate::error::{LegionError, Result};

/// Format an ISO 8601 timestamp to a date-only string (YYYY-MM-DD).
///
/// Falls back to the raw value if parsing fails, which keeps output
/// usable even with unexpected timestamp formats.
pub(crate) fn format_date(iso_timestamp: &str) -> &str {
    match iso_timestamp.split_once('T') {
        Some((date, _)) => date,
        None => iso_timestamp,
    }
}

/// Persistent storage for reflections backed by SQLite.
pub struct Database {
    conn: Connection,
}

/// A single stored reflection tied to a repository.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Reflection {
    pub id: String,
    pub repo: String,
    pub text: String,
    pub created_at: String,
    pub audience: String,
    // Phase 2.0: Synapse metadata
    pub domain: Option<String>,
    pub tags: Option<String>,
    pub recall_count: i64,
    pub last_recalled_at: Option<String>,
    pub parent_id: Option<String>,
}

/// Aggregate statistics for a repository's reflections.
#[derive(Debug)]
pub struct RepoStats {
    pub repo: String,
    pub count: u64,
    pub oldest: String,
    pub newest: String,
}

/// Map a database row to a Reflection struct.
///
/// Shared by all queries that select
/// (id, repo, text, created_at, audience, domain, tags, recall_count,
///  last_recalled_at, parent_id).
fn map_reflection_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reflection> {
    Ok(Reflection {
        id: row.get(0)?,
        repo: row.get(1)?,
        text: row.get(2)?,
        created_at: row.get(3)?,
        audience: row.get(4)?,
        domain: row.get(5)?,
        tags: row.get(6)?,
        recall_count: row.get(7)?,
        last_recalled_at: row.get(8)?,
        parent_id: row.get(9)?,
    })
}

impl Database {
    /// Open (or create) a SQLite database at the given path.
    ///
    /// Parent directories are created automatically if they do not exist.
    /// WAL mode is enabled for concurrent read performance.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .map_err(LegionError::Database)?;
        if mode != "wal" {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }

        Self::init_schema(&conn)?;

        Ok(Self { conn })
    }

    /// Check whether a table has a specific column via PRAGMA table_info.
    fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let names: Vec<String> = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LegionError::Database)?;
        Ok(names.iter().any(|n| n == column))
    }

    /// Create the reflections table, indexes, and supporting tables.
    ///
    /// Uses `has_column` checks to skip already-applied migrations, so
    /// on a fully-migrated database this does minimal work (CREATE IF NOT
    /// EXISTS checks and a single PRAGMA query).
    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reflections (
                id TEXT PRIMARY KEY,
                repo TEXT NOT NULL,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL,
                embedding BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_reflections_repo ON reflections(repo);
            CREATE INDEX IF NOT EXISTS idx_reflections_created ON reflections(created_at);",
        )?;

        // Migration 1: add audience column + board_reads table.
        // Only run when the column does not yet exist.
        if !Self::has_column(conn, "reflections", "audience")? {
            conn.execute_batch(
                "ALTER TABLE reflections ADD COLUMN audience TEXT NOT NULL DEFAULT 'self';",
            )?;
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS board_reads (
                reader_repo TEXT NOT NULL PRIMARY KEY,
                last_read_at TEXT NOT NULL
            );",
        )?;

        // Migration 2: Phase 2.0 Synapse metadata columns.
        if !Self::has_column(conn, "reflections", "domain")? {
            conn.execute_batch(
                "ALTER TABLE reflections ADD COLUMN domain TEXT;",
            )?;
        }
        if !Self::has_column(conn, "reflections", "tags")? {
            conn.execute_batch(
                "ALTER TABLE reflections ADD COLUMN tags TEXT;",
            )?;
        }
        if !Self::has_column(conn, "reflections", "recall_count")? {
            conn.execute_batch(
                "ALTER TABLE reflections ADD COLUMN recall_count INTEGER NOT NULL DEFAULT 0;",
            )?;
        }
        if !Self::has_column(conn, "reflections", "last_recalled_at")? {
            conn.execute_batch(
                "ALTER TABLE reflections ADD COLUMN last_recalled_at TEXT;",
            )?;
        }
        if !Self::has_column(conn, "reflections", "parent_id")? {
            conn.execute_batch(
                "ALTER TABLE reflections ADD COLUMN parent_id TEXT;",
            )?;
        }

        Ok(())
    }

    /// Insert a new reflection for the given repository.
    ///
    /// Generates a UUIDv7 id and ISO 8601 timestamp automatically.
    /// The `audience` parameter controls visibility: "self" for private
    /// reflections, "team" for board posts visible to all agents.
    pub fn insert_reflection(&self, repo: &str, text: &str, audience: &str) -> Result<Reflection> {
        let id = Uuid::now_v7().to_string();
        let created_at = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO reflections (id, repo, text, created_at, audience) VALUES (?1, ?2, ?3, ?4, ?5)",
            (&id, repo, text, &created_at, audience),
        )?;

        Ok(Reflection {
            id,
            repo: repo.to_owned(),
            text: text.to_owned(),
            created_at,
            audience: audience.to_owned(),
        })
    }

    /// Retrieve a single reflection by its ID.
    ///
    /// Returns `None` if no reflection exists with the given ID.
    pub fn get_reflection_by_id(&self, id: &str) -> Result<Option<Reflection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo, text, created_at, audience FROM reflections WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map([id], map_reflection_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Retrieve all reflections for a repository, ordered newest first.
    #[cfg(test)]
    pub fn get_reflections_by_repo(&self, repo: &str) -> Result<Vec<Reflection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo, text, created_at, audience FROM reflections WHERE repo = ?1 ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([repo], map_reflection_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LegionError::Database)
    }

    /// Retrieve the most recent reflections for a repository, limited by SQL.
    ///
    /// More efficient than `get_reflections_by_repo` when only a small
    /// number of results are needed, since the database handles the LIMIT.
    pub fn get_latest_reflections(&self, repo: &str, limit: usize) -> Result<Vec<Reflection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo, text, created_at, audience FROM reflections WHERE repo = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![repo, limit], map_reflection_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LegionError::Database)
    }

    /// Retrieve all board posts (audience = "team"), ordered newest first.
    pub fn get_board_posts(&self) -> Result<Vec<Reflection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, repo, text, created_at, audience FROM reflections WHERE audience = 'team' ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], map_reflection_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LegionError::Database)
    }

    /// Count team posts that are unread by the given reader repo.
    ///
    /// If the reader has no entry in board_reads, all team posts are unread.
    pub fn get_unread_count(&self, reader_repo: &str) -> Result<u64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM reflections WHERE audience = 'team' \
             AND created_at > COALESCE( \
                 (SELECT last_read_at FROM board_reads WHERE reader_repo = ?1), \
                 '' \
             )",
        )?;

        let count: u64 = stmt
            .query_row([reader_repo], |row| row.get(0))
            .map_err(LegionError::Database)?;

        Ok(count)
    }

    /// Mark all current board posts as read for the given reader repo.
    ///
    /// Upserts the board_reads row with the current timestamp.
    pub fn mark_board_read(&self, reader_repo: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO board_reads (reader_repo, last_read_at) VALUES (?1, ?2) \
             ON CONFLICT(reader_repo) DO UPDATE SET last_read_at = excluded.last_read_at",
            (reader_repo, &now),
        )?;

        Ok(())
    }

    /// Retrieve all reflections for reindexing.
    ///
    /// Returns every reflection in the database regardless of audience or
    /// repo. Used by the `reindex` command to rebuild the search index
    /// from the database (the source of truth).
    pub fn get_all_for_reindex(&self) -> Result<Vec<Reflection>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, repo, text, created_at, audience FROM reflections")?;
        let rows = stmt.query_map([], map_reflection_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LegionError::Database)
    }

    /// Get aggregate statistics, optionally filtered to a single repository.
    pub fn get_stats(&self, repo: Option<&str>) -> Result<Vec<RepoStats>> {
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<RepoStats> {
            Ok(RepoStats {
                repo: row.get(0)?,
                count: row.get(1)?,
                oldest: row.get(2)?,
                newest: row.get(3)?,
            })
        };

        let base = "SELECT repo, COUNT(*) as count, MIN(created_at) as oldest, \
                     MAX(created_at) as newest FROM reflections";

        let sql = match repo {
            Some(_) => format!("{base} WHERE repo = ?1 GROUP BY repo"),
            None => format!("{base} GROUP BY repo ORDER BY repo"),
        };

        let mut stmt = self.conn.prepare(&sql)?;

        let rows = match repo {
            Some(r) => stmt.query_map([r], map_row)?,
            None => stmt.query_map([], map_row)?,
        };

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LegionError::Database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an in-memory database for testing.
    fn test_db() -> Database {
        let dir = tempfile::tempdir().unwrap();
        Database::open(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn open_creates_database() {
        let dir = tempfile::tempdir().unwrap();
        let _db = Database::open(&dir.path().join("test.db")).unwrap();
        assert!(dir.path().join("test.db").exists());
    }

    #[test]
    fn open_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c").join("test.db");
        let _db = Database::open(&nested).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn insert_and_retrieve_reflection() {
        let db = test_db();
        let r = db
            .insert_reflection("kelex", "mapping rules are fragile", "self")
            .unwrap();
        assert_eq!(r.repo, "kelex");
        assert_eq!(r.text, "mapping rules are fragile");
        assert!(!r.id.is_empty());

        let all = db.get_reflections_by_repo("kelex").unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, r.id);
    }

    #[test]
    fn reflections_scoped_to_repo() {
        let db = test_db();
        db.insert_reflection("kelex", "reflection 1", "self")
            .unwrap();
        db.insert_reflection("rafters", "reflection 2", "self")
            .unwrap();

        let kelex = db.get_reflections_by_repo("kelex").unwrap();
        assert_eq!(kelex.len(), 1);
        assert_eq!(kelex[0].text, "reflection 1");
    }

    #[test]
    fn stats_returns_counts() {
        let db = test_db();
        db.insert_reflection("kelex", "one", "self").unwrap();
        db.insert_reflection("kelex", "two", "self").unwrap();
        db.insert_reflection("rafters", "three", "self").unwrap();

        let stats = db.get_stats(None).unwrap();
        assert_eq!(stats.len(), 2);

        let kelex_stats = db.get_stats(Some("kelex")).unwrap();
        assert_eq!(kelex_stats.len(), 1);
        assert_eq!(kelex_stats[0].count, 2);
    }

    #[test]
    fn ids_are_uuidv7() {
        let db = test_db();
        let r = db.insert_reflection("test", "text", "self").unwrap();
        assert_eq!(r.id.len(), 36);
        // UUIDv7 has version nibble '7' at position 14
        assert_eq!(&r.id[14..15], "7");
    }

    #[test]
    fn created_at_is_iso8601() {
        let db = test_db();
        let r = db.insert_reflection("test", "text", "self").unwrap();
        // ISO 8601 strings contain 'T' separator and '+' or end with 'Z'
        assert!(r.created_at.contains('T'));
    }

    #[test]
    fn empty_repo_returns_empty_vec() {
        let db = test_db();
        let results = db.get_reflections_by_repo("nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn stats_empty_database() {
        let db = test_db();
        let stats = db.get_stats(None).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn stats_for_nonexistent_repo() {
        let db = test_db();
        db.insert_reflection("kelex", "one", "self").unwrap();
        let stats = db.get_stats(Some("nonexistent")).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn insert_reflection_with_audience_self() {
        let db = test_db();
        let r = db.insert_reflection("kelex", "test", "self").unwrap();
        assert_eq!(r.audience, "self");
    }

    #[test]
    fn insert_reflection_with_audience_team() {
        let db = test_db();
        let r = db
            .insert_reflection("rafters", "night shift musings", "team")
            .unwrap();
        assert_eq!(r.audience, "team");
    }

    #[test]
    fn get_board_posts_returns_only_team() {
        let db = test_db();
        db.insert_reflection("kelex", "private note", "self")
            .unwrap();
        db.insert_reflection("rafters", "shared insight", "team")
            .unwrap();
        let posts = db.get_board_posts().unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].audience, "team");
    }

    #[test]
    fn unread_count_all_unread_when_no_reads() {
        let db = test_db();
        db.insert_reflection("rafters", "post 1", "team").unwrap();
        db.insert_reflection("kelex", "post 2", "team").unwrap();
        assert_eq!(db.get_unread_count("legion").unwrap(), 2);
    }

    #[test]
    fn mark_board_read_resets_unread_count() {
        let db = test_db();
        db.insert_reflection("rafters", "old post", "team").unwrap();
        db.mark_board_read("kelex").unwrap();
        assert_eq!(db.get_unread_count("kelex").unwrap(), 0);
    }

    #[test]
    fn get_all_for_reindex_returns_all_reflections() {
        let db = test_db();
        db.insert_reflection("kelex", "one", "self").unwrap();
        db.insert_reflection("rafters", "two", "team").unwrap();
        db.insert_reflection("platform", "three", "self").unwrap();

        let all = db.get_all_for_reindex().unwrap();
        assert_eq!(all.len(), 3);

        let repos: Vec<&str> = all.iter().map(|r| r.repo.as_str()).collect();
        assert!(repos.contains(&"kelex"));
        assert!(repos.contains(&"rafters"));
        assert!(repos.contains(&"platform"));
    }

    #[test]
    fn get_all_for_reindex_empty_db() {
        let db = test_db();
        let all = db.get_all_for_reindex().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn existing_reflections_default_to_self() {
        let db = test_db();
        let r = db
            .insert_reflection("test", "old reflection", "self")
            .unwrap();
        assert_eq!(r.audience, "self");
        let posts = db.get_board_posts().unwrap();
        assert!(posts.is_empty());
    }

    #[test]
    fn get_board_posts_ordered_newest_first() {
        let db = test_db();
        db.insert_reflection("kelex", "first post", "team").unwrap();
        db.insert_reflection("rafters", "second post", "team")
            .unwrap();
        let posts = db.get_board_posts().unwrap();
        assert_eq!(posts.len(), 2);
        // Newest first means second post should be first in results
        assert_eq!(posts[0].text, "second post");
        assert_eq!(posts[1].text, "first post");
    }

    #[test]
    fn mark_board_read_is_idempotent() {
        let db = test_db();
        db.insert_reflection("rafters", "a post", "team").unwrap();
        db.mark_board_read("kelex").unwrap();
        db.mark_board_read("kelex").unwrap();
        assert_eq!(db.get_unread_count("kelex").unwrap(), 0);
    }

    #[test]
    fn unread_count_tracks_new_posts_after_read() {
        let db = test_db();
        db.insert_reflection("rafters", "old post", "team").unwrap();
        db.mark_board_read("kelex").unwrap();
        assert_eq!(db.get_unread_count("kelex").unwrap(), 0);

        // New post after marking read should be unread
        // Small sleep to ensure timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(10));
        db.insert_reflection("platform", "new post", "team")
            .unwrap();
        assert_eq!(db.get_unread_count("kelex").unwrap(), 1);
    }
}
