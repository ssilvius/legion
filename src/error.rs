use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LegionError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("search index error: {0}")]
    Search(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("no reflection text provided (use --text or --transcript)")]
    NoReflectionInput,

    #[error("transcript file not found: {0}")]
    TranscriptNotFound(PathBuf),

    #[error("data directory not available")]
    NoDataDir,

    #[error("one or more repos failed during compound reflect")]
    ReflectPartialFailure,

    #[error("malformed settings.json: {0}")]
    MalformedSettings(String),

    #[error("home directory not available")]
    NoHomeDir,

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("invalid state transition: cannot {action} a task with status '{current}'")]
    InvalidTaskTransition { action: String, current: String },

    #[error("server error: {0}")]
    Server(String),

    #[error("invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("schedule not found: {0}")]
    ScheduleNotFound(String),

    #[error("watch config error: {0}")]
    WatchConfig(String),

    #[error("watch already running (pid {0})")]
    WatchAlreadyRunning(u32),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, LegionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = LegionError::NoReflectionInput;
        assert_eq!(
            err.to_string(),
            "no reflection text provided (use --text or --transcript)"
        );
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: LegionError = io_err.into();
        assert!(matches!(err, LegionError::Io(_)));
    }

    #[test]
    fn error_from_json() {
        let json_err = serde_json::from_str::<String>("not json").unwrap_err();
        let err: LegionError = json_err.into();
        assert!(matches!(err, LegionError::Json(_)));
    }

    #[test]
    fn error_from_rusqlite() {
        let db_err = rusqlite::Error::InvalidParameterName("bad".to_string());
        let err: LegionError = db_err.into();
        assert!(matches!(err, LegionError::Database(_)));
    }

    #[test]
    fn error_display_transcript_not_found() {
        let err = LegionError::TranscriptNotFound(PathBuf::from("/tmp/missing.jsonl"));
        assert_eq!(
            err.to_string(),
            "transcript file not found: /tmp/missing.jsonl"
        );
    }

    #[test]
    fn error_display_search() {
        let err = LegionError::Search("index corrupted".to_string());
        assert_eq!(err.to_string(), "search index error: index corrupted");
    }

    #[test]
    fn error_display_no_data_dir() {
        let err = LegionError::NoDataDir;
        assert_eq!(err.to_string(), "data directory not available");
    }

    #[test]
    fn result_type_alias_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(LegionError::NoDataDir);
        assert!(err.is_err());
    }
}
