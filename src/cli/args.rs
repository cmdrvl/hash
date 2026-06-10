use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hashbytes")]
#[command(about = "Streaming content hashing for manifest enrichment")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// JSONL manifest file (default: stdin)
    pub input: Option<PathBuf>,

    /// Accept structured-output intent; hash output is already JSONL
    #[arg(long)]
    pub json: bool,

    /// Emit one machine-readable triage report without reading input
    #[arg(long)]
    pub robot_triage: bool,

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
    /// Describe agent-facing capabilities
    Capabilities {
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
    /// Print agent-facing usage guidance
    RobotDocs {
        #[command(subcommand)]
        action: Option<RobotDocsAction>,
    },
    /// Query the witness ledger
    Witness {
        #[command(subcommand)]
        action: WitnessAction,
    },
    /// Inspect hashbytes health and agent-facing capabilities
    Doctor {
        /// Emit one machine-readable triage report
        #[arg(long)]
        robot_triage: bool,

        /// Emit JSON for the default health report
        #[arg(long)]
        json: bool,

        /// Repair mode is unavailable; prints safe alternatives
        #[arg(long, hide = true)]
        fix: bool,

        #[command(subcommand)]
        action: Option<DoctorAction>,
    },
}

#[derive(Subcommand)]
pub enum RobotDocsAction {
    /// Print the agent guide
    Guide,
}

#[derive(Subcommand)]
pub enum DoctorAction {
    /// Check the compiled binary contract without reading input data
    Health {
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
    /// Describe read-only doctor capabilities
    Capabilities {
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
    /// Print agent-facing doctor usage notes
    RobotDocs,
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
