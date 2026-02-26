use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub fn hash_file(path: &Path) -> Result<String, io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file); // 64 KB buffer

    let mut hasher = Sha256::new();
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }
        hasher.update(buf);
        let len = buf.len();
        reader.consume(len);
    }

    Ok(format!("sha256:{:x}", hasher.finalize()))
}
