mod board;
mod db;
mod embed;
mod error;
mod init;
mod recall;
mod reflect;
mod search;
mod serve;
mod signal;
mod stats;
mod status;
mod surface;
mod task;
#[cfg(test)]
mod testutil;
mod watch;

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

        /// Domain tag for classification (e.g., "color-tokens", "auth")
        #[arg(long)]
        domain: Option<String>,

        /// Comma-separated tags (e.g., "semantic-tokens,consumer,debugging")
        #[arg(long)]
        tags: Option<String>,

        /// Link to a parent reflection ID to form a learning chain
        #[arg(long)]
        follows: Option<String>,
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

    /// Post a message to the shared bullpen for other agents
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

        /// Domain tag for classification
        #[arg(long)]
        domain: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Link to a parent reflection ID to form a learning chain
        #[arg(long)]
        follows: Option<String>,
    },

    /// Mark a reflection as useful after recalling and applying it
    Boost {
        /// Reflection ID to boost
        #[arg(long)]
        id: String,
    },

    /// Trace a learning chain from a reflection
    Chain {
        /// Any reflection ID in the chain
        #[arg(long)]
        id: String,
    },

    /// Send a structured signal to another agent
    Signal {
        /// Repository name (identifies the sender)
        #[arg(long)]
        repo: Vec<String>,

        /// Recipient agent name (or "all")
        #[arg(long)]
        to: String,

        /// Signal verb (e.g., review, request, announce, question, blocker)
        #[arg(long)]
        verb: String,

        /// Signal status (e.g., approved, blocked, ready)
        #[arg(long)]
        status: Option<String>,

        /// Free-text note
        #[arg(long)]
        note: Option<String>,

        /// Comma-separated key:value detail pairs (e.g., "surface:cap-output,chain:confirmed")
        #[arg(long)]
        details: Option<String>,

        /// Link to a parent reflection ID to thread signals
        #[arg(long)]
        follows: Option<String>,

        /// Domain tag for classification
        #[arg(long)]
        domain: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
    },

    /// Read the bullpen or check for unread posts
    #[command(alias = "bp", alias = "board")]
    Bullpen {
        /// Repository name (identifies who is reading)
        #[arg(long)]
        repo: String,

        /// Only show unread count instead of full bullpen
        #[arg(long)]
        count: bool,

        /// Show only signals (structured coordination messages)
        #[arg(long, conflicts_with = "musings")]
        signals: bool,

        /// Show only musings (natural language posts)
        #[arg(long, conflicts_with = "signals")]
        musings: bool,
    },

    /// Surface cross-repo highlights for a session start
    Surface {
        /// Repository name
        #[arg(long)]
        repo: String,
    },

    /// Rebuild the search index from the database
    Reindex,

    /// Compute embeddings for all reflections that are missing them
    Backfill,

    /// Show reflection statistics
    Stats {
        /// Repository name (omit for all repos)
        #[arg(long)]
        repo: Option<String>,
    },

    /// Start the web dashboard
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "3131")]
        port: u16,
    },

    /// Check your work state, team needs, and recent changes
    Status {
        /// Repository name
        #[arg(long)]
        repo: String,
    },

    /// Show what the team needs help with
    Needs {
        /// Repository name
        #[arg(long)]
        repo: String,
    },

    /// Announce completed work and notify blocked agents
    Done {
        /// Repository name
        #[arg(long)]
        repo: String,

        /// Description of what was completed
        #[arg(long)]
        text: String,
    },

    /// Manage delegated tasks between agents
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Manage scheduled bullpen posts
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },

    /// Watch for signals and auto-wake sleeping agents
    Watch,
}

#[derive(Subcommand)]
enum TaskAction {
    /// Create a new task for another agent
    Create {
        /// Sender repository name
        #[arg(long)]
        from: String,

        /// Target repository name
        #[arg(long)]
        to: String,

        /// Task description
        #[arg(long)]
        text: String,

        /// Additional context for the task
        #[arg(long)]
        context: Option<String>,

        /// Priority: low, med, high (default: med)
        #[arg(long, default_value = "med", value_parser = ["low", "med", "high"])]
        priority: String,
    },

