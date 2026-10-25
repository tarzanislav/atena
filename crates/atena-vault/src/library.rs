//! The private library: documents the user saved for themselves, encrypted with the data key of
//! the private account and kept only on this computer.
//!
//! * **References** – documents the public part keeps and shows in full anyway (official texts,
//!   open licences): only the facts and the fingerprint of the text are saved, never a second copy.
//! * **Copies for the user's own use** – everything else the user saved: the text and, for PDF and
//!   office documents, the file as the publisher published it (after the danger check of the
//!   reader). This is the private copy of a natural person (in Slovakia § 42 of the Copyright Act
//!   185/2015); copies are made only on the user's click and are never shared or sent.
//!
//! The catalogue (`catalogue.atena`) is encrypted as a whole, so even the titles of what the user
//! saved stay private; each text and file is encrypted on its own, bound to its item.

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{DataKey, Result, VaultError, hex, io, random, write_secret};

/// The catalogue of the library.
const CATALOGUE_FILE: &str = "catalogue.atena";
/// Binds the encrypted catalogue to its purpose and format.
const CATALOGUE_CONTEXT: &[u8] = b"atena-private-v1 library catalogue";
/// Format of the catalogue.
const CATALOGUE_FORMAT: u32 = 1;
/// Most items a library holds.
pub const MAX_ITEMS: usize = 20_000;

/// How an item is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// The public part keeps and shows the document; the library refers to it.
    Reference,
    /// A copy for the user's own use, encrypted in the library.
    Copy,
}

/// What is known about a saved document, given when it is saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemFacts {
    /// The document in the public part, while it is there.
    pub document_id: Option<i64>,
    /// Address of the original – with the fingerprint, the identity of the document.
    pub url: String,
    /// Title.
    pub title: String,
    /// Source ID of the register.
    pub source_id: String,
    /// Source name.
    pub source_name: String,
    /// Licence of the source, as in the register.
    pub licence: String,
    /// Authority of the source (A1 … E).
    pub authority: String,
    /// Language code.
    pub language: Option<String>,
    /// Taxonomy labels `scheme/concept`.
    pub labels: Vec<String>,
    /// BLAKE3 of the text (hexadecimal).
    pub fingerprint: Option<String>,
    /// Media type of the kept file.
    pub media_type: Option<String>,
}

/// A saved document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Random identifier in the library.
    pub id: String,
    /// What is known about it.
    #[serde(flatten)]
    pub facts: ItemFacts,
    /// How it is kept.
    pub kind: Kind,
    /// Characters of the kept text (copies).
    pub text_chars: u64,
    /// Bytes of the kept file (copies of PDF and office documents).
    pub file_bytes: Option<u64>,
    /// When it was saved.
    pub saved_at: Timestamp,
}

/// The result of saving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Added {
    /// Saved now.
    New(Item),
    /// The library holds it already (same address or same public document).
    Already(Item),
}

#[derive(Serialize, Deserialize)]
struct Catalogue {
    format: u32,
    items: Vec<Item>,
}

/// The library in its folder (`private/library`).
#[derive(Debug, Clone)]
pub struct Library {
    folder: PathBuf,
}

fn damaged(what: &str) -> VaultError {
    VaultError::Damaged(format!("the library {what} cannot be read with this key"))
}

