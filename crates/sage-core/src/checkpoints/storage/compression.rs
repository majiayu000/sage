//! Content compression utilities for checkpoint storage

use crate::error::{SageError, SageResult};

/// Compress content using gzip
pub(super) fn compress_content(content: &str) -> SageResult<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(content.as_bytes())
        .map_err(|e| SageError::storage(format!("Failed to compress content: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| SageError::storage(format!("Failed to finish compression: {}", e)))
}

/// Decompress content using gzip
pub(super) fn decompress_content(compressed: &[u8]) -> SageResult<String> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(compressed);
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .map_err(|e| SageError::storage(format!("Failed to decompress content: {}", e)))?;
    Ok(decompressed)
}

/// Compute content hash
pub(super) fn compute_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
