use hash::cli::Algorithm;
use hash::hash::{blake3, compute, sha256};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
const EMPTY_SHA256_HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const EMPTY_BLAKE3_HEX: &str = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
const ABC_SHA256_HEX: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const ABC_BLAKE3_HEX: &str = "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

fn temp_file_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let ts_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "hash-engine-{}-{ts_nanos}-{counter}.bin",
        std::process::id()
    ));
    path
}

fn write_temp_file(contents: &[u8]) -> PathBuf {
    let path = temp_file_path();
    std::fs::write(&path, contents).expect("write temp file");
    path
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[test]
fn hashes_are_prefixed_and_lowercase_hex() {
    let path = write_temp_file(b"hash-engine");

    let sha = sha256::hash_file(&path).expect("hash with sha256");
    let sha_hex = sha.strip_prefix("sha256:").expect("sha256 prefix");
    assert_eq!(sha_hex.len(), 64);
    assert!(is_lower_hex(sha_hex));

    let blake = blake3::hash_file(&path).expect("hash with blake3");
    let blake_hex = blake.strip_prefix("blake3:").expect("blake3 prefix");
    assert_eq!(blake_hex.len(), 64);
    assert!(is_lower_hex(blake_hex));

    let _ = std::fs::remove_file(path);
}

#[test]
fn dispatcher_matches_algorithm_specific_hashers() {
    let path = write_temp_file(b"dispatcher-test");

    let via_dispatch_sha = compute::hash_file(&path, Algorithm::Sha256).expect("dispatch sha256");
    let direct_sha = sha256::hash_file(&path).expect("direct sha256");
    assert_eq!(via_dispatch_sha, direct_sha);

    let via_dispatch_blake = compute::hash_file(&path, Algorithm::Blake3).expect("dispatch blake3");
    let direct_blake = blake3::hash_file(&path).expect("direct blake3");
    assert_eq!(via_dispatch_blake, direct_blake);

    let _ = std::fs::remove_file(path);
}

#[test]
fn large_inputs_hash_deterministically() {
    let mut data = Vec::with_capacity(256 * 1024);
    while data.len() < 256 * 1024 {
        data.extend(0_u8..=255);
    }
    data.truncate(256 * 1024);

    let path = write_temp_file(&data);

    let first_sha = compute::hash_file(&path, Algorithm::Sha256).expect("first sha256");
    let second_sha = compute::hash_file(&path, Algorithm::Sha256).expect("second sha256");
    assert_eq!(first_sha, second_sha);

    let first_blake = compute::hash_file(&path, Algorithm::Blake3).expect("first blake3");
    let second_blake = compute::hash_file(&path, Algorithm::Blake3).expect("second blake3");
    assert_eq!(first_blake, second_blake);

    let _ = std::fs::remove_file(path);
}

#[test]
fn empty_file_matches_known_vectors() {
    let path = write_temp_file(b"");

    let sha = sha256::hash_file(&path).expect("empty sha256");
    assert_eq!(sha, format!("sha256:{EMPTY_SHA256_HEX}"));

    let blake = blake3::hash_file(&path).expect("empty blake3");
    assert_eq!(blake, format!("blake3:{EMPTY_BLAKE3_HEX}"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn abc_matches_known_vectors() {
    let path = write_temp_file(b"abc");

    let sha = sha256::hash_file(&path).expect("abc sha256");
    assert_eq!(sha, format!("sha256:{ABC_SHA256_HEX}"));

    let blake = blake3::hash_file(&path).expect("abc blake3");
    assert_eq!(blake, format!("blake3:{ABC_BLAKE3_HEX}"));

    let _ = std::fs::remove_file(path);
}
