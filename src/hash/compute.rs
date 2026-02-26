use crate::cli::Algorithm;
use std::io;
use std::path::Path;

pub fn hash_file(path: &Path, algorithm: Algorithm) -> Result<String, io::Error> {
    match algorithm {
        Algorithm::Sha256 => super::sha256::hash_file(path),
        Algorithm::Blake3 => super::blake3::hash_file(path),
    }
}
