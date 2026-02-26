use clap::{CommandFactory, Parser, error::ErrorKind};
use hash::cli::{Cli, Command, WitnessAction};
use std::path::PathBuf;

#[test]
fn parses_default_root_command() {
    let cli = Cli::try_parse_from(["hash"]).expect("default parse should succeed");
    assert!(cli.command.is_none());
    assert!(cli.input.is_none());
    assert_eq!(cli.algorithm, "sha256");
    assert!(cli.jobs.is_none());
    assert!(!cli.no_witness);
    assert!(!cli.progress);
    assert!(!cli.describe);
    assert!(!cli.schema);
}

#[test]
fn parses_input_and_flags() {
    let cli = Cli::try_parse_from([
        "hash",
        "manifest.jsonl",
        "--algorithm",
        "BLAKE3",
        "--jobs",
        "4",
        "--no-witness",
        "--progress",
        "--describe",
        "--schema",
    ])
    .expect("flag parse should succeed");

    assert_eq!(cli.input, Some(PathBuf::from("manifest.jsonl")));
    assert_eq!(cli.algorithm, "BLAKE3");
    assert_eq!(cli.jobs, Some(4));
    assert!(cli.no_witness);
    assert!(cli.progress);
    assert!(cli.describe);
    assert!(cli.schema);
}

#[test]
fn parses_witness_query_subcommand() {
    let cli = Cli::try_parse_from([
        "hash",
        "witness",
        "query",
        "--tool",
        "hash",
        "--since",
        "2026-01-01T00:00:00Z",
        "--until",
        "2026-02-01T00:00:00Z",
        "--outcome",
        "PARTIAL",
        "--input-hash",
        "abc",
        "--limit",
        "10",
        "--json",
    ])
    .expect("witness query parse should succeed");

    match cli.command {
        Some(Command::Witness {
            action:
                WitnessAction::Query {
                    tool,
                    since,
                    until,
                    outcome,
                    input_hash,
                    limit,
                    json,
                },
        }) => {
            assert_eq!(tool.as_deref(), Some("hash"));
            assert_eq!(since.as_deref(), Some("2026-01-01T00:00:00Z"));
            assert_eq!(until.as_deref(), Some("2026-02-01T00:00:00Z"));
            assert_eq!(outcome.as_deref(), Some("PARTIAL"));
            assert_eq!(input_hash.as_deref(), Some("abc"));
            assert_eq!(limit, Some(10));
            assert!(json);
        }
        _ => panic!("expected witness query command"),
    }
}

#[test]
fn parses_witness_last_and_count_subcommands() {
    let last = Cli::try_parse_from(["hash", "witness", "last", "--json"])
        .expect("witness last parse should succeed");
    match last.command {
        Some(Command::Witness {
            action: WitnessAction::Last { json },
        }) => assert!(json),
        _ => panic!("expected witness last command"),
    }

    let count = Cli::try_parse_from([
        "hash",
        "witness",
        "count",
        "--tool",
        "hash",
        "--outcome",
        "ALL_HASHED",
        "--json",
    ])
    .expect("witness count parse should succeed");
    match count.command {
        Some(Command::Witness {
            action:
                WitnessAction::Count {
                    tool,
                    since,
                    until,
                    outcome,
                    input_hash,
                    json,
                },
        }) => {
            assert_eq!(tool.as_deref(), Some("hash"));
            assert!(since.is_none());
            assert!(until.is_none());
            assert_eq!(outcome.as_deref(), Some("ALL_HASHED"));
            assert!(input_hash.is_none());
            assert!(json);
        }
        _ => panic!("expected witness count command"),
    }
}

#[test]
fn exposes_expected_long_flags_and_version_behavior() {
    let command = Cli::command();
    let long_flags: Vec<_> = command
        .get_arguments()
        .filter_map(|arg| arg.get_long())
        .collect();

    for flag in [
        "algorithm",
        "jobs",
        "no-witness",
        "progress",
        "describe",
        "schema",
    ] {
        assert!(long_flags.contains(&flag), "missing --{flag}");
    }

    let version_err = match Cli::try_parse_from(["hash", "--version"]) {
        Ok(_) => panic!("version should short-circuit"),
        Err(err) => err,
    };
    assert_eq!(version_err.kind(), ErrorKind::DisplayVersion);
}
