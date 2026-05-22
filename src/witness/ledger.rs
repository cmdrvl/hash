use super::record::{WitnessRecord, canonical_json};
use serde_json::{Value, json};
use std::{
    env,
    ffi::OsString,
    fs,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub fn default_witness_path() -> PathBuf {
    default_witness_path_from_env(|key| env::var_os(key))
}

fn default_witness_path_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if let Some(path) = get_env("EPISTEMIC_WITNESS")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    cmdrvl_root_from_env(get_env)
        .join("state")
        .join("witness")
        .join("witness.jsonl")
}

fn cmdrvl_root_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if let Some(home) = get_env("HOME")
        .or_else(|| get_env("USERPROFILE"))
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(home).join(".cmdrvl");
    }

    PathBuf::from(".cmdrvl")
}

pub fn default_witness_path_for_append() -> io::Result<PathBuf> {
    ensure_ledger_migrated()?;
    prepare_canonical_tree_from_env(|key| env::var_os(key))?;
    Ok(default_witness_path())
}

pub fn ensure_ledger_migrated() -> io::Result<()> {
    ensure_ledger_migrated_from_env(|key| env::var_os(key))
}

fn ensure_ledger_migrated_from_env<F>(get_env: F) -> io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if get_env("EPISTEMIC_WITNESS").is_some_and(|value| !value.is_empty()) {
        return Ok(());
    }

    let canonical = default_witness_path_from_env(get_env);
    let Some(legacy) = legacy_ledger_paths_from_env(get_env)
        .into_iter()
        .find(|path| path != &canonical && path.exists())
    else {
        return Ok(());
    };

    let root = cmdrvl_root_from_env(get_env);
    prepare_canonical_tree_from_env(get_env)?;
    let notice_path = root.join("notices").join("deprecated-paths.jsonl");
    let migration_path = root.join("migrations").join("applied.jsonl");

    if canonical.exists() {
        append_record_once(
            &notice_path,
            deprecation_record(
                &legacy,
                &canonical,
                "legacy_path_present",
                "canonical_preferred",
            ),
        )?;
        return Ok(());
    }

    if let Some(parent) = canonical.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    fs::copy(&legacy, &canonical)?;
    let permissions = fs::metadata(&legacy)?.permissions();
    fs::set_permissions(&canonical, permissions)?;

    append_record_once(
        &migration_path,
        migration_record(&legacy, &canonical, "copied_legacy_to_canonical"),
    )?;
    append_record_once(
        &notice_path,
        deprecation_record(
            &legacy,
            &canonical,
            "legacy_path_migrated",
            "canonical_created",
        ),
    )?;

    Ok(())
}

fn prepare_canonical_tree_from_env<F>(get_env: F) -> io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if get_env("EPISTEMIC_WITNESS").is_some_and(|value| !value.is_empty()) {
        return Ok(());
    }

    let root = cmdrvl_root_from_env(get_env);
    for dir in [
        root.clone(),
        root.join("state"),
        root.join("state").join("witness"),
    ] {
        fs::create_dir_all(&dir)?;
        harden_directory(&dir)?;
    }

    Ok(())
}

fn legacy_ledger_paths_from_env<F>(get_env: F) -> Vec<PathBuf>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    let mut paths = Vec::new();

    if let Some(home) = get_env("HOME")
        .or_else(|| get_env("USERPROFILE"))
        .filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(home).join(".epistemic").join("witness.jsonl"));
    }

    paths.push(PathBuf::from(".epistemic").join("witness.jsonl"));
    paths
}

fn migration_record(source: &Path, destination: &Path, action: &str) -> Value {
    json!({
        "version": "cmdrvl.migration.v1",
        "tool": "hash",
        "path_class": "witness_ledger",
        "source_path": source.display().to_string(),
        "destination_path": destination.display().to_string(),
        "action": action,
        "outcome": "ok",
        "secret_values_recorded": false
    })
}

fn deprecation_record(source: &Path, destination: &Path, action: &str, outcome: &str) -> Value {
    json!({
        "version": "cmdrvl.deprecated_path_notice.v1",
        "tool": "hash",
        "path_class": "witness_ledger",
        "source_path": source.display().to_string(),
        "destination_path": destination.display().to_string(),
        "action": action,
        "outcome": outcome,
        "secret_values_recorded": false
    })
}

