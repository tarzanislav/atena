//! Content-addressed file store: every original file is kept once, under its BLAKE3 hash.
//!
//! Layout: `<root>/ab/cd/abcd…` (the first two bytes of the hash as directories, so no directory
//! grows huge). Files are written to a temporary name and renamed, so a crash never leaves a
//! partial file under a valid name. Text formats are stored gzip-compressed.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

/// A 32-byte BLAKE3 digest.
pub type Hash = [u8; 32];

/// The hash of `bytes`.
pub fn hash(bytes: &[u8]) -> Hash {
    *blake3::hash(bytes).as_bytes()
}

/// Lower-case hexadecimal form of a hash.
pub fn hex(hash: &Hash) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

/// A hash from its hexadecimal form.
pub fn parse_hex(text: &str) -> Option<Hash> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// How a file is stored on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobCompression {
    /// As received (PDF, images and archives are compressed already).
    None,
    /// gzip (HTML, XML, JSON, plain text).
    Gzip,
}

impl BlobCompression {
    /// The name stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gzip => "gzip",
        }
    }

    /// Compression worth applying to a content type.
    pub fn for_content_type(content_type: Option<&str>) -> Self {
        let essence = content_type
            .and_then(|c| c.split(';').next())
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let textual = essence.starts_with("text/")
            || essence.ends_with("+xml")
            || matches!(
                essence.as_str(),
                "application/xml" | "application/json" | "application/xhtml+xml"
            );
        if textual { Self::Gzip } else { Self::None }
    }
}

/// Result of storing a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredBlob {
    /// BLAKE3 hash of the original bytes.
    pub hash: Hash,
    /// Size of the original.
    pub size: u64,
    /// Size on disk.
    pub stored_size: u64,
    /// How it is stored.
    pub compression: BlobCompression,
    /// `false` when the file was already there.
    pub written: bool,
}

/// The file store under one root directory.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl BlobStore {
    /// A store under `root` (created when needed).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where a hash is stored.
    pub fn path(&self, hash: &Hash) -> PathBuf {
        let h = hex(hash);
        self.root.join(&h[..2]).join(&h[2..4]).join(h)
    }

    /// Stores `bytes` (blocking file I/O; call it from a blocking context).
    pub fn put(&self, bytes: &[u8], compression: BlobCompression) -> std::io::Result<StoredBlob> {
        let hash = hash(bytes);
        let path = self.path(&hash);
        if let Ok(meta) = std::fs::metadata(&path) {
            return Ok(StoredBlob {
                hash,
                size: bytes.len() as u64,
                stored_size: meta.len(),
                compression,
                written: false,
            });
        }
        let dir = path.parent().expect("blob paths have a parent");
        std::fs::create_dir_all(dir)?;
        let data = match compression {
            BlobCompression::None => bytes.to_vec(),
            BlobCompression::Gzip => {
                let mut gz =
                    GzEncoder::new(Vec::with_capacity(bytes.len() / 4), Compression::default());
                gz.write_all(bytes)?;
                gz.finish()?
            }
        };
        let temp = dir.join(format!(
            ".tmp-{}-{}-{}",
            hex(&hash),
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        {
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(&data)?;
            file.sync_all()?;
        }
        if let Err(e) = std::fs::rename(&temp, &path) {
            let _ = std::fs::remove_file(&temp);
            // another process stored the same content first
            if !path.exists() {
                return Err(e);
            }
        }
        Ok(StoredBlob {
            hash,
            size: bytes.len() as u64,
            stored_size: data.len() as u64,
            compression,
            written: true,
        })
    }

    /// Reads a stored file back; `None` when it is not there. The content is verified against the hash.
    pub fn get(
        &self,
        hash: &Hash,
        compression: BlobCompression,
    ) -> std::io::Result<Option<Vec<u8>>> {
        let stored = match std::fs::read(self.path(hash)) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let bytes = match compression {
            BlobCompression::None => stored,
            BlobCompression::Gzip => {
                let mut out = Vec::new();
                GzDecoder::new(&stored[..]).read_to_end(&mut out)?;
                out
            }
        };
        if self::hash(&bytes) != *hash {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("stored file {} does not match its hash", hex(hash)),
            ));
        }
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("atena-blob-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn stores_once_compresses_text_and_verifies() {
        let root = temp_root("roundtrip");
        let store = BlobStore::new(&root);
        let html = "<p>Zápis do 1. ročníka</p>".repeat(200);
        let a = store.put(html.as_bytes(), BlobCompression::Gzip).unwrap();
        assert!(a.written && a.stored_size < a.size);
        let b = store.put(html.as_bytes(), BlobCompression::Gzip).unwrap();
        assert!(!b.written);
        assert_eq!(
            store.get(&a.hash, BlobCompression::Gzip).unwrap().unwrap(),
            html.as_bytes()
        );
        assert_eq!(
            store.get(&hash(b"missing"), BlobCompression::None).unwrap(),
            None
        );

        std::fs::write(store.path(&a.hash), b"tampered").unwrap();
        assert!(store.get(&a.hash, BlobCompression::None).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn hashes_and_compression_choice() {
        let h = hash(b"atena");
        assert_eq!(parse_hex(&hex(&h)), Some(h));
        assert_eq!(parse_hex("zz"), None);
        assert_eq!(
            BlobCompression::for_content_type(Some("text/html; charset=utf-8")),
            BlobCompression::Gzip
        );
        assert_eq!(
            BlobCompression::for_content_type(Some("application/rss+xml")),
            BlobCompression::Gzip
        );
        assert_eq!(
            BlobCompression::for_content_type(Some("application/pdf")),
            BlobCompression::None
        );
        assert_eq!(
            BlobCompression::for_content_type(None),
            BlobCompression::None
        );
    }
}
