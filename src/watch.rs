use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Timelike;
use serde::Deserialize;

use crate::db::Database;
use crate::error::{LegionError, Result};
use crate::signal;

// -- Config ------------------------------------------------------------------

/// A watched repository entry from watch.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct WatchRepoConfig {
    pub name: String,
    pub workdir: String,
}

/// Top-level watch configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WatchConfig {
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,

    /// Work hours start (0-23, local time). No cooldown during work hours.
    #[serde(default)]
    pub work_hours_start: Option<u8>,

    /// Work hours end (0-23, local time). No cooldown during work hours.
    #[serde(default)]
    pub work_hours_end: Option<u8>,

    #[serde(default)]
    pub repos: Vec<WatchRepoConfig>,
}

fn default_poll_interval() -> u64 {
    30
}

fn default_cooldown_secs() -> u64 {
    300
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: default_poll_interval(),
            cooldown_secs: default_cooldown_secs(),
            work_hours_start: None,
            work_hours_end: None,
            repos: Vec::new(),
        }
    }
}

/// Load watch config from the given path. Returns a default config if the
/// file does not exist.
pub fn load_config(path: &Path) -> Result<WatchConfig> {
    if !path.exists() {
        return Err(LegionError::WatchConfig(format!(
            "config file not found: {}. Create it with watched repos.",
            path.display()
        )));
    }

    let contents = std::fs::read_to_string(path)?;
    let config: WatchConfig = toml::from_str(&contents)?;

    if config.repos.is_empty() {
        return Err(LegionError::WatchConfig(
            "no repos configured in watch.toml".to_string(),
        ));
    }

    // Validate workdirs exist
    for repo in &config.repos {
        if !Path::new(&repo.workdir).is_dir() {
            return Err(LegionError::WatchConfig(format!(
                "workdir does not exist for repo '{}': {}",
                repo.name, repo.workdir
            )));
        }
    }

    Ok(config)
}

// -- PID Lock ----------------------------------------------------------------

/// Acquire a PID lock file. Returns an error if another watcher is running.
pub fn acquire_pid_lock(lock_path: &Path) -> Result<()> {
    if lock_path.exists() {
        let contents = std::fs::read_to_string(lock_path).unwrap_or_default();
        if let Ok(pid) = contents.trim().parse::<u32>() {
            // Check if the process is actually running
            if process_alive(pid) {
                return Err(LegionError::WatchAlreadyRunning(pid));
            }
            // Stale lock file -- process is dead, remove it
            eprintln!("[legion watch] removing stale lock (pid {})", pid);
        }
    }

    let pid = std::process::id();
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(lock_path, pid.to_string())?;
    Ok(())
}

/// Release the PID lock file.
pub fn release_pid_lock(lock_path: &Path) {
    let _ = std::fs::remove_file(lock_path);
}

/// RAII guard that releases the PID lock file on drop.
struct PidLockGuard(PathBuf);

impl Drop for PidLockGuard {
    fn drop(&mut self) {
        release_pid_lock(&self.0);
        eprintln!("[legion watch] released lock");
    }
}