fn append_record_once(path: &Path, record: Value) -> io::Result<()> {
    if record_already_exists(path, &record)? {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{record}")?;
    Ok(())
}

fn record_already_exists(path: &Path, record: &Value) -> io::Result<bool> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(false);
    };

    Ok(contents.lines().any(|line| {
        let Ok(existing) = serde_json::from_str::<Value>(line) else {
            return false;
        };

        existing.get("tool") == record.get("tool")
            && existing.get("path_class") == record.get("path_class")
            && existing.get("source_path") == record.get("source_path")
            && existing.get("destination_path") == record.get("destination_path")
            && existing.get("action") == record.get("action")
    }))
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

pub fn append_record(path: &Path, record: &WitnessRecord) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(canonical_json(record).as_bytes())?;
    file.write_all(b"\n")
}

pub fn append_default_record(record: &WitnessRecord) -> io::Result<PathBuf> {
    let path = default_witness_path_for_append()?;
    append_record(&path, record)?;
    Ok(path)
}

pub fn last_record_id(path: &Path) -> Option<String> {
    let _ = ensure_ledger_migrated();

    let file = fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    let mut last_non_empty = None;
    for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            last_non_empty = Some(trimmed.to_owned());
        }
    }

    let last = last_non_empty?;
    let value: serde_json::Value = serde_json::from_str(&last).ok()?;
    value.get("id")?.as_str().map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{default_witness_path_from_env, ensure_ledger_migrated_from_env};
    use std::{ffi::OsString, fs, path::PathBuf, time::SystemTime};

    fn unique_home(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hash-witness-ledger-{}-{label}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp home should be created");
        path
    }

    fn canonical_witness_path(home: &std::path::Path) -> PathBuf {
        home.join(".cmdrvl")
            .join("state")
            .join("witness")
            .join("witness.jsonl")
    }

    #[test]
    fn empty_epistemic_witness_falls_back_to_cmdrvl_home() {
        let path = default_witness_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(OsString::new()),
            "HOME" => Some(OsString::from("/tmp/home")),
            _ => None,
        });

        assert_eq!(
            path,
            PathBuf::from("/tmp/home/.cmdrvl/state/witness/witness.jsonl")
        );
    }

    #[test]
    fn empty_home_falls_back_to_relative_cmdrvl_dir() {
        let path = default_witness_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => None,
            "HOME" => Some(OsString::new()),
            "USERPROFILE" => Some(OsString::new()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from(".cmdrvl/state/witness/witness.jsonl"));
    }

    #[test]
    fn explicit_epistemic_witness_override_still_wins() {
        let path = default_witness_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(OsString::from("/tmp/custom-witness.jsonl")),
            "HOME" => Some(OsString::from("/tmp/home")),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/custom-witness.jsonl"));
    }

    #[test]
    fn migrates_legacy_home_witness_to_cmdrvl_root() {
        let home = unique_home("legacy-migration");
        let legacy = home.join(".epistemic").join("witness.jsonl");
        fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("legacy parent");
        fs::write(&legacy, "{\"version\":\"witness.v0\"}\n").expect("legacy ledger");

        ensure_ledger_migrated_from_env(|key| match key {
            "HOME" => Some(home.as_os_str().to_owned()),
            "USERPROFILE" => None,
            "EPISTEMIC_WITNESS" => None,
            _ => None,
        })
        .expect("migration should succeed");

        assert_eq!(
            fs::read_to_string(canonical_witness_path(&home)).expect("canonical ledger"),
            "{\"version\":\"witness.v0\"}\n"
        );
        assert!(home.join(".cmdrvl/migrations/applied.jsonl").exists());
        assert!(home.join(".cmdrvl/notices/deprecated-paths.jsonl").exists());
    }

    #[test]
    fn migration_prefers_existing_canonical_ledger_without_overwrite() {
        let home = unique_home("canonical-present");
        let legacy = home.join(".epistemic").join("witness.jsonl");
        let canonical = canonical_witness_path(&home);
        fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("legacy parent");
        fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        fs::write(&legacy, "legacy\n").expect("legacy ledger");
        fs::write(&canonical, "canonical\n").expect("canonical ledger");

        ensure_ledger_migrated_from_env(|key| match key {
            "HOME" => Some(home.as_os_str().to_owned()),
            "USERPROFILE" => None,
            "EPISTEMIC_WITNESS" => None,
            _ => None,
        })
        .expect("migration should succeed");

        assert_eq!(
            fs::read_to_string(&canonical).expect("canonical ledger"),
            "canonical\n"
        );
        let notice = fs::read_to_string(home.join(".cmdrvl/notices/deprecated-paths.jsonl"))
            .expect("notice should exist");
        assert!(notice.contains("canonical_preferred"));
    }
}
