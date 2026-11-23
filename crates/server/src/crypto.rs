//! The cryptography of the account store, in one place.
//!
//! - Passwords are hashed one way with Argon2id. Each hash carries its own
//!   random salt, and a secret pepper from the key file is mixed in, so a
//!   copy of the database alone is not enough to test guesses.
//! - Usernames are stored encrypted (AES-256-GCM, a fresh random nonce each
//!   time) so they can be shown again, and beside that as a keyed hash
//!   (HMAC-SHA256, the "blind index") of the lower-cased name, which is what
//!   makes a name unique and findable without decrypting anything.
//! - Session tokens are random; only their SHA-256 is stored.
//!
//! All keys come from one master key in a file outside the database. What
//! this protects: a leaked or stolen database file. What it does not: an
//! attacker who can read the key file as well, such as one with full access
//! to the machine.

use std::fs;
use std::io;
use std::path::Path;

use aes_gcm::aead::{Aead, Generate, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;

/// The keys derived from the master key, each used for one thing only.
pub struct Keys {
    index: [u8; KEY_BYTES],
    cipher: Aes256Gcm,
    pepper: [u8; KEY_BYTES],
    params: Params,
}

fn derive(master: &[u8], purpose: &str) -> [u8; KEY_BYTES] {
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(master).expect("HMAC takes a key of any size");
    mac.update(purpose.as_bytes());
    mac.finalize().into_bytes().into()
}

impl Keys {
    /// Keys from a master key. `params` sets the cost of password hashing.
    pub fn new(master: &[u8; KEY_BYTES], params: Params) -> Self {
        let cipher_key = derive(master, "owl username encryption v1");
        Self {
            index: derive(master, "owl username index v1"),
            cipher: Aes256Gcm::new_from_slice(&cipher_key).expect("the key is 32 bytes"),
            pepper: derive(master, "owl password pepper v1"),
            params,
        }
    }

    /// Reads the master key from `path`, creating it with fresh random bytes
    /// (readable by the owner only) if the file does not exist yet.
    pub fn load_or_create(path: &Path, params: Params) -> io::Result<Self> {
        let master: [u8; KEY_BYTES] = match fs::read(path) {
            Ok(bytes) => bytes
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "the key file must hold exactly 32 bytes"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let mut fresh = [0u8; KEY_BYTES];
                rand::fill(&mut fresh);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, fresh)?;
                restrict(path)?;
                fresh
            }
            Err(error) => return Err(error),
        };
        Ok(Self::new(&master, params))
    }

    /// The blind index of a username: the same for every spelling of its case.
    pub fn username_index(&self, username: &str) -> [u8; KEY_BYTES] {
        let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&self.index).expect("HMAC takes a key of any size");
        mac.update(username.to_lowercase().as_bytes());
        mac.finalize().into_bytes().into()
    }

    /// Encrypts a username for storage: the nonce, then the ciphertext.
    /// `bound_to` is checked again on decryption (here: the index of the
    /// row), so a ciphertext moved to another row does not decrypt.
    pub fn encrypt(&self, text: &str, bound_to: &[u8]) -> Vec<u8> {
        let nonce = Nonce::generate();
        let mut out = nonce.to_vec();
        let payload = Payload { msg: text.as_bytes(), aad: bound_to };
        out.extend(self.cipher.encrypt(&nonce, payload).expect("encrypting in memory cannot fail"));
        out
    }

    /// Decrypts what `encrypt` made. `None` if it was damaged, made with
    /// another key, or bound to something else.
    pub fn decrypt(&self, stored: &[u8], bound_to: &[u8]) -> Option<String> {
        if stored.len() < NONCE_BYTES {
            return None;
        }
        let (nonce, ciphertext) = stored.split_at(NONCE_BYTES);
        let nonce = Nonce::try_from(nonce).ok()?;
        let plain = self.cipher.decrypt(&nonce, Payload { msg: ciphertext, aad: bound_to }).ok()?;
        String::from_utf8(plain).ok()
    }

    fn argon(&self) -> Argon2<'_> {
        Argon2::new_with_secret(&self.pepper, Algorithm::Argon2id, Version::V0x13, self.params.clone())
            .expect("the pepper has a valid length")
    }

    /// A one-way hash of a password, as a PHC string that carries its salt and cost.
    pub fn hash_password(&self, password: &str) -> String {
        self.argon().hash_password(password.as_bytes()).expect("hashing a password cannot fail").to_string()
    }

    pub fn verify_password(&self, password: &str, stored: &str) -> bool {
        PasswordHash::new(stored).is_ok_and(|hash| self.argon().verify_password(password.as_bytes(), &hash).is_ok())
    }
}