/// Check whether a process with the given PID is alive.
fn process_alive(pid: u32) -> bool {
    // On Unix, signal 0 checks process existence without sending a signal.
    // SAFETY: this is not unsafe -- libc::kill with signal 0 is a standard POSIX check.
    #[cfg(unix)]
    {
        // kill(pid, 0) returns 0 if the process exists and we can signal it
        let result = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        matches!(result, Ok(status) if status.success())
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

// -- Cooldown ----------------------------------------------------------------

/// Tracks per-repo cooldown to prevent wake storms.
pub struct CooldownTracker {
    last_wake: HashMap<String, Instant>,
    cooldown: Duration,
    work_hours_start: Option<u8>,
    work_hours_end: Option<u8>,
}

impl CooldownTracker {
    pub fn new(cooldown_secs: u64, work_hours_start: Option<u8>, work_hours_end: Option<u8>) -> Self {
        Self {
            last_wake: HashMap::new(),
            cooldown: Duration::from_secs(cooldown_secs),
            work_hours_start,
            work_hours_end,
        }
    }

    /// Check whether we are in work hours (no cooldown applies).
    fn is_work_hours(&self) -> bool {
        if let (Some(start), Some(end)) = (self.work_hours_start, self.work_hours_end) {
            let hour = chrono::Local::now().hour() as u8;
            if start <= end {
                hour >= start && hour < end
            } else {
                // Overnight range (e.g., 22-06)
                hour >= start || hour < end
            }
        } else {
            false
        }
    }

    /// Check whether the repo is on cooldown. Returns true if we should skip.
    /// During work hours, cooldown is disabled.
    pub fn is_cooling_down(&self, repo: &str) -> bool {
        if self.is_work_hours() {
            return false;
        }
        self.last_wake
            .get(repo)
            .is_some_and(|t| t.elapsed() < self.cooldown)
    }

    /// Record that a repo was just woken.
    pub fn record_wake(&mut self, repo: &str) {
        self.last_wake.insert(repo.to_string(), Instant::now());
    }
}

// -- Signal Detection --------------------------------------------------------

/// Find unhandled signals targeting a specific repo.
///
/// Returns signal reflection IDs and their text, filtered to only actual
/// signals (text starts with @).
pub fn find_pending_signals(
    db: &Database,
    repo_name: &str,
    since: Option<&str>,
) -> Result<Vec<(String, String, String)>> {
    let reflections = db.get_unhandled_signals_for_repo(repo_name, since)?;

    let mut signals: Vec<(String, String, String)> = Vec::new();
    for r in reflections {
        if signal::is_signal(&r.text) {
            signals.push((r.id, r.text, r.repo));
        }
    }

    Ok(signals)
}

// -- Agent Spawning ----------------------------------------------------------

/// Build the prompt context for a woken agent from pending signals.
pub fn build_wake_prompt(repo_name: &str, signals: &[(String, String, String)]) -> String {
    let mut prompt = format!(
        "You were auto-woken by legion watch. The following signal(s) are directed at you ({}):\n\n",
        repo_name
    );

    for (id, text, from_repo) in signals {
        prompt.push_str(&format!("- [from {}] {} (id: {})\n", from_repo, text, id));
    }

    prompt.push_str(
        "\nRead and respond to each signal. Use `legion signal` to reply if needed. \
         Use `legion bullpen` to check for broader context. When done, use `legion reflect` \
         to store any learnings.\n\n\
         IMPORTANT: Do NOT respond to announcements or signals that don't need a response. \
         Silence is acknowledgment. Only respond if you have NEW information, a concern, \
         a dissent, or an action item. Empty acknowledgments like 'acknowledged, no action needed' \
         waste tokens and trigger wake storms. If you have nothing substantive to add, \
         reflect and exit.",
    );

    prompt
}

/// Spawn a `claude --print` session for the given repo.
///
/// Returns Ok(()) after spawning (does not wait for completion).
pub fn spawn_agent(workdir: &str, prompt: &str) -> Result<()> {
    let child = std::process::Command::new("claude")
        .args(["--print", "-p", prompt])
        .current_dir(workdir)
        .env("LEGION_AUTO_WAKE", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match child {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("[legion watch] failed to spawn agent: {}", e);
            Err(LegionError::Io(e))
        }
    }
}

// -- Main Loop ---------------------------------------------------------------

/// Run a single poll cycle across all configured repos.
///
/// Returns the number of agents spawned in this cycle.
pub fn poll_cycle(
    db: &Database,
    config: &WatchConfig,
    cooldown: &mut CooldownTracker,
    since: Option<&str>,
) -> Result<u32> {
    let mut spawned: u32 = 0;

    for repo in &config.repos {
        if cooldown.is_cooling_down(&repo.name) {
            continue;
        }

        let signals = find_pending_signals(db, &repo.name, since)?;
        if signals.is_empty() {
            continue;
        }

        eprintln!(
            "[legion watch] {} signal(s) for {} -- waking agent",
            signals.len(),
            repo.name
        );

        let prompt = build_wake_prompt(&repo.name, &signals);

        // Mark targeted signals as handled BEFORE spawning to prevent re-processing.
        // Broadcast signals (@all) are NOT marked handled -- they need to be seen by
        // every configured repo. Cooldown + since-timestamp prevent duplicate wakes.
        for (id, text, _) in &signals {
            if !text.starts_with("@all ") && db.mark_signal_handled(id).is_err() {
                eprintln!("[legion watch] failed to mark signal {} as handled", id);
            }
        }

        match spawn_agent(&repo.workdir, &prompt) {
            Ok(()) => {
                cooldown.record_wake(&repo.name);
                spawned += 1;
                eprintln!("[legion watch] spawned agent for {}", repo.name);
            }
            Err(e) => {
                eprintln!("[legion watch] spawn failed for {}: {}", repo.name, e);
            }
        }
    }

    Ok(spawned)
}

/// Run the watch daemon main loop.
///
/// This function blocks indefinitely, polling SQLite at the configured
/// interval. It handles SIGINT/SIGTERM for graceful shutdown.
pub fn run(data_dir: &Path) -> Result<()> {
    let config_path: PathBuf = data_dir.join("watch.toml");
    let lock_path: PathBuf = data_dir.join("watch.pid");
    let db_path: PathBuf = data_dir.join("legion.db");

    let config = load_config(&config_path)?;

    eprintln!(
        "[legion watch] config loaded: {} repo(s), poll every {}s, cooldown {}s",
        config.repos.len(),
        config.poll_interval_secs,
        config.cooldown_secs
    );

    acquire_pid_lock(&lock_path)?;
    eprintln!("[legion watch] acquired lock (pid {})", std::process::id());

    // Guard that releases the PID lock when dropped
    let _guard = PidLockGuard(lock_path);

    let db = Database::open(&db_path)?;
    let mut cooldown = CooldownTracker::new(
        config.cooldown_secs,
        config.work_hours_start,
        config.work_hours_end,
    );
    let poll_interval = Duration::from_secs(config.poll_interval_secs);
    let start_time = chrono::Utc::now().to_rfc3339();

    eprintln!(
        "[legion watch] watching repos: {}",
        config
            .repos
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    loop {
        match poll_cycle(&db, &config, &mut cooldown, Some(&start_time)) {
            Ok(n) if n > 0 => {
                eprintln!("[legion watch] cycle complete: {} agent(s) spawned", n);
            }
            Ok(_) => {} // quiet cycle, no spam
            Err(e) => {
                eprintln!("[legion watch] poll error: {}", e);
            }
        }

        std::thread::sleep(poll_interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::test_storage;

    #[test]
    fn parse_config_basic() {
        let toml_str = r#"
poll_interval_secs = 15
cooldown_secs = 120

[[repos]]
name = "rafters"
workdir = "/tmp"

[[repos]]
name = "legion"
workdir = "/tmp"
"#;
        let config: WatchConfig = toml::from_str(toml_str).expect("parse config");
        assert_eq!(config.poll_interval_secs, 15);
        assert_eq!(config.cooldown_secs, 120);
        assert_eq!(config.repos.len(), 2);
        assert_eq!(config.repos[0].name, "rafters");
        assert_eq!(config.repos[1].name, "legion");
    }

    #[test]
    fn parse_config_defaults() {
        let toml_str = r#"
[[repos]]
name = "test"
workdir = "/tmp"
"#;
        let config: WatchConfig = toml::from_str(toml_str).expect("parse config");
        assert_eq!(config.poll_interval_secs, 30);
        assert_eq!(config.cooldown_secs, 300);
    }

    #[test]
    fn cooldown_tracker_prevents_rapid_wake() {
        let mut tracker = CooldownTracker::new(300, None, None);
        assert!(!tracker.is_cooling_down("rafters"));

        tracker.record_wake("rafters");
        assert!(tracker.is_cooling_down("rafters"));
        assert!(!tracker.is_cooling_down("legion"));
    }

    #[test]
    fn find_pending_signals_detects_targeted_signals() {
        let (db, _index, _dir) = test_storage();

        // Post a signal from kelex to legion
        db.insert_reflection("kelex", "@legion review:approved", "team")
            .expect("insert signal");

        // Post a non-signal
        db.insert_reflection("rafters", "just a musing", "team")
            .expect("insert musing");

        // Post a signal to all
        db.insert_reflection("rafters", "@all announce: shipped", "team")
            .expect("insert broadcast");

        let signals = find_pending_signals(&db, "legion", None).expect("find signals");
        assert_eq!(signals.len(), 2);

        // Verify the targeted signal is found
        assert!(
            signals
                .iter()
                .any(|(_, text, _)| text == "@legion review:approved")
        );
        // Verify the broadcast is found
        assert!(
            signals
                .iter()
                .any(|(_, text, _)| text == "@all announce: shipped")
        );
    }

    #[test]
    fn find_pending_signals_detects_multi_recipient() {
        let (db, _index, _dir) = test_storage();

        // Multi-recipient signal: @shingle @huttspawn -- message
        db.insert_reflection(
            "legion",
            "@shingle @huttspawn -- build draft sites from current content",
            "team",
        )
        .expect("insert multi-recipient");

        // Both shingle and huttspawn should see it
        let shingle = find_pending_signals(&db, "shingle", None).expect("shingle");
        let huttspawn = find_pending_signals(&db, "huttspawn", None).expect("huttspawn");
        assert_eq!(shingle.len(), 1, "shingle should see multi-recipient signal");
        assert_eq!(
            huttspawn.len(),
            1,
            "huttspawn should see multi-recipient signal"
        );

        // legion (sender) should NOT see it
        let legion = find_pending_signals(&db, "legion", None).expect("legion");
        assert!(legion.is_empty(), "sender should not see own signal");

        // unrelated repo should NOT see it
        let kelex = find_pending_signals(&db, "kelex", None).expect("kelex");
        assert!(kelex.is_empty(), "unmentioned repo should not see signal");
    }

    #[test]
    fn find_pending_signals_excludes_self_signals() {
        let (db, _index, _dir) = test_storage();

        // Signal from legion to legion should not be returned
        db.insert_reflection("legion", "@legion review:approved", "team")
            .expect("insert self-signal");

        let signals = find_pending_signals(&db, "legion", None).expect("find signals");
        assert!(signals.is_empty(), "self-signals should be excluded");
    }

    #[test]
    fn mark_handled_prevents_re_detection() {
        let (db, _index, _dir) = test_storage();

        db.insert_reflection("kelex", "@legion review:approved", "team")
            .expect("insert signal");

        let signals = find_pending_signals(&db, "legion", None).expect("first poll");
        assert_eq!(signals.len(), 1);

        // Mark as handled
        let (id, _, _) = &signals[0];
        db.mark_signal_handled(id).expect("mark handled");

        // Should not appear again
        let signals = find_pending_signals(&db, "legion", None).expect("second poll");
        assert!(signals.is_empty());
    }

    #[test]
    fn build_wake_prompt_formats_signals() {
        let signals = vec![
            (
                "id-1".to_string(),
                "@legion review:approved".to_string(),
                "kelex".to_string(),
            ),
            (
                "id-2".to_string(),
                "@all announce: shipped".to_string(),
                "rafters".to_string(),
            ),
        ];

        let prompt = build_wake_prompt("legion", &signals);
        assert!(prompt.contains("auto-woken by legion watch"));
        assert!(prompt.contains("@legion review:approved"));
        assert!(prompt.contains("@all announce: shipped"));
        assert!(prompt.contains("from kelex"));
        assert!(prompt.contains("from rafters"));
    }

    #[test]
    fn poll_cycle_skips_cooling_repos() {
        let (db, _index, _dir) = test_storage();

        let config = WatchConfig {
            poll_interval_secs: 1,
            cooldown_secs: 300,
            work_hours_start: None,
            work_hours_end: None,
            repos: vec![WatchRepoConfig {
                name: "legion".to_string(),
                workdir: "/tmp".to_string(),
            }],
        };

        // Insert a signal
        db.insert_reflection("kelex", "@legion review:ready", "team")
            .expect("insert");

        // Pre-cool the repo
        let mut cooldown = CooldownTracker::new(300, None, None);
        cooldown.record_wake("legion");

        let spawned = poll_cycle(&db, &config, &mut cooldown, None).expect("poll");
        assert_eq!(spawned, 0, "cooling repo should be skipped");
    }

    #[test]
    fn broadcast_signals_visible_to_all_repos() {
        let (db, _index, _dir) = test_storage();

        // Post an @all signal from kelex
        db.insert_reflection("kelex", "@all RFC:help -- discover proposal", "team")
            .expect("insert broadcast");

        // Both legion and rafters should see it
        let legion_signals = find_pending_signals(&db, "legion", None).expect("legion");
        let rafters_signals = find_pending_signals(&db, "rafters", None).expect("rafters");
        assert_eq!(legion_signals.len(), 1);
        assert_eq!(rafters_signals.len(), 1);

        // Mark handled for legion (targeted signal path) -- but @all should NOT be marked
        // Simulate poll_cycle behavior: @all signals are skipped in mark_handled
        for (id, text, _) in &legion_signals {
            if !text.starts_with("@all ") {
                db.mark_signal_handled(id).expect("mark handled");
            }
        }

        // rafters should STILL see the broadcast
        let rafters_after = find_pending_signals(&db, "rafters", None).expect("rafters after");
        assert_eq!(
            rafters_after.len(),
            1,
            "broadcast should remain visible to other repos"
        );
    }

    #[test]
    fn load_config_rejects_empty_repos() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("watch.toml");
        std::fs::write(&config_path, "poll_interval_secs = 10\n").expect("write");

        let err = load_config(&config_path).unwrap_err();
        assert!(err.to_string().contains("no repos configured"));
    }

    #[test]
    fn load_config_rejects_missing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("nonexistent.toml");

        let err = load_config(&config_path).unwrap_err();
        assert!(err.to_string().contains("config file not found"));
    }

    #[test]
    fn load_config_rejects_bad_workdir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("watch.toml");
        std::fs::write(
            &config_path,
            r#"
[[repos]]
name = "test"
workdir = "/nonexistent/path/that/does/not/exist"
"#,
        )
        .expect("write");

        let err = load_config(&config_path).unwrap_err();
        assert!(err.to_string().contains("workdir does not exist"));
    }

    #[test]
    fn pid_lock_acquire_and_release() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("test.pid");

        acquire_pid_lock(&lock_path).expect("acquire lock");
        assert!(lock_path.exists());

        release_pid_lock(&lock_path);
        assert!(!lock_path.exists());
    }

    #[test]
    fn pid_lock_detects_stale_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("test.pid");

        // Write a fake PID that is very unlikely to be running
        std::fs::write(&lock_path, "999999999").expect("write stale lock");

        // Should succeed because the process is not running
        acquire_pid_lock(&lock_path).expect("acquire lock over stale");
    }
}
