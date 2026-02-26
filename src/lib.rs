#![forbid(unsafe_code)]

pub mod cli;
pub mod hash;
pub mod output;
pub mod pipeline;
pub mod progress;
pub mod refusal;
pub mod witness;

/// Main entry point that handles all errors internally and returns exit code
pub fn run() -> u8 {
    cli::exit_code(cli::Outcome::AllHashed)
}