/// A new session token for the client, and the hash of it that is stored.
pub fn new_token() -> (String, [u8; 32]) {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let hash = token_hash(&token);
    (token, hash)
}

pub fn token_hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// The cost of password hashing: the OWASP minimum for Argon2id (19 MiB,
/// 2 passes, 1 lane), which a Raspberry Pi does in a fraction of a second.
pub fn password_cost() -> Params {
    Params::new(19 * 1024, 2, 1, None).expect("valid Argon2 parameters")
}

#[cfg(unix)]
fn restrict(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Keys {
        // Cheap hashing: these tests are about behaviour, not cost.
        Keys::new(&[7; KEY_BYTES], Params::new(64, 1, 1, None).unwrap())
    }

    #[test]
    fn a_password_hash_verifies_only_the_right_password_and_never_repeats() {
        let keys = keys();
        let first = keys.hash_password("correct horse");
        let second = keys.hash_password("correct horse");
        assert!(first.starts_with("$argon2id$"));
        assert_ne!(first, second, "each hash has its own salt");
        assert!(keys.verify_password("correct horse", &first));
        assert!(!keys.verify_password("wrong horse", &first));
        assert!(!keys.verify_password("correct horse", "not a hash"));
    }

    #[test]
    fn a_password_hash_made_with_another_key_does_not_verify() {
        let hash = keys().hash_password("correct horse");
        let other = Keys::new(&[8; KEY_BYTES], Params::new(64, 1, 1, None).unwrap());
        assert!(!other.verify_password("correct horse", &hash), "the pepper is part of the hash");
    }

    #[test]
    fn a_username_encrypts_differently_each_time_and_decrypts_to_itself() {
        let keys = keys();
        let (a, b) = (keys.encrypt("Minerva", b"row 1"), keys.encrypt("Minerva", b"row 1"));
        assert_ne!(a, b, "a fresh nonce each time");
        assert!(!a.windows(7).any(|w| w == b"Minerva"), "the name is not readable in what is stored");
        assert_eq!(keys.decrypt(&a, b"row 1").as_deref(), Some("Minerva"));
        assert_eq!(keys.decrypt(&a, b"row 2"), None, "moved to another row it does not decrypt");
        let mut damaged = a.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert_eq!(keys.decrypt(&damaged, b"row 1"), None, "a changed byte is detected");
    }

    #[test]
    fn the_username_index_ignores_case_and_depends_on_the_key() {
        let keys = keys();
        assert_eq!(keys.username_index("Minerva"), keys.username_index("minerva"));
        assert_ne!(keys.username_index("Minerva"), keys.username_index("Minerv"));
        let other = Keys::new(&[8; KEY_BYTES], Params::new(64, 1, 1, None).unwrap());
        assert_ne!(keys.username_index("Minerva"), other.username_index("Minerva"));
    }

    #[test]
    fn a_session_token_is_random_and_stored_only_as_a_hash() {
        let (token, hash) = new_token();
        let (other, _) = new_token();
        assert_eq!(token.len(), 64);
        assert_ne!(token, other);
        assert_eq!(token_hash(&token), hash);
    }
}
