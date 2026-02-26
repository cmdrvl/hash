use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hash")]
#[command(about = "Streaming content hashing for manifest enrichment")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// JSONL manifest file (default: stdin)
    pub input: Option<PathBuf>,

    /// Hash algorithm: sha256 or blake3
    #[arg(long, default_value = "sha256")]
    pub algorithm: String,

    /// Number of parallel workers (default: CPU count)
    #[arg(long)]
    pub jobs: Option<usize>,

    /// Suppress witness ledger recording
    #[arg(long)]
    pub no_witness: bool,

    /// Emit progress to stderr
    #[arg(long)]
    pub progress: bool,

    /// Print operator.json and exit
    #[arg(long)]
    pub describe: bool,

    /// Print JSON Schema and exit
    #[arg(long)]
    pub schema: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Query the witness ledger
    Witness {
        #[command(subcommand)]
        action: WitnessAction,
    },
}

#[derive(Subcommand)]
pub enum WitnessAction {
    /// Query witness records with filters
    Query {
        /// Tool name filter
        #[arg(long)]
        tool: Option<String>,

        /// Since timestamp (ISO 8601)
        #[arg(long)]
        since: Option<String>,

        /// Until timestamp (ISO 8601)
        #[arg(long)]
        until: Option<String>,

        /// Outcome filter
        #[arg(long)]
        outcome: Option<String>,

        /// Input hash substring filter
        #[arg(long)]
        input_hash: Option<String>,

        /// Limit number of results
        #[arg(long)]
        limit: Option<usize>,

        /// JSON output format
        #[arg(long)]
        json: bool,
    },
    /// Get the most recent witness record
    Last {
        /// JSON output format
        #[arg(long)]
        json: bool,
    },
    /// Count witness records matching filters
    Count {
        /// Tool name filter
        #[arg(long)]
        tool: Option<String>,

        /// Since timestamp (ISO 8601)
        #[arg(long)]
        since: Option<String>,

        /// Until timestamp (ISO 8601)
        #[arg(long)]
        until: Option<String>,

        /// Outcome filter
        #[arg(long)]
        outcome: Option<String>,

        /// Input hash substring filter
        #[arg(long)]
        input_hash: Option<String>,

        /// JSON output format
        #[arg(long)]
        json: bool,
    },
}
