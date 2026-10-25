//! ATENA's private part: one local account per installation whose key encrypts the user's private
//! data (pillar 2 – privacy).
//!
//! * Nothing leaves the computer: no server, no e-mail, no recovery through anyone else.
//! * The account file holds no password and no key in the clear. A random 256-bit **data key**
//!   encrypts the private data; it is stored twice, wrapped (XChaCha20-Poly1305):
//!   - by a key derived from the **password** (Argon2id, 64 MiB, 3 passes),
//!   - by a key derived from the **recovery key**, a random 160-bit code shown once when the
//!     account is made (`ATENA-XXXX-…`), which sets a new password when the old one is forgotten.
//! * A wrong name or password gives one and the same error; the key derivation makes guessing slow.
//! * Keys are wiped from memory when dropped (`zeroize`).
//! * On request – at the user's own risk – a persistent sign-in opens the account on this
//!   computer without the password (see [`remember`]).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

pub mod remember;
pub use remember::{Keychain, REMEMBER_FILE};

/// Name of the account file in the private folder.
pub const ACCOUNT_FILE: &str = "account.json";
/// Shortest password, in characters.
pub const MIN_PASSWORD_CHARS: usize = 10;
/// Longest account name, in characters.
pub const MAX_NAME_CHARS: usize = 40;
/// Format of the account file.
const FORMAT: u32 = 1;
/// Associated data of every wrapped data key: binds it to this purpose and format.
const WRAP_AAD: &[u8] = b"atena-private-v1 data key";
/// Context of the key derived from a recovery key.
const RECOVERY_CONTEXT: &str = "ATENA 2026-10-25 private recovery key v1";
/// Bytes of a recovery key (160 bits).
const RECOVERY_BYTES: usize = 20;
/// Crockford's base32 alphabet (no I, L, O, U).
const BASE32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Prefix of a written recovery key.
const RECOVERY_PREFIX: &str = "ATENA";

/// What went wrong; the interface translates the codes.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// There is no private account yet.
    #[error("no private account")]
    NoAccount,
    /// An account exists already (one per installation).
    #[error("a private account exists already")]
    Exists,
    /// The name or the password is wrong (never told apart).
    #[error("wrong name or password")]
    WrongLogin,
    /// The recovery key is not the one of this account.
    #[error("wrong recovery key")]
    WrongRecoveryKey,
    /// The password is too short or equals the name.
    #[error(
        "the password needs at least {MIN_PASSWORD_CHARS} characters and must differ from the name"
    )]
    WeakPassword,
    /// The name is empty or too long.
    #[error("the name must have 1 to {MAX_NAME_CHARS} characters")]
    BadName,
    /// The account file is damaged or of an unknown format.
    #[error("the account file is damaged: {0}")]
    Damaged(String),
    /// The file system failed.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The error.
        source: std::io::Error,
    },
    /// The system has no random numbers.
    #[error("no random numbers: {0}")]
    Random(String),
    /// The system has no credential store for a persistent sign-in.
    #[error("this system has no credential store for a persistent sign-in")]
    NoKeychain,
    /// The credential store failed (e.g. it is locked).
    #[error("the credential store failed: {0}")]
    Keychain(String),
}

type Result<T> = std::result::Result<T, VaultError>;

/// The key that encrypts the private data; wiped when dropped.
pub struct DataKey(Zeroizing<[u8; 32]>);

impl fmt::Debug for DataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DataKey(…)")
    }
}

impl DataKey {
    /// Encrypts `plaintext`; `context` (e.g. the record's identity) must be given again to decrypt.
    /// The result is the 24-byte nonce followed by the ciphertext and tag.
    pub fn encrypt(&self, context: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        seal(&self.0, context, plaintext)
    }

    /// Decrypts what [`DataKey::encrypt`] made with the same `context`; `None` when it was
    /// altered, made with another key or for another context.
    pub fn decrypt(&self, context: &[u8], sealed: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
        open(&self.0, context, sealed)
    }

    /// A key derived for one purpose (e.g. the database password), never the data key itself.
    pub fn derive(&self, purpose: &str) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(blake3::derive_key(purpose, &*self.0))
    }
}

