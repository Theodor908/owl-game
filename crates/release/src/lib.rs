//! What a published build is made of, and the signature that lets a phone
//! trust it.
//!
//! A release puts up to three files on the server: the pack (scripts,
//! scenes, pictures and content), the native library (the rules), and the
//! whole game as an APK. The game downloads the first two by itself, so
//! each is described by an [`Entry`] signed with a key that never leaves
//! the PC that publishes. The game carries the public half ([`PUBLIC_KEY`])
//! and refuses any download whose signature does not verify: the server
//! alone cannot hand out code.
//!
//! `owl-local` uses the checking half; the `owl-release` tool the rest.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The first line of every signed message. Changing what is signed, or how
/// the game checks it, changes this, and with it the shell id.
pub const FORMAT: &str = "owl-release-1";

/// The public half of the release key, in hex. The private half is made by
/// `owl-release keygen` and lives outside the project.
pub const PUBLIC_KEY: &str = include_str!("../release.pub");

/// What `owl-release plan` exits with when the whole game must be published.
pub const FULL_RELEASE: i32 = 10;

/// One downloadable file of a build, as `version.json` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// `pack` or `lib`.
    pub kind: String,
    /// The build it was published with.
    pub build: u64,
    /// The shell (engine, project settings, loader) it runs in.
    pub shell: String,
    /// The native library it is (`lib`) or needs (`pack`): that file's SHA-256.
    pub native: String,
    /// The platform a library is built for, such as `android.arm64`. Empty for a pack.
    pub platform: String,
    /// The SHA-256 of the file itself.
    pub sha256: String,
    /// Its size in bytes.
    pub size: u64,
}

impl Entry {
    /// Describes a file about to be published.
    pub fn of(kind: &str, build: u64, shell: &str, native: &str, platform: &str, file: &[u8]) -> Self {
        Self {
            kind: kind.into(),
            build,
            shell: shell.into(),
            native: native.into(),
            platform: platform.into(),
            sha256: sha256_hex(file),
            size: file.len() as u64,
        }
    }

    /// Reads an entry from its JSON. Numbers may arrive as `5` or `5.0`:
    /// Godot writes every number back with a fraction.
    pub fn from_json(value: &Value) -> Option<Self> {
        let text = |name: &str| value.get(name)?.as_str().map(str::to_owned);
        let number = |name: &str| {
            let field = value.get(name)?;
            field.as_u64().or_else(|| field.as_f64().filter(|n| n.fract() == 0.0 && *n >= 0.0).map(|n| n as u64))
        };
        Some(Self {
            kind: text("kind")?,
            build: number("build")?,
            shell: text("shell")?,
            native: text("native")?,
            platform: text("platform")?,
            sha256: text("sha256")?,
            size: number("size")?,
        })
    }

    /// The entry as it stands in `version.json`, with its signature.
    pub fn to_json(&self, signature: &str) -> Value {
        json!({
            "kind": self.kind, "build": self.build, "shell": self.shell, "native": self.native,
            "platform": self.platform, "sha256": self.sha256, "size": self.size, "sig": signature,
        })
    }

    /// The exact text that is signed. Every field is in it, so none can be
    /// changed on the way: not the build number, and not which library a
    /// pack goes with.
    pub fn message(&self) -> String {
        format!(
            "{FORMAT}\nkind={}\nbuild={}\nshell={}\nnative={}\nplatform={}\nsha256={}\nsize={}\n",
            self.kind, self.build, self.shell, self.native, self.platform, self.sha256, self.size
        )
    }

    /// Signs the entry. Returns the signature in hex.
    pub fn sign(&self, secret: &[u8; 32]) -> String {
        to_hex(&SigningKey::from_bytes(secret).sign(self.message().as_bytes()).to_bytes())
    }

    /// True if `signature` is this entry signed by the holder of `public_key`.
    pub fn verify(&self, public_key: &str, signature: &str) -> bool {
        let Some(key) = from_hex::<32>(public_key.trim()).and_then(|bytes| VerifyingKey::from_bytes(&bytes).ok())
        else {
            return false;
        };
        let Some(signature) = from_hex::<64>(signature.trim()) else { return false };
        key.verify(self.message().as_bytes(), &Signature::from_bytes(&signature)).is_ok()
    }
}

