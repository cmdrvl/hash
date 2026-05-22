use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn hashbytes_command(label: &str) -> Command {
    let root = unique_root(label);
    let home = root.join("home");
    fs::create_dir_all(&home).expect("temporary home should be creatable");

    let mut command = Command::new(env!("CARGO_BIN_EXE_hashbytes"));
    command.env("HOME", &home);
    command.env("USERPROFILE", root.join("profile"));
    command
}

fn unique_root(label: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hashbytes-integration-{label}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("temporary root should be creatable");
    root
}