/// A recovery key as shown to the user: `ATENA-XXXX-XXXX-…` (32 characters of base32).
pub struct RecoveryKey(Zeroizing<String>);

impl RecoveryKey {
    /// The text to show, print or store.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryKey(…)")
    }
}

/// The public facts of the account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountInfo {
    /// The account name.
    pub name: String,
    /// When it was made.
    pub created: Timestamp,
}

/// Cost of the password key derivation (stored in the file, so it can grow later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    /// Memory in KiB.
    pub memory_kib: u32,
    /// Passes.
    pub passes: u32,
    /// Lanes.
    pub lanes: u32,
}

impl Cost {
    /// The cost of new accounts: 64 MiB, 3 passes, 1 lane.
    pub const DEFAULT: Self = Self {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    /// Cheap enough for tests (never for real accounts).
    pub const TEST: Self = Self {
        memory_kib: 64,
        passes: 1,
        lanes: 1,
    };
}

#[derive(Serialize, Deserialize)]
struct Wrapped {
    salt: String,
    sealed: String,
}

#[derive(Serialize, Deserialize)]
struct AccountFile {
    format: u32,
    name: String,
    created: Timestamp,
    cost: Cost,
    by_password: Wrapped,
    by_recovery: Wrapped,
}

/// The private account of one installation, kept in `account.json` of its folder.
#[derive(Debug, Clone)]
pub struct Vault {
    folder: PathBuf,
    cost: Cost,
    /// Where a persistent sign-in keeps its remember key; `None`: no persistent sign-in.
    keychain: Option<Keychain>,
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> VaultError + '_ {
    move |source| VaultError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn random<const N: usize>() -> Result<Zeroizing<[u8; N]>> {
    let mut out = Zeroizing::new([0u8; N]);
    getrandom::fill(&mut *out).map_err(|e| VaultError::Random(e.to_string()))?;
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>> {
    let bad = || VaultError::Damaged("not hexadecimal".into());
    if !text.len().is_multiple_of(2) {
        return Err(bad());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2).ok_or_else(bad)?, 16).map_err(|_| bad()))
        .collect()
}

fn seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(&(*key).into());
    let nonce: Zeroizing<[u8; 24]> = random()?;
    let nonce_array = XNonce::from(*nonce);
    let sealed = cipher
        .encrypt(
            &nonce_array,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| VaultError::Damaged("encryption failed".into()))?;
    let mut out = Vec::with_capacity(24 + sealed.len());
    out.extend_from_slice(&*nonce);
    out.extend_from_slice(&sealed);
    Ok(out)
}

fn open(key: &[u8; 32], aad: &[u8], sealed: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if sealed.len() < 24 + 16 {
        return None;
    }
    let (nonce, body) = sealed.split_at(24);
    let nonce: [u8; 24] = nonce.try_into().ok()?;
    let cipher = XChaCha20Poly1305::new(&(*key).into());
    cipher
        .decrypt(&XNonce::from(nonce), Payload { msg: body, aad })
        .ok()
        .map(Zeroizing::new)
}

fn password_key(password: &str, salt: &[u8], cost: Cost) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32))
        .map_err(|e| VaultError::Damaged(e.to_string()))?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut *out)
        .map_err(|e| VaultError::Damaged(e.to_string()))?;
    Ok(out)
}

fn recovery_key(bytes: &[u8], salt: &[u8]) -> Zeroizing<[u8; 32]> {
    let mut material = Zeroizing::new(Vec::with_capacity(bytes.len() + salt.len()));
    material.extend_from_slice(bytes);
    material.extend_from_slice(salt);
    Zeroizing::new(blake3::derive_key(RECOVERY_CONTEXT, &material))
}

/// Writes the 160 bits as 32 base32 characters in groups of four, after `ATENA-`.
fn write_recovery(bytes: &[u8; RECOVERY_BYTES]) -> RecoveryKey {
    let mut bits: u64 = 0;
    let mut have = 0;
    let mut chars = Zeroizing::new(String::new());
    for &byte in bytes {
        bits = (bits << 8) | u64::from(byte);
        have += 8;
        while have >= 5 {
            have -= 5;
            chars.push(BASE32[((bits >> have) & 31) as usize] as char);
        }
    }
    bits.zeroize();
    let mut text = Zeroizing::new(String::from(RECOVERY_PREFIX));
    for (i, c) in chars.chars().enumerate() {
        if i % 4 == 0 {
            text.push('-');
        }
        text.push(c);
    }
    RecoveryKey(text)
}

