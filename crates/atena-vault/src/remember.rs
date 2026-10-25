//! The persistent sign-in: when the user turns it on – knowingly and at their own risk – the
//! private part opens on this computer without the password.
//!
//! Whoever can sign in to the user's Windows account then opens the private part too. It is kept
//! in two halves, so neither alone opens anything:
//! * a random **remember key** in the system's credential store (the Windows Credential Manager,
//!   protected by the user's Windows sign-in and kept on this computer only – never in a roaming
//!   profile);
//! * the data key, encrypted with the remember key, in `remember.json` next to the account and
//!   bound to it (its name and creation time).
//!
//! Copying the folder to another computer, or reading the credential store without the folder,
//! gives nothing; turning the persistent sign-in off deletes both halves.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::sync::Arc;

use keyring_core::{CredentialStore, Entry};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{AccountFile, DataKey, Result, Vault, VaultError, hex, io, open, random, seal, unhex};

/// Name of the file of the persistent sign-in in the private folder.
pub const REMEMBER_FILE: &str = "remember.json";
/// Service of the remember key in the credential store.
const SERVICE: &str = "ATENA private part";
/// Format of the file.
const REMEMBER_FORMAT: u32 = 1;

/// The system's credential store, where the persistent sign-in keeps its remember key.
#[derive(Clone)]
pub struct Keychain {
    store: Arc<CredentialStore>,
    /// The Windows Credential Manager: keep the credential on this computer, under a clear name.
    windows: bool,
}

impl fmt::Debug for Keychain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.windows {
            "Keychain(Windows Credential Manager)"
        } else {
            "Keychain(memory)"
        })
    }
}

impl Keychain {
    /// The Windows Credential Manager; `None` when it cannot be reached.
    #[cfg(windows)]
    pub fn system() -> Option<Self> {
        let store: Arc<CredentialStore> = windows_native_keyring_store::Store::new().ok()?;
        Some(Self {
            store,
            windows: true,
        })
    }

    /// `None`: ATENA keeps a persistent sign-in only in the Windows Credential Manager.
    #[cfg(not(windows))]
    pub fn system() -> Option<Self> {
        None
    }

    /// A credential store in memory, gone with its last clone (for tests).
    pub fn memory() -> Option<Self> {
        let store: Arc<CredentialStore> = keyring_core::mock::Store::new().ok()?;
        Some(Self {
            store,
            windows: false,
        })
    }

