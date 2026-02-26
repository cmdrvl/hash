use hash::cli::{Cli, Command, WitnessAction};
use hash::run_with_cli;

fn base_cli() -> Cli {
    Cli {
        command: None,
        input: None,
        algorithm: "sha256".to_string(),
        jobs: None,
        no_witness: false,
        progress: false,
        describe: false,
        schema: false,
    }
}

#[test]
fn describe_short_circuits_before_main_workflow() {
    let mut cli = base_cli();
    cli.describe = true;
    cli.algorithm = "invalid".to_string();
    assert_eq!(run_with_cli(cli), 0);
}

#[test]
fn schema_short_circuits_before_main_workflow() {
    let mut cli = base_cli();
    cli.schema = true;
    cli.algorithm = "invalid".to_string();
    assert_eq!(run_with_cli(cli), 0);
}

#[test]
fn witness_subcommands_route_to_witness_handler_exit_codes() {
    let mut cli = base_cli();
    cli.command = Some(Command::Witness {
        action: WitnessAction::Query {
            tool: None,
            since: None,
            until: None,
            outcome: None,
            input_hash: None,
            limit: None,
            json: true,
        },
    });

    assert_eq!(run_with_cli(cli), 1);
}

#[test]
fn invalid_algorithm_returns_refusal_exit_code() {
    let mut cli = base_cli();
    cli.algorithm = "md5".to_string();
    assert_eq!(run_with_cli(cli), 2);
}

#[test]
fn default_main_path_returns_all_hashed_exit_code() {
    assert_eq!(run_with_cli(base_cli()), 0);
}
