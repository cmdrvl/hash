use serde_json::{Map, Value};

pub const HASH_VERSION: &str = "hash.v0";

pub fn set_hash_version(record: &mut Map<String, Value>) {
    record.insert("version".to_owned(), Value::String(HASH_VERSION.to_owned()));
}

pub fn mark_skipped(record: &mut Map<String, Value>) {
    record.insert("_skipped".to_owned(), Value::Bool(true));
    record.insert("bytes_hash".to_owned(), Value::Null);
    record.insert("hash_algorithm".to_owned(), Value::Null);
}
