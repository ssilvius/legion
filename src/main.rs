mod board;
mod db;
mod error;
mod init;
mod recall;
mod reflect;
mod search;
mod stats;
#[cfg(test)]
mod testutil;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use directories::ProjectDirs;

#[derive(Parser)]
#[command(
    name = "legion",
    about = "Agent specialization through deliberate practice"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a reflection from a completed session
    Reflect {
        /// Repository name(s), comma-separated (e.g., "kelex" or "platform,legion")
        #[arg(long, value_delimiter = ',', required = true)]
        repo: Vec<String>,

        /// Reflection text (mutually exclusive with --transcript)
        #[arg(long, conflicts_with = "transcript")]
        text: Option<String>,

        /// Path to session transcript JSONL file
        #[arg(long, conflicts_with = "text")]
        transcript: Option<PathBuf>,
    },

    /// Recall relevant reflections for the current context
    Recall {
        /// Repository name
        #[arg(long)]
        repo: String,

        /// Current task context to match against (ignored with --latest)
        #[arg(long, default_value = "")]
        context: String,

        /// Maximum number of reflections to return
        #[arg(long, default_value = "5")]
        limit: usize,

        /// Return most recent reflections instead of BM25 search
        #[arg(long)]
        latest: bool,
    },

    /// Search reflections across all repos for cross-agent consultation
    Consult {
        /// Context describing the problem to search for
        #[arg(long)]
        context: String,

        /// Maximum number of reflections to return
        #[arg(long, default_value = "3")]
        limit: usize,
    },

    /// Configure Claude Code hooks for legion
    Init {
        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
    },

    /// Post a message to the shared board for other agents
    Post {
        /// Repository name(s), comma-separated (e.g., "kelex" or "platform,legion")
        #[arg(long, value_delimiter = ',', required = true)]
        repo: Vec<String>,

        /// Post text (mutually exclusive with --transcript)
        #[arg(long, conflicts_with = "transcript")]
        text: Option<String>,

        /// Path to session transcript JSONL file
        #[arg(long, conflicts_with = "text")]
        transcript: Option<PathBuf>,
    },

    /// Read the shared board or check for unread posts
    Board {
        /// Repository name (identifies who is reading)
        #[arg(long)]
        repo: String,

        /// Only show unread count instead of full board
        #[arg(long)]
        count: bool,
    },

    /// Show reflection statistics
    Stats {
        /// Repository name (omit for all repos)
        #[arg(long)]
        repo: Option<String>,
    },
}

fn data_dir() -> error::Result<PathBuf> {
    let path = match std::env::var("LEGION_DATA_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            let dirs = ProjectDirs::from("", "", "legion").ok_or(error::LegionError::NoDataDir)?;
            dirs.data_dir().to_path_buf()
        }
    };
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Run a compound command (text or transcript) across multiple repos.
///
/// Shared by both Reflect and Post to avoid duplicating the repo loop,
/// input validation, and error-collection logic.
#[allow(clippy::too_many_arguments)]
fn run_compound_command(
    db: &db::Database,
    index: &search::SearchIndex,
    repos: &[String],
    text: &Option<String>,
    transcript: &Option<PathBuf>,
    from_text: fn(&db::Database, &search::SearchIndex, &str, &str) -> error::Result<()>,
    from_transcript: fn(
        &db::Database,
        &search::SearchIndex,
        &str,
        &std::path::Path,
    ) -> error::Result<()>,
    label: &str,
) -> error::Result<()> {
    if text.is_none() && transcript.is_none() {
        return Err(error::LegionError::NoReflectionInput);
    }

    let mut had_error = false;
    for r in repos {
        let result = match (text, transcript) {
            (Some(t), None) => from_text(db, index, r, t),
            (None, Some(path)) => from_transcript(db, index, r, path),
            (Some(_), Some(_)) => return Err(error::LegionError::NoReflectionInput),
            (None, None) => unreachable!("guarded by early return above"),
        };
        if let Err(e) = result {
            eprintln!("[legion] error {label} for {r}: {e}");
            had_error = true;
        }
    }
    if had_error {
        return Err(error::LegionError::ReflectPartialFailure);
    }
    Ok(())
}

fn main() -> error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Reflect {
            repo,
            text,
            transcript,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;

            run_compound_command(
                &database,
                &index,
                &repo,
                &text,
                &transcript,
                reflect::reflect_from_text,
                reflect::reflect_from_transcript,
                "storing reflection",
            )?;
        }
        Commands::Recall {
            repo,
            context,
            limit,
            latest,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            let result = if latest {
                recall::recall_latest(&database, &repo, limit)?
            } else {
                let index = search::SearchIndex::open(&base.join("index"))?;
                recall::recall(&database, &index, &repo, &context, limit)?
            };
            let output = recall::format_for_hook(&result);
            if !output.is_empty() {
                print!("{output}");
            }
        }
        Commands::Consult { context, limit } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;

            let result = recall::consult(&database, &index, &context, limit)?;
            let output = recall::format_for_consult(&result);
            if output.is_empty() {
                eprintln!("[legion] no reflections matched context: \"{}\"", context);
            } else {
                print!("{output}");
            }
        }
        Commands::Post {
            repo,
            text,
            transcript,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;

            run_compound_command(
                &database,
                &index,
                &repo,
                &text,
                &transcript,
                board::post_from_text,
                board::post_from_transcript,
                "posting",
            )?;
        }
        Commands::Board { repo, count } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            if count {
                let n = board::board_count(&database, &repo)?;
                let output = board::format_board_count(n);
                if !output.is_empty() {
                    println!("{output}");
                }
            } else {
                let posts = board::board(&database, &repo)?;
                let output = board::format_board(&posts);
                if !output.is_empty() {
                    print!("{output}");
                }
            }
        }
        Commands::Init { force } => {
            init::init(force)?;
        }
        Commands::Stats { repo } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            stats::stats(&database, repo.as_deref())?;
        }
    }

    Ok(())
}