/// Reads a recovery key as typed: any case, with or without the prefix, dashes and spaces;
/// `I`/`L` read as 1 and `O` as 0.
fn read_recovery(text: &str) -> Option<Zeroizing<[u8; RECOVERY_BYTES]>> {
    let upper = Zeroizing::new(text.trim().to_ascii_uppercase());
    let body = upper.strip_prefix(RECOVERY_PREFIX).unwrap_or(&upper);
    let mut bits: u64 = 0;
    let mut have = 0;
    let mut out = Zeroizing::new([0u8; RECOVERY_BYTES]);
    let mut n = 0;
    for c in body.chars().filter(|c| !matches!(c, '-' | ' ')) {
        let c = match c {
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        };
        let value = BASE32.iter().position(|&b| b as char == c)? as u64;
        bits = (bits << 5) | value;
        have += 5;
        if have >= 8 {
            have -= 8;
            *out.get_mut(n)? = ((bits >> have) & 0xff) as u8;
            n += 1;
        }
    }
    bits.zeroize();
    (n == RECOVERY_BYTES && have == 0).then_some(out)
}

fn check_name(name: &str) -> Result<String> {
    let name: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars = name.chars().count();
    if chars == 0 || chars > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(VaultError::BadName);
    }
    Ok(name)
}

fn check_password(password: &str, name: &str) -> Result<()> {
    if password.chars().count() < MIN_PASSWORD_CHARS
        || password.trim().eq_ignore_ascii_case(name.trim())
    {
        return Err(VaultError::WeakPassword);
    }
    Ok(())
}

