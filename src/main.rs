#[allow(dead_code)]
mod db;
mod error;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
        /// Repository name (e.g., "kelex", "rafters")
        #[arg(long)]
        repo: String,

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

        /// Current task context to match against
        #[arg(long)]
        context: String,

        /// Maximum number of reflections to return
        #[arg(long, default_value = "5")]
        limit: usize,
    },

    /// Show reflection statistics
    Stats {
        /// Repository name (omit for all repos)
        #[arg(long)]
        repo: Option<String>,
    },
}

fn main() -> error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Reflect {
            repo,
            text,
            transcript,
        } => {
            println!("reflect: repo={repo}, text={text:?}, transcript={transcript:?}");
        }
        Commands::Recall {
            repo,
            context,
            limit,
        } => {
            println!("recall: repo={repo}, context={context}, limit={limit}");
        }
        Commands::Stats { repo } => {
            println!("stats: repo={repo:?}");
        }
    }

    Ok(())
}