/// Item identifiers are 32 hexadecimal characters; nothing else ever names a file.
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Library {
    /// The library in `folder` (created when the first item is saved).
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
        }
    }

    fn catalogue_path(&self) -> PathBuf {
        self.folder.join(CATALOGUE_FILE)
    }

    fn payload(&self, id: &str, part: &str) -> PathBuf {
        self.folder.join(format!("{id}.{part}"))
    }

    fn context(id: &str, part: &str) -> Vec<u8> {
        format!("atena-private-v1 item {id} {part}").into_bytes()
    }

    fn read(&self, key: &DataKey) -> Result<Vec<Item>> {
        let path = self.catalogue_path();
        let sealed = match fs::read(&path) {
            Ok(sealed) => sealed,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io(&path)(e)),
        };
        let plain = key
            .decrypt(CATALOGUE_CONTEXT, &sealed)
            .ok_or_else(|| damaged("catalogue"))?;
        let catalogue: Catalogue =
            serde_json::from_slice(&plain).map_err(|e| VaultError::Damaged(e.to_string()))?;
        if catalogue.format != CATALOGUE_FORMAT {
            return Err(VaultError::Damaged(format!(
                "library format {}",
                catalogue.format
            )));
        }
        Ok(catalogue.items)
    }

    fn write(&self, key: &DataKey, items: Vec<Item>) -> Result<()> {
        let catalogue = Catalogue {
            format: CATALOGUE_FORMAT,
            items,
        };
        let plain = Zeroizing::new(
            serde_json::to_vec(&catalogue).map_err(|e| VaultError::Damaged(e.to_string()))?,
        );
        let sealed = key.encrypt(CATALOGUE_CONTEXT, &plain)?;
        write_secret(&self.folder, &self.catalogue_path(), &sealed)
    }

    fn write_part(&self, key: &DataKey, id: &str, part: &str, plain: &[u8]) -> Result<()> {
        let sealed = key.encrypt(&Self::context(id, part), plain)?;
        write_secret(&self.folder, &self.payload(id, part), &sealed)
    }

    fn read_part(&self, key: &DataKey, id: &str, part: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        if !valid_id(id) {
            return Ok(None);
        }
        let path = self.payload(id, part);
        let sealed = match fs::read(&path) {
            Ok(sealed) => sealed,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(&path)(e)),
        };
        key.decrypt(&Self::context(id, part), &sealed)
            .map(Some)
            .ok_or_else(|| damaged(part))
    }

    /// The saved documents, newest first.
    pub fn items(&self, key: &DataKey) -> Result<Vec<Item>> {
        let mut items = self.read(key)?;
        items.sort_by_key(|item| std::cmp::Reverse(item.saved_at));
        Ok(items)
    }

    /// The item already saved for this address or public document.
    pub fn find(&self, key: &DataKey, url: &str, document_id: Option<i64>) -> Result<Option<Item>> {
        Ok(self.read(key)?.into_iter().find(|item| {
            item.facts.url == url
                || (document_id.is_some() && item.facts.document_id == document_id)
        }))
    }

    /// Saves a document: a reference (no text, no file) or a copy with its text and, optionally,
    /// its file. The parts are written before the catalogue, so a crash never lists a missing part.
    pub fn add(
        &self,
        key: &DataKey,
        facts: ItemFacts,
        kind: Kind,
        text: Option<&str>,
        file: Option<&[u8]>,
    ) -> Result<Added> {
        let mut items = self.read(key)?;
        if let Some(found) = items.iter().find(|item| {
            item.facts.url == facts.url
                || (facts.document_id.is_some() && item.facts.document_id == facts.document_id)
        }) {
            return Ok(Added::Already(found.clone()));
        }
        if items.len() >= MAX_ITEMS {
            return Err(VaultError::LibraryFull);
        }
        let id = hex(&*random::<16>()?);
        let (text, file) = match kind {
            Kind::Reference => (None, None),
            Kind::Copy => (text, file),
        };
        if let Some(text) = text {
            self.write_part(key, &id, "text", text.as_bytes())?;
        }
        if let Some(file) = file {
            self.write_part(key, &id, "file", file)?;
        }
        let item = Item {
            id,
            facts,
            kind,
            text_chars: text.map_or(0, |t| t.chars().count() as u64),
            file_bytes: file.map(|f| f.len() as u64),
            saved_at: Timestamp::now(),
        };
        items.push(item.clone());
        if let Err(e) = self.write(key, items) {
            self.delete_parts(&item.id);
            return Err(e);
        }
        Ok(Added::New(item))
    }

    /// The kept text of a copy.
    pub fn text(&self, key: &DataKey, id: &str) -> Result<Option<Zeroizing<String>>> {
        let Some(bytes) = self.read_part(key, id, "text")? else {
            return Ok(None);
        };
        String::from_utf8(bytes.to_vec())
            .map(|text| Some(Zeroizing::new(text)))
            .map_err(|_| damaged("text"))
    }

    /// The kept file of a copy.
    pub fn file(&self, key: &DataKey, id: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        self.read_part(key, id, "file")
    }

    fn delete_parts(&self, id: &str) {
        if valid_id(id) {
            for part in ["text", "file"] {
                let _ = fs::remove_file(self.payload(id, part));
            }
        }
    }

    /// Removes items and their parts; returns how many were removed.
    pub fn remove(&self, key: &DataKey, ids: &[String]) -> Result<usize> {
        let items = self.read(key)?;
        let (gone, kept): (Vec<Item>, Vec<Item>) =
            items.into_iter().partition(|item| ids.contains(&item.id));
        if gone.is_empty() {
            return Ok(0);
        }
        self.write(key, kept)?;
        for item in &gone {
            self.delete_parts(&item.id);
        }
        Ok(gone.len())
    }

    /// The folder of the library.
    pub fn folder(&self) -> &Path {
        &self.folder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str) -> Library {
        let folder =
            std::env::temp_dir().join(format!("atena-library-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        Library::new(folder)
    }

    fn key(byte: u8) -> DataKey {
        DataKey(Zeroizing::new([byte; 32]))
    }

    fn facts(url: &str, document_id: Option<i64>) -> ItemFacts {
        ItemFacts {
            document_id,
            url: url.into(),
            title: "Metodika hodnotenia".into(),
            source_id: "SK07".into(),
            source_name: "NIVaM".into(),
            licence: "copyrighted".into(),
            authority: "A2".into(),
            language: Some("sk".into()),
            labels: vec!["section/practice".into()],
            fingerprint: Some("ab".repeat(32)),
            media_type: Some("application/pdf".into()),
        }
    }

    #[test]
    fn copies_and_references_are_kept_encrypted() {
        let lib = library("keep");
        let k = key(1);
        assert!(lib.items(&k).unwrap().is_empty());
        let Added::New(copy) = lib
            .add(
                &k,
                facts("https://a.example.sk/m.pdf", Some(7)),
                Kind::Copy,
                Some("Hodnotenie žiakov – text"),
                Some(b"%PDF-1.7 file"),
            )
            .unwrap()
        else {
            panic!("saved");
        };
        assert_eq!(copy.text_chars, 24);
        assert_eq!(copy.file_bytes, Some(13));
        let Added::New(reference) = lib
            .add(
                &k,
                facts("https://b.example.sk/zakon", Some(8)),
                Kind::Reference,
                Some("ignored"),
                Some(b"ignored"),
            )
            .unwrap()
        else {
            panic!("saved");
        };
        assert_eq!((reference.text_chars, reference.file_bytes), (0, None));
        assert!(lib.text(&k, &reference.id).unwrap().is_none());

        // the same address or public document is not saved twice
        assert!(matches!(
            lib.add(&k, facts("https://a.example.sk/m.pdf", None), Kind::Copy, Some("x"), None)
                .unwrap(),
            Added::Already(item) if item.id == copy.id
        ));
        assert!(matches!(
            lib.add(&k, facts("https://c.example.sk/", Some(8)), Kind::Copy, Some("x"), None)
                .unwrap(),
            Added::Already(item) if item.id == reference.id
        ));
        assert_eq!(
            lib.find(&k, "https://x.example.sk/", Some(7))
                .unwrap()
                .unwrap()
                .id,
            copy.id
        );

        // read back; nothing readable on disk, not even the titles
        assert_eq!(
            lib.text(&k, &copy.id).unwrap().unwrap().as_str(),
            "Hodnotenie žiakov – text"
        );
        assert_eq!(
            lib.file(&k, &copy.id).unwrap().unwrap().as_slice(),
            b"%PDF-1.7 file"
        );
        for entry in fs::read_dir(lib.folder()).unwrap() {
            let bytes = fs::read(entry.unwrap().path()).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(
                !text.contains("Metodika") && !text.contains("Hodnotenie") && !text.contains("PDF")
            );
        }
        assert_eq!(lib.items(&k).unwrap().len(), 2);

        // another key opens nothing; a part moved to another item opens nothing
        assert!(lib.items(&key(2)).is_err());
        assert!(lib.text(&key(2), &copy.id).is_err());
        fs::copy(
            lib.payload(&copy.id, "text"),
            lib.payload(&reference.id, "text"),
        )
        .unwrap();
        assert!(lib.text(&k, &reference.id).is_err());
        assert!(lib.text(&k, "../account").unwrap().is_none());

        // removal deletes the parts
        assert_eq!(lib.remove(&k, &[copy.id.clone(), "x".into()]).unwrap(), 1);
        assert!(!lib.payload(&copy.id, "text").exists() && !lib.payload(&copy.id, "file").exists());
        assert_eq!(lib.items(&k).unwrap().len(), 1);
        assert_eq!(lib.remove(&k, &[copy.id]).unwrap(), 0);
        fs::remove_dir_all(lib.folder()).unwrap();
    }
}