/// True if `entry_json` (an entry of `version.json`, `sig` included) is
/// signed with the release key. What the game asks before using a download.
pub fn signed_by_release_key(entry_json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(entry_json) else { return false };
    let (Some(entry), Some(signature)) = (Entry::from_json(&value), value.get("sig").and_then(Value::as_str)) else {
        return false;
    };
    entry.verify(PUBLIC_KEY, signature)
}

/// The public half of a private key, in hex.
pub fn public_key_of(secret: &[u8; 32]) -> String {
    to_hex(SigningKey::from_bytes(secret).verifying_key().as_bytes())
}

/// Names the shell a pack or a library needs: the engine and the parts of
/// the game that only a whole APK can change. `files` are name and content
/// pairs; line endings do not count, so a checkout on another PC names the
/// same shell.
pub fn shell_id(godot: &str, files: &[(String, Vec<u8>)]) -> String {
    let mut hash = Sha256::new();
    hash.update(FORMAT.as_bytes());
    hash.update(PUBLIC_KEY.trim().as_bytes());
    hash.update(godot.trim().as_bytes());
    for (name, content) in files {
        let plain: Vec<u8> = content.iter().copied().filter(|byte| *byte != b'\r').collect();
        hash.update(name.as_bytes());
        hash.update((plain.len() as u64).to_le_bytes());
        hash.update(&plain);
    }
    to_hex(&hash.finalize()[..8])
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    to_hex(&Sha256::digest(bytes))
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn from_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != 2 * N || !text.is_ascii() {
        return None;
    }
    let mut bytes = [0; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * index..2 * index + 2], 16).ok()?;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: [u8; 32] = [7; 32];

    fn pack() -> Entry {
        Entry::of("pack", 5, "shell", "native", "", b"the pack")
    }

    #[test]
    fn a_signed_entry_verifies_only_with_its_own_key() {
        let signature = pack().sign(&SECRET);
        assert!(pack().verify(&public_key_of(&SECRET), &signature));
        assert!(!pack().verify(&public_key_of(&[8; 32]), &signature), "another key");
        assert!(!pack().verify("not a key", &signature));
        assert!(!pack().verify(&public_key_of(&SECRET), "not a signature"));
    }

    #[test]
    fn changing_any_field_breaks_the_signature() {
        let key = public_key_of(&SECRET);
        let signature = pack().sign(&SECRET);
        let changes: [fn(&mut Entry); 7] = [
            |entry| entry.kind = "lib".into(),
            |entry| entry.build = 6,
            |entry| entry.shell = "other".into(),
            |entry| entry.native = "other".into(),
            |entry| entry.platform = "android.arm64".into(),
            |entry| entry.sha256 = sha256_hex(b"another pack"),
            |entry| entry.size += 1,
        ];
        for change in changes {
            let mut entry = pack();
            change(&mut entry);
            assert!(!entry.verify(&key, &signature), "{entry:?}");
        }
    }

    #[test]
    fn an_entry_survives_godots_numbers() {
        let plain = pack().to_json("00");
        assert_eq!(Entry::from_json(&plain), Some(pack()));
        let from_godot: Value = serde_json::from_str(&plain.to_string().replace(":5,", ":5.0,")).unwrap();
        assert_eq!(Entry::from_json(&from_godot), Some(pack()));
        assert_eq!(Entry::from_json(&json!({"kind": "pack"})), None, "fields missing");
    }

    #[test]
    fn only_the_release_key_is_accepted_by_the_game() {
        let forged = pack().to_json(&pack().sign(&SECRET)).to_string();
        assert!(!signed_by_release_key(&forged), "signed, but not with the release key");
        assert!(!signed_by_release_key("{}"));
        assert!(!signed_by_release_key("not json"));
    }

    #[test]
    fn the_shell_id_ignores_line_endings_and_nothing_else() {
        let id = |godot: &str, text: &str| shell_id(godot, &[("project.godot".into(), text.as_bytes().to_vec())]);
        assert_eq!(id("4.7.2", "a\nb\n"), id("4.7.2", "a\r\nb\r\n"));
        assert_ne!(id("4.7.2", "a\nb\n"), id("4.7.2", "a\nc\n"));
        assert_ne!(id("4.7.2", "a\nb\n"), id("4.8", "a\nb\n"));
    }
}