    fn entry(&self, id: &str) -> keyring_core::Result<Entry> {
        if self.windows {
            // "Local": only on this computer, never copied with a roaming profile
            let target = format!("{SERVICE} {id}");
            let modifiers = HashMap::from([("target", target.as_str()), ("persistence", "Local")]);
            self.store.build(SERVICE, id, Some(&modifiers))
        } else {
            self.store.build(SERVICE, id, None)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RememberFile {
    format: u32,
    /// Which credential of the store holds the remember key.
    id: String,
    /// The data key encrypted with the remember key.
    sealed: String,
}

fn keychain_error(error: keyring_core::Error) -> VaultError {
    VaultError::Keychain(error.to_string())
}

/// Binds the encrypted data key to this account.
fn bound_to(account: &AccountFile) -> Vec<u8> {
    format!(
        "atena-private-v1 remember {} {}",
        account.name, account.created
    )
    .into_bytes()
}

impl Vault {
    /// The same vault with a credential store for the persistent sign-in (`None`: not offered).
    pub fn with_keychain(mut self, keychain: Option<Keychain>) -> Self {
        self.keychain = keychain;
        self
    }

    fn remember_path(&self) -> std::path::PathBuf {
        self.folder.join(REMEMBER_FILE)
    }

    /// A persistent sign-in can be turned on here (the system has a credential store).
    pub fn can_remember(&self) -> bool {
        self.keychain.is_some()
    }

    /// The persistent sign-in is on.
    pub fn is_remembered(&self) -> bool {
        self.can_remember() && self.remember_path().is_file()
    }

    /// Turns the persistent sign-in on for the open account, whose data key is `key`.
    pub fn remember(&self, key: &DataKey) -> Result<()> {
        let keychain = self.keychain.as_ref().ok_or(VaultError::NoKeychain)?;
        let account = self.read()?;
        self.forget()?;
        let id = hex(&*random::<16>()?);
        let wrap: Zeroizing<[u8; 32]> = random()?;
        let sealed = seal(&wrap, &bound_to(&account), &*key.0)?;
        keychain
            .entry(&id)
            .and_then(|entry| entry.set_secret(&*wrap))
            .map_err(keychain_error)?;
        let file = RememberFile {
            format: REMEMBER_FORMAT,
            id: id.clone(),
            sealed: hex(&sealed),
        };
        let text =
            serde_json::to_string_pretty(&file).map_err(|e| VaultError::Damaged(e.to_string()))?;
        if let Err(e) = self.write_private(&self.remember_path(), &text) {
            let _ = keychain
                .entry(&id)
                .and_then(|entry| entry.delete_credential());
            return Err(e);
        }
        Ok(())
    }

    /// The data key of the persistent sign-in, when it is on and still fits this account. A
    /// sign-in that no longer fits (another account, a folder copied from elsewhere, a deleted
    /// credential) is turned off; a credential store that cannot be read now is an error and the
    /// sign-in stays for the next time.
    pub fn remembered(&self) -> Result<Option<DataKey>> {
        let Some(keychain) = &self.keychain else {
            return Ok(None);
        };
        let path = self.remember_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(&path)(e)),
        };
        let account = match self.read() {
            Ok(account) => account,
            Err(VaultError::NoAccount) => return self.forget().map(|()| None),
            Err(e) => return Err(e),
        };
        let file = match serde_json::from_str::<RememberFile>(&text) {
            Ok(file) if file.format == REMEMBER_FORMAT => file,
            _ => return self.forget().map(|()| None),
        };
        let secret = match keychain
            .entry(&file.id)
            .and_then(|entry| entry.get_secret())
        {
            Ok(secret) => Zeroizing::new(secret),
            Err(keyring_core::Error::NoEntry) => return self.forget().map(|()| None),
            Err(e) => return Err(keychain_error(e)),
        };
        let key = (secret.len() == 32)
            .then(|| {
                let mut wrap = Zeroizing::new([0u8; 32]);
                wrap.copy_from_slice(&secret);
                wrap
            })
            .zip(unhex(&file.sealed).ok())
            .and_then(|(wrap, sealed)| open(&wrap, &bound_to(&account), &sealed))
            .and_then(|plain| {
                let key: [u8; 32] = plain.as_slice().try_into().ok()?;
                Some(DataKey(Zeroizing::new(key)))
            });
        match key {
            Some(key) => Ok(Some(key)),
            None => self.forget().map(|()| None),
        }
    }

    /// Turns the persistent sign-in off: deletes the file and the remember key.
    pub fn forget(&self) -> Result<()> {
        let path = self.remember_path();
        let id = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<RememberFile>(&text).ok())
            .map(|file| file.id);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io(&path)(e)),
        }
        if let (Some(keychain), Some(id)) = (&self.keychain, id) {
            // without the file the remember key opens nothing; it is deleted all the same
            let _ = keychain
                .entry(&id)
                .and_then(|entry| entry.delete_credential());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cost;

    fn vault(name: &str, keychain: Option<Keychain>) -> Vault {
        let folder =
            std::env::temp_dir().join(format!("atena-remember-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        Vault::new(folder)
            .with_cost(Cost::TEST)
            .with_keychain(keychain)
    }

    fn id_of(v: &Vault) -> String {
        let text = fs::read_to_string(v.remember_path()).unwrap();
        serde_json::from_str::<RememberFile>(&text).unwrap().id
    }

    #[test]
    fn the_persistent_sign_in_opens_the_account_until_it_is_turned_off() {
        let keychain = Keychain::memory().unwrap();
        let v = vault("on", Some(keychain.clone()));
        assert!(v.can_remember() && !v.is_remembered());
        assert!(v.remembered().unwrap().is_none());
        let (key, _) = v.create("Ján", "dlhé heslo 123").unwrap();
        let sealed = key.encrypt(b"doc", b"text").unwrap();
        v.remember(&key).unwrap();
        assert!(v.is_remembered());

        // the next start: the same folder and credential store
        let next = Vault::new(v.folder.clone()).with_keychain(Some(keychain.clone()));
        let opened = next.remembered().unwrap().unwrap();
        assert_eq!(opened.decrypt(b"doc", &sealed).unwrap().as_slice(), b"text");
        // the file holds no key in the clear, the credential store holds only the remember key
        let text = fs::read_to_string(v.remember_path()).unwrap();
        assert!(!text.contains(&hex(&*key.0)));
        let id = id_of(&v);
        let stored = keychain.entry(&id).unwrap().get_secret().unwrap();
        assert_ne!(stored.as_slice(), &*key.0);

        // a new password keeps it; turning it off deletes both halves
        v.change_password("Ján", "dlhé heslo 123", "nové heslo 456")
            .unwrap();
        assert!(next.remembered().unwrap().is_some());
        next.forget().unwrap();
        assert!(!v.is_remembered() && v.remembered().unwrap().is_none());
        assert!(matches!(
            keychain.entry(&id).unwrap().get_secret(),
            Err(keyring_core::Error::NoEntry)
        ));
        fs::remove_dir_all(&v.folder).unwrap();
    }

    #[test]
    fn a_sign_in_that_no_longer_fits_is_turned_off() {
        let keychain = Keychain::memory().unwrap();
        let v = vault("fits", Some(keychain.clone()));
        let (key, _) = v.create("Mária", "dlhé heslo 123").unwrap();

        // the folder copied to another computer: its credential store has no remember key
        v.remember(&key).unwrap();
        let elsewhere = Vault::new(v.folder.clone()).with_keychain(Keychain::memory());
        assert!(elsewhere.remembered().unwrap().is_none());
        assert!(!v.remember_path().exists());

        // an altered file opens nothing and is removed
        v.remember(&key).unwrap();
        let text = fs::read_to_string(v.remember_path()).unwrap();
        let mut file: RememberFile = serde_json::from_str(&text).unwrap();
        let first = if file.sealed.starts_with('0') {
            "1"
        } else {
            "0"
        };
        file.sealed.replace_range(..1, first);
        fs::write(v.remember_path(), serde_json::to_string(&file).unwrap()).unwrap();
        assert!(v.remembered().unwrap().is_none());
        assert!(!v.is_remembered());

        // a new account in the folder never opens with the sign-in of the old one
        v.remember(&key).unwrap();
        let kept = fs::read_to_string(v.remember_path()).unwrap();
        let id = id_of(&v);
        let secret = keychain.entry(&id).unwrap().get_secret().unwrap();
        fs::remove_file(v.path()).unwrap();
        v.create("Iná", "iné dlhé heslo").unwrap();
        assert!(!v.is_remembered());
        fs::write(v.remember_path(), kept).unwrap();
        keychain.entry(&id).unwrap().set_secret(&secret).unwrap();
        assert!(v.remembered().unwrap().is_none());
        assert!(!v.is_remembered());

        // remembering again replaces the old remember key
        v.remember(&key).unwrap();
        let old = id_of(&v);
        v.remember(&key).unwrap();
        assert_ne!(id_of(&v), old);
        assert!(keychain.entry(&old).unwrap().get_secret().is_err());
        // without an account the sign-in is turned off
        v.remember(&key).unwrap();
        fs::remove_file(v.path()).unwrap();
        assert!(v.remembered().unwrap().is_none() && !v.is_remembered());
        fs::remove_dir_all(&v.folder).unwrap();
    }

    #[test]
    fn without_a_credential_store_there_is_no_persistent_sign_in() {
        let v = vault("none", None);
        let (key, _) = v.create("Peter", "dlhé heslo 123").unwrap();
        assert!(!v.can_remember());
        assert!(matches!(v.remember(&key), Err(VaultError::NoKeychain)));
        assert!(v.remembered().unwrap().is_none());
        v.forget().unwrap();
        assert!(format!("{:?}", Keychain::memory().unwrap()).contains("memory"));
        fs::remove_dir_all(&v.folder).unwrap();
    }
}