    /// List tasks for a repo
    List {
        /// Repository name
        #[arg(long)]
        repo: String,

        /// Show outbound tasks (tasks created by this repo) instead of inbound
        #[arg(long)]
        from: bool,
    },

    /// Accept a pending task
    Accept {
        /// Task ID
        #[arg(long)]
        id: String,
    },

    /// Mark an accepted task as done
    Done {
        /// Task ID
        #[arg(long)]
        id: String,

        /// Completion note
        #[arg(long)]
        note: Option<String>,
    },

    /// Block an accepted task
    Block {
        /// Task ID
        #[arg(long)]
        id: String,

        /// Reason for blocking
        #[arg(long)]
        reason: Option<String>,
    },

    /// Unblock a blocked task (returns to accepted)
    Unblock {
        /// Task ID
        #[arg(long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// Create a new scheduled bullpen post
    Create {
        /// Human-readable name for the schedule
        #[arg(long)]
        name: String,

        /// Cron expression: "HH:MM" for daily or "*/Nm" for every N minutes
        #[arg(long)]
        cron: String,

        /// Text to post to the bullpen when the schedule fires
        #[arg(long)]
        command: String,

        /// Repository name for the post
        #[arg(long)]
        repo: String,

        /// Active window start time (HH:MM UTC). Only fires within the window. Requires --active-end.
        #[arg(long, requires = "active_end")]
        active_start: Option<String>,

        /// Active window end time (HH:MM UTC). Only fires within the window. Requires --active-start.
        #[arg(long, requires = "active_start")]
        active_end: Option<String>,
    },

    /// List all schedules
    List,

    /// Enable a schedule
    Enable {
        /// Schedule ID
        #[arg(long)]
        id: String,
    },

    /// Disable a schedule
    Disable {
        /// Schedule ID
        #[arg(long)]
        id: String,
    },

    /// Delete a schedule
    Delete {
        /// Schedule ID
        #[arg(long)]
        id: String,
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

/// Run a compound command (text or transcript) across multiple repos with metadata.
#[allow(clippy::too_many_arguments)]
fn run_compound_command_with_meta(
    db: &db::Database,
    index: &search::SearchIndex,
    repos: &[String],
    text: &Option<String>,
    transcript: &Option<PathBuf>,
    meta: &db::ReflectionMeta,
    from_text: fn(
        &db::Database,
        &search::SearchIndex,
        &str,
        &str,
        &db::ReflectionMeta,
    ) -> error::Result<()>,
    from_transcript: fn(
        &db::Database,
        &search::SearchIndex,
        &str,
        &std::path::Path,
        &db::ReflectionMeta,
    ) -> error::Result<()>,
    label: &str,
) -> error::Result<()> {
    if text.is_none() && transcript.is_none() {
        return Err(error::LegionError::NoReflectionInput);
    }

    let mut had_error = false;
    for r in repos {
        let result = match (text, transcript) {
            (Some(t), None) => from_text(db, index, r, t, meta),
            (None, Some(path)) => from_transcript(db, index, r, path, meta),
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

/// Try to load the embedding model. Returns None if not available.
///
/// Logs a warning to stderr on failure so degraded hybrid search is visible.
fn try_load_embed_model() -> Option<embed::EmbedModel> {
    match embed::EmbedModel::load() {
        Ok(model) => Some(model),
        Err(e) => {
            eprintln!("[legion] embedding model unavailable, falling back to BM25: {e}");
            None
        }
    }
}

/// Compute and store embeddings for all reflections that are missing them.
fn backfill_embeddings(db: &db::Database, model: &embed::EmbedModel) -> error::Result<usize> {
    let missing = db.get_ids_without_embeddings()?;
    let mut count: usize = 0;

    for (id, text) in &missing {
        match model.encode_one(text) {
            Ok(embedding) => {
                let bytes = embed::embedding_to_bytes(&embedding);
                if db.store_embedding(id, &bytes)? {
                    count += 1;
                }
            }
            Err(e) => {
                eprintln!("[legion] warning: failed to embed {}: {}", id, e);
            }
        }
    }

    Ok(count)
}

fn main() -> error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Reflect {
            repo,
            text,
            transcript,
            domain,
            tags,
            follows,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;
            let meta = db::ReflectionMeta {
                domain,
                tags,
                parent_id: follows,
            };

            run_compound_command_with_meta(
                &database,
                &index,
                &repo,
                &text,
                &transcript,
                &meta,
                reflect::reflect_from_text_with_meta,
                reflect::reflect_from_transcript_with_meta,
                "storing reflection",
            )?;

            // Compute embeddings for new reflections (silent fail if model unavailable)
            if let Some(model) = try_load_embed_model() {
                let n = backfill_embeddings(&database, &model)?;
                if n > 0 {
                    eprintln!("[legion] embedded {} reflections", n);
                }
            }
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
                // Try hybrid (BM25 + cosine) recall, fall back to BM25-only
                match try_load_embed_model() {
                    Some(model) => {
                        recall::recall(&database, &index, &model, &repo, &context, limit)?
                    }
                    None => recall::recall_bm25(&database, &index, &repo, &context, limit)?,
                }
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

            let result = match try_load_embed_model() {
                Some(model) => recall::consult(&database, &index, &model, &context, limit)?,
                None => recall::consult_bm25(&database, &index, &context, limit)?,
            };
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
            domain,
            tags,
            follows,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;
            let meta = db::ReflectionMeta {
                domain,
                tags,
                parent_id: follows,
            };

            run_compound_command_with_meta(
                &database,
                &index,
                &repo,
                &text,
                &transcript,
                &meta,
                board::post_from_text_with_meta,
                board::post_from_transcript_with_meta,
                "posting",
            )?;

            // Compute embeddings for new posts
            if let Some(model) = try_load_embed_model() {
                let n = backfill_embeddings(&database, &model)?;
                if n > 0 {
                    eprintln!("[legion] embedded {} posts", n);
                }
            }
        }
        Commands::Signal {
            repo,
            to,
            verb,
            status,
            note,
            details,
            follows,
            domain,
            tags,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;

            let detail_pairs: Vec<(String, String)> = details
                .as_deref()
                .map(|d| {
                    d.split(',')
                        .filter_map(|pair| {
                            let pair = pair.trim();
                            pair.find(':').map(|pos| {
                                (
                                    pair[..pos].trim().to_string(),
                                    pair[pos + 1..].trim().to_string(),
                                )
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let text = signal::format_signal(
                &to,
                &verb,
                status.as_deref(),
                note.as_deref(),
                &detail_pairs,
            );

            let meta = db::ReflectionMeta {
                domain,
                tags,
                parent_id: follows,
            };

            for r in &repo {
                board::post_from_text_with_meta(&database, &index, r, &text, &meta)?;
            }

            // Compute embeddings for new signals
            if let Some(model) = try_load_embed_model() {
                let n = backfill_embeddings(&database, &model)?;
                if n > 0 {
                    eprintln!("[legion] embedded {} signals", n);
                }
            }
        }
        Commands::Boost { id } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            if database.boost_reflection(&id)? {
                eprintln!("[legion] boosted reflection {}", id);
            } else {
                eprintln!("[legion] reflection not found: {}", id);
            }
        }
        Commands::Chain { id } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            let chain = database.get_chain(&id)?;
            if chain.is_empty() {
                eprintln!("[legion] no chain found for {}", id);
            } else {
                for (i, r) in chain.iter().enumerate() {
                    let prefix = if i == 0 {
                        String::new()
                    } else {
                        "  ".repeat(i) + "-> "
                    };
                    let date = db::format_date(&r.created_at);
                    let domain_tag = r
                        .domain
                        .as_deref()
                        .map(|d| format!(" [{}]", d))
                        .unwrap_or_default();
                    let truncated: String = r.text.chars().take(80).collect();
                    let ellipsis = if r.text.len() > 80 { "..." } else { "" };
                    eprintln!(
                        "{}{} {}{}: {}{}",
                        prefix, r.repo, date, domain_tag, truncated, ellipsis
                    );
                }
            }
        }
        Commands::Bullpen {
            repo,
            count,
            signals,
            musings,
        } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            if count {
                let post_count = board::bullpen_count(&database, &repo)?;
                let task_count = task::count_pending_inbound(&database, &repo)?;
                let output = board::format_bullpen_count(post_count, task_count);
                if !output.is_empty() {
                    println!("{output}");
                }
            } else {
                let filter = if signals {
                    board::BullpenFilter::SignalsOnly
                } else if musings {
                    board::BullpenFilter::MusingsOnly
                } else {
                    board::BullpenFilter::All
                };
                let posts = board::bullpen_filtered(&database, &repo, filter)?;
                let mut output = board::format_bullpen(&posts);
                if filter == board::BullpenFilter::All {
                    let pending_tasks = task::get_pending_inbound(&database, &repo)?;
                    let task_output = task::format_pending_for_surface(&pending_tasks);
                    output.push_str(&task_output);
                }
                if !output.is_empty() {
                    print!("{output}");
                }
            }
        }
        Commands::Reindex => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;

            let reflections = database.get_all_for_reindex()?;
            let count = reflections.len();
            index.rebuild(&reflections)?;
            eprintln!("[legion] reindexed {} reflections", count);
        }
        Commands::Backfill => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            let model = embed::EmbedModel::load()?;
            let count = backfill_embeddings(&database, &model)?;
            eprintln!("[legion] embedded {} reflections", count);
        }
        Commands::Init { force } => {
            init::init(force)?;
        }
        Commands::Surface { repo } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            let result = surface::surface(&database, &repo)?;
            let output = surface::format_surface(&result, &repo);
            if !output.is_empty() {
                print!("{output}");
            }
        }
        Commands::Stats { repo } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            stats::stats(&database, repo.as_deref())?;
        }
        Commands::Serve { port } => {
            serve::run_server(port, data_dir()?)?;
        }
        Commands::Status { repo } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let output = status::get_status(&database, &repo)?;
            let formatted = status::format_status(&output);
            if formatted.is_empty() {
                println!(
                    "[Legion] Status for {}: all clear. Check `gh issue list` for GitHub issues.",
                    repo
                );
            } else {
                print!("{formatted}");
            }
        }
        Commands::Needs { repo } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let items = status::get_needs(&database, &repo)?;
            print!("{}", status::format_needs(&repo, &items));
        }
        Commands::Done { repo, text } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;
            let index = search::SearchIndex::open(&base.join("index"))?;

            let announcement = format!("{repo} completed: {text}");
            let reflection = database.insert_reflection_with_meta(
                &repo,
                &announcement,
                "team",
                &db::ReflectionMeta::default(),
            )?;
            if let Err(e) = index.add(&reflection.id, &reflection.repo, &announcement) {
                eprintln!("[legion] search index add failed: {e}");
            }
            eprintln!("[legion] done: {text}");

            let blocked_agents = status::find_blocked_agents(&database, &repo)?;
            for agent in &blocked_agents {
                let notify_text = format!(
                    "@{agent} announce from {repo} -- {repo} completed: {text}. Your blocker may be cleared."
                );
                let notify_ref = database.insert_reflection_with_meta(
                    &repo,
                    &notify_text,
                    "team",
                    &db::ReflectionMeta::default(),
                )?;
                if let Err(e) = index.add(&notify_ref.id, &notify_ref.repo, &notify_text) {
                    eprintln!("[legion] search index add failed: {e}");
                }
                eprintln!("[legion] notified {agent} (was blocked on {repo})");
            }

            if blocked_agents.is_empty() {
                eprintln!("[legion] no blocked agents found");
            }
        }
        Commands::Task { action } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            match action {
                TaskAction::Create {
                    from,
                    to,
                    text,
                    context,
                    priority,
                } => {
                    let id = task::create_task(
                        &database,
                        &from,
                        &to,
                        &text,
                        context.as_deref(),
                        &priority,
                    )?;
                    eprintln!("[legion] task created: {} -> {} ({})", from, to, id);
                }
                TaskAction::List { repo, from } => {
                    let direction = if from {
                        task::Direction::Outbound
                    } else {
                        task::Direction::Inbound
                    };
                    let tasks = task::list_tasks(&database, &repo, direction)?;
                    let output = task::format_task_list(&tasks, &repo, direction);
                    if output.is_empty() {
                        eprintln!("[legion] no tasks found");
                    } else {
                        print!("{output}");
                    }
                }
                TaskAction::Accept { id } => {
                    task::accept_task(&database, &id)?;
                    eprintln!("[legion] task accepted: {}", id);
                }
                TaskAction::Done { id, note } => {
                    task::complete_task(&database, &id, note.as_deref())?;
                    eprintln!("[legion] task completed: {}", id);
                }
                TaskAction::Block { id, reason } => {
                    task::block_task(&database, &id, reason.as_deref())?;
                    eprintln!("[legion] task blocked: {}", id);
                }
                TaskAction::Unblock { id } => {
                    task::unblock_task(&database, &id)?;
                    eprintln!("[legion] task unblocked: {}", id);
                }
            }
        }
        Commands::Schedule { action } => {
            let base = data_dir()?;
            let database = db::Database::open(&base.join("legion.db"))?;

            match action {
                ScheduleAction::Create {
                    name,
                    cron,
                    command,
                    repo,
                    active_start,
                    active_end,
                } => {
                    let id = database.insert_schedule(
                        &name,
                        &cron,
                        &command,
                        &repo,
                        active_start.as_deref(),
                        active_end.as_deref(),
                    )?;
                    eprintln!("[legion] schedule created: {} ({})", name, id);
                }
                ScheduleAction::List => {
                    let schedules = database.list_schedules()?;
                    if schedules.is_empty() {
                        eprintln!("[legion] no schedules");
                    } else {
                        println!("[Legion] Schedules:");
                        for s in &schedules {
                            let status = if s.enabled { "on" } else { "off" };
                            let next = if s.enabled { &s.next_run } else { "-" };
                            let truncated: String = s.command.chars().take(20).collect();
                            let ellipsis = if s.command.len() > 20 { "..." } else { "" };
                            let window = match (&s.active_start, &s.active_end) {
                                (Some(start), Some(end)) => format!("  window: {start}-{end}"),
                                _ => String::new(),
                            };
                            println!(
                                "  [{status}] {cron:<6} {name:<20} \"{text}{ellip}\"  ({repo})  next: {next}{window}",
                                status = status,
                                cron = s.cron,
                                name = s.name,
                                text = truncated,
                                ellip = ellipsis,
                                repo = s.repo,
                                next = next,
                                window = window,
                            );
                        }
                    }
                }
                ScheduleAction::Enable { id } => {
                    if database.toggle_schedule(&id, true)? {
                        eprintln!("[legion] schedule enabled: {}", id);
                    } else {
                        eprintln!("[legion] schedule not found: {}", id);
                    }
                }
                ScheduleAction::Disable { id } => {
                    if database.toggle_schedule(&id, false)? {
                        eprintln!("[legion] schedule disabled: {}", id);
                    } else {
                        eprintln!("[legion] schedule not found: {}", id);
                    }
                }
                ScheduleAction::Delete { id } => {
                    if database.delete_schedule(&id)? {
                        eprintln!("[legion] schedule deleted: {}", id);
                    } else {
                        eprintln!("[legion] schedule not found: {}", id);
                    }
                }
            }
        }
        Commands::Watch => {
            let base = data_dir()?;
            watch::run(&base)?;
        }
    }

    Ok(())
}
