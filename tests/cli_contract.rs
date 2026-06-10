use clap::{CommandFactory, Parser, error::ErrorKind};
use hashbytes::cli::{Cli, Command, RobotDocsAction, WitnessAction};
use std::path::PathBuf;

#[test]
fn parses_default_root_command() {
    let cli = Cli::try_parse_from(["hashbytes"]).expect("default parse should succeed");
    assert!(cli.command.is_none());
    assert!(cli.input.is_none());
    assert!(!cli.json);
    assert!(!cli.robot_triage);
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
        "--json",
        "--robot-triage",
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
    assert!(cli.json);
    assert!(cli.robot_triage);
    assert_eq!(cli.algorithm, "BLAKE3");
    assert_eq!(cli.jobs, Some(4));
    assert!(cli.no_witness);
    assert!(cli.progress);
    assert!(cli.describe);
    assert!(cli.schema);
}

#[test]
fn parses_top_level_agent_surfaces() -> Result<(), String> {
    let capabilities = Cli::try_parse_from(["hashbytes", "capabilities", "--json"])
        .map_err(|err| err.to_string())?;
    match capabilities.command {
        Some(Command::Capabilities { json }) => assert!(json),
        _ => return Err("expected capabilities command".to_owned()),
    }

    let robot_docs =
        Cli::try_parse_from(["hashbytes", "robot-docs", "guide"]).map_err(|err| err.to_string())?;
    match robot_docs.command {
        Some(Command::RobotDocs {
            action: Some(RobotDocsAction::Guide),
        }) => {}
        _ => return Err("expected robot-docs guide command".to_owned()),
    }

    let triage =
        Cli::try_parse_from(["hashbytes", "--robot-triage"]).map_err(|err| err.to_string())?;
    assert!(triage.robot_triage);
    Ok(())
}

#[test]
fn parses_witness_query_subcommand() -> Result<(), String> {
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
    .map_err(|err| err.to_string())?;

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
        _ => return Err("expected witness query command".to_owned()),
    }

    Ok(())
}

#[test]
fn parses_witness_last_and_count_subcommands() -> Result<(), String> {
    let last = Cli::try_parse_from(["hashbytes", "witness", "last", "--json"])
        .map_err(|err| err.to_string())?;
    match last.command {
        Some(Command::Witness {
            action: WitnessAction::Last { json },
        }) => assert!(json),
        _ => return Err("expected witness last command".to_owned()),
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
    .map_err(|err| err.to_string())?;
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
        _ => return Err("expected witness count command".to_owned()),
    }

    Ok(())
}

#[test]
fn exposes_expected_long_flags_and_version_behavior() -> Result<(), String> {
    let command = Cli::command();
    let long_flags: Vec<_> = command
        .get_arguments()
        .filter_map(|arg| arg.get_long())
        .collect();

    for flag in [
        "algorithm",
        "json",
        "robot-triage",
        "jobs",
        "no-witness",
        "progress",
        "describe",
        "schema",
    ] {
        assert!(long_flags.contains(&flag), "missing --{flag}");
    }

    let Err(version_err) = Cli::try_parse_from(["hashbytes", "--version"]) else {
        return Err("version should short-circuit".to_owned());
    };
    assert_eq!(version_err.kind(), ErrorKind::DisplayVersion);
    Ok(())
}