impl Vault {
    /// The vault in `folder` (created when an account is made).
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
            cost: Cost::DEFAULT,
            keychain: None,
        }
    }

    /// The same vault with another key derivation cost for new passwords (tests).
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = cost;
        self
    }

    fn path(&self) -> PathBuf {
        self.folder.join(ACCOUNT_FILE)
    }

    fn read(&self) -> Result<AccountFile> {
        let path = self.path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(VaultError::NoAccount);
            }
            Err(e) => return Err(io(&path)(e)),
        };
        let file: AccountFile =
            serde_json::from_str(&text).map_err(|e| VaultError::Damaged(e.to_string()))?;
        if file.format != FORMAT {
            return Err(VaultError::Damaged(format!("format {}", file.format)));
        }
        Ok(file)
    }

    /// Writes through a temporary file, so a crash leaves the old account intact.
    fn write(&self, file: &AccountFile) -> Result<()> {
        let text =
            serde_json::to_string_pretty(file).map_err(|e| VaultError::Damaged(e.to_string()))?;
        self.write_private(&self.path(), &text)
    }

    /// Writes a file of the folder through a temporary file, readable by the user only.
    fn write_private(&self, path: &Path, text: &str) -> Result<()> {
        fs::create_dir_all(&self.folder).map_err(io(&self.folder))?;
        let temporary = path.with_extension("json.new");
        fs::write(&temporary, text).map_err(io(&temporary))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
                .map_err(io(&temporary))?;
        }
        fs::rename(&temporary, path).map_err(io(path))
    }

    fn wrap_password(&self, key: &[u8; 32], password: &str) -> Result<Wrapped> {
        let salt: Zeroizing<[u8; 16]> = random()?;
        let kek = password_key(password, &*salt, self.cost)?;
        Ok(Wrapped {
            salt: hex(&*salt),
            sealed: hex(&seal(&kek, WRAP_AAD, key)?),
        })
    }

    fn unwrap(kek: &[u8; 32], wrapped: &Wrapped) -> Result<Option<DataKey>> {
        let sealed = unhex(&wrapped.sealed)?;
        Ok(open(kek, WRAP_AAD, &sealed).and_then(|plain| {
            let key: [u8; 32] = plain.as_slice().try_into().ok()?;
            Some(DataKey(Zeroizing::new(key)))
        }))
    }

    /// The account, if there is one.
    pub fn account(&self) -> Result<Option<AccountInfo>> {
        match self.read() {
            Ok(file) => Ok(Some(AccountInfo {
                name: file.name,
                created: file.created,
            })),
            Err(VaultError::NoAccount) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Makes the account: returns its data key and the recovery key to show once.
    pub fn create(&self, name: &str, password: &str) -> Result<(DataKey, RecoveryKey)> {
        if self.account()?.is_some() {
            return Err(VaultError::Exists);
        }
        let name = check_name(name)?;
        check_password(password, &name)?;
        // a persistent sign-in left from an earlier account never opens this one
        self.forget()?;
        let key: Zeroizing<[u8; 32]> = random()?;
        let recovery: Zeroizing<[u8; RECOVERY_BYTES]> = random()?;
        let recovery_salt: Zeroizing<[u8; 16]> = random()?;
        let by_recovery = Wrapped {
            salt: hex(&*recovery_salt),
            sealed: hex(&seal(
                &recovery_key(&*recovery, &*recovery_salt),
                WRAP_AAD,
                &*key,
            )?),
        };
        let file = AccountFile {
            format: FORMAT,
            name,
            created: Timestamp::now(),
            cost: self.cost,
            by_password: self.wrap_password(&key, password)?,
            by_recovery,
        };
        self.write(&file)?;
        Ok((DataKey(key), write_recovery(&recovery)))
    }

    /// Opens the account with its name and password.
    pub fn unlock(&self, name: &str, password: &str) -> Result<DataKey> {
        let file = self.read()?;
        let salt = unhex(&file.by_password.salt)?;
        // the key is derived even for a wrong name, so the answer takes as long either way
        let kek = password_key(password, &salt, file.cost)?;
        let same_name =
            check_name(name).is_ok_and(|n| n.to_lowercase() == file.name.to_lowercase());
        match Self::unwrap(&kek, &file.by_password)? {
            Some(key) if same_name => Ok(key),
            _ => Err(VaultError::WrongLogin),
        }
    }

    /// Sets a new password with the recovery key; the recovery key stays valid.
    pub fn recover(&self, recovery: &str, new_password: &str) -> Result<DataKey> {
        let mut file = self.read()?;
        check_password(new_password, &file.name)?;
        let bytes = read_recovery(recovery).ok_or(VaultError::WrongRecoveryKey)?;
        let salt = unhex(&file.by_recovery.salt)?;
        let key = Self::unwrap(&recovery_key(&*bytes, &salt), &file.by_recovery)?
            .ok_or(VaultError::WrongRecoveryKey)?;
        file.cost = self.cost;
        file.by_password = self.wrap_password(&key.0, new_password)?;
        self.write(&file)?;
        Ok(key)
    }

    /// Changes the password; the old one must be right.
    pub fn change_password(&self, name: &str, old: &str, new: &str) -> Result<()> {
        let key = self.unlock(name, old)?;
        let mut file = self.read()?;
        check_password(new, &file.name)?;
        file.cost = self.cost;
        file.by_password = self.wrap_password(&key.0, new)?;
        self.write(&file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(name: &str) -> Vault {
        let folder =
            std::env::temp_dir().join(format!("atena-vault-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        Vault::new(folder).with_cost(Cost::TEST)
    }

    #[test]
    fn an_account_is_made_once_and_opened_with_its_password() {
        let v = vault("make");
        assert!(v.account().unwrap().is_none());
        assert!(matches!(
            v.unlock("Ján", "dlhéheslo123"),
            Err(VaultError::NoAccount)
        ));
        let (key, recovery) = v.create("  Ján   Hraško ", "dlhé heslo 123").unwrap();
        assert_eq!(v.account().unwrap().unwrap().name, "Ján Hraško");
        assert!(matches!(
            v.create("Iný", "dlhé heslo 456"),
            Err(VaultError::Exists)
        ));

        let sealed = key.encrypt(b"doc:1", "súkromná kópia".as_bytes()).unwrap();
        let again = v.unlock("ján hraško", "dlhé heslo 123").unwrap();
        assert_eq!(
            again.decrypt(b"doc:1", &sealed).unwrap().as_slice(),
            "súkromná kópia".as_bytes()
        );
        // another context, altered data or a wrong login open nothing
        assert!(again.decrypt(b"doc:2", &sealed).is_none());
        let mut altered = sealed.clone();
        altered[30] ^= 1;
        assert!(again.decrypt(b"doc:1", &altered).is_none());
        assert!(matches!(
            v.unlock("Ján Hraško", "zlé heslo 123"),
            Err(VaultError::WrongLogin)
        ));
        assert!(matches!(
            v.unlock("Iné meno", "dlhé heslo 123"),
            Err(VaultError::WrongLogin)
        ));

        // nothing secret in the file
        let text = fs::read_to_string(v.path()).unwrap();
        assert!(!text.contains("dlhé heslo") && !text.contains(&recovery.as_str()[6..]));
        assert_eq!(v.cost, Cost::TEST);
        fs::remove_dir_all(&v.folder).unwrap();
    }

    #[test]
    fn the_recovery_key_sets_a_new_password() {
        let v = vault("recover");
        let (key, recovery) = v.create("Mária", "prvé heslo 2026").unwrap();
        let sealed = key.encrypt(b"x", b"data").unwrap();
        let written = recovery.as_str().to_string();
        assert!(written.starts_with("ATENA-"));
        assert_eq!(written.len(), "ATENA".len() + 8 * 5);
        // typed loosely: lower case, without dashes, O for 0
        let typed = written.to_lowercase().replace('-', " ").replace('0', "o");
        assert!(matches!(
            v.recover(
                "ATENA-AAAA-BBBB-CCCC-DDDD-EEEE-FFFF-GGGG-HHHH",
                "nové heslo 2026"
            ),
            Err(VaultError::WrongRecoveryKey)
        ));
        assert!(matches!(
            v.recover("nezmysel", "nové heslo 2026"),
            Err(VaultError::WrongRecoveryKey)
        ));
        let back = v.recover(&typed, "nové heslo 2026").unwrap();
        assert_eq!(back.decrypt(b"x", &sealed).unwrap().as_slice(), b"data");
        assert!(v.unlock("Mária", "prvé heslo 2026").is_err());
        assert!(v.unlock("Mária", "nové heslo 2026").is_ok());
        fs::remove_dir_all(&v.folder).unwrap();
    }

    #[test]
    fn passwords_and_names_are_checked() {
        let v = vault("check");
        assert!(matches!(
            v.create("Ján", "krátke"),
            Err(VaultError::WeakPassword)
        ));
        assert!(matches!(
            v.create("dlhé meno ok", "Dlhé Meno OK"),
            Err(VaultError::WeakPassword)
        ));
        assert!(matches!(
            v.create("   ", "dostatočne dlhé"),
            Err(VaultError::BadName)
        ));
        assert!(matches!(
            v.create(&"x".repeat(41), "dostatočne dlhé"),
            Err(VaultError::BadName)
        ));
        v.create("Ján", "dostatočne dlhé").unwrap();
        assert!(matches!(
            v.change_password("Ján", "zlé heslo 12345", "nové heslo 12345"),
            Err(VaultError::WrongLogin)
        ));
        v.change_password("Ján", "dostatočne dlhé", "nové heslo 12345")
            .unwrap();
        assert!(v.unlock("Ján", "nové heslo 12345").is_ok());
        fs::remove_dir_all(&v.folder).unwrap();
    }

    #[test]
    fn recovery_keys_round_trip_and_derived_keys_differ() {
        for seed in [
            [0u8; RECOVERY_BYTES],
            [0xff; RECOVERY_BYTES],
            [0x5a; RECOVERY_BYTES],
        ] {
            let written = write_recovery(&seed);
            assert_eq!(*read_recovery(written.as_str()).unwrap(), seed);
        }
        assert!(read_recovery("ATENA-0000").is_none());
        let key = DataKey(Zeroizing::new([7u8; 32]));
        assert_ne!(*key.derive("database"), *key.derive("files"));
        assert_ne!(*key.derive("database"), [7u8; 32]);
        assert_eq!(format!("{key:?}"), "DataKey(…)");
    }

    // the cost of new accounts never drops below the documented strength
    const _: () = assert!(Cost::DEFAULT.memory_kib >= 64 * 1024 && Cost::DEFAULT.passes >= 3);
}
