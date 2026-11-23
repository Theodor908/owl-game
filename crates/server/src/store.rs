//! Accounts, sessions and friends, in one SQLite file.
//!
//! The store keeps as little as the game needs: a name, a password hash,
//! who is friends with whom, and when each of those was made. No e-mail,
//! no address, no device or network details. Everything a user has here can
//! be exported (`export`) and erased (`delete_account`).

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use thiserror::Error;

use crate::crypto::{self, Keys};
use crate::limits::{Backoff, Clock, retry_seconds, wait_text};

pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 14;
/// NIST SP 800-63B-4: at least 15 characters when a password is the only factor.
pub const PASSWORD_MIN: usize = 15;
pub const PASSWORD_MAX: usize = 128;
/// A session ends this long after it was opened, whatever happens.
pub const SESSION_SECONDS: i64 = 30 * 24 * 60 * 60;
/// A session also ends when it has not been used for this long.
pub const IDLE_SECONDS: i64 = 14 * 24 * 60 * 60;
/// An account nobody has used for this long is erased.
pub const INACTIVE_SECONDS: i64 = 365 * 24 * 60 * 60;
/// Activity is written down at most this often per account, to spare the disk.
const ACTIVITY_STEP_SECONDS: i64 = 60 * 60;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("a username has 3 to 14 letters, digits or underscores")]
    BadUsername,
    #[error("a password has at least 15 characters; a few words work well")]
    BadPassword,
    #[error("that username is taken")]
    UsernameTaken,
    #[error("wrong username or password")]
    WrongCredentials,
    /// The name has to wait this many seconds before its next try.
    #[error("too many wrong passwords; try again in {}", wait_text(*.0))]
    TooManyAttempts(u64),
    #[error("read the privacy notice and confirm it to make an account")]
    NoticeNotAcknowledged,
    #[error("you are not logged in")]
    NotLoggedIn,
    #[error("no player has that username")]
    NoSuchUser,
    #[error("you cannot add yourself")]
    Yourself,
    #[error("there is no request from that player")]
    NoRequest,
    #[error("a guest cannot do that; make an account first")]
    GuestsCannot,
    #[error("the account store failed")]
    Database,
}

impl From<rusqlite::Error> for StoreError {
    fn from(_: rusqlite::Error) -> Self {
        StoreError::Database
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub username: String,
    /// A guest: a made-up name and no password, gone when its session ends.
    pub guest: bool,
}

/// A user's friends and open requests, by username.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Friends {
    pub friends: Vec<String>,
    /// Requests others sent to this user.
    pub incoming: Vec<String>,
    /// Requests this user sent.
    pub outgoing: Vec<String>,
}

/// Everything the store holds about one user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Export {
    pub username: String,
    pub guest: bool,
    pub created_at: i64,
    /// When the user confirmed they had read the privacy notice, and which version it was.
    pub notice_acknowledged_at: i64,
    pub notice_version: u32,
    /// When the account was last used. It is erased a year after that.
    pub last_active_at: i64,
    pub friends: Friends,
    /// When each of the user's live sessions was opened and when it ends.
    pub sessions: Vec<(i64, i64)>,
    pub note: &'static str,
}

pub struct Store {
    db: Mutex<Connection>,
    keys: Keys,
    /// Recent wrong passwords by username index (see `limits`).
    failures: Mutex<Backoff<[u8; 32]>>,
    /// The time the waits between wrong passwords are measured in.
    clock: Clock,
    /// The version of the privacy notice the server shows now. Confirming
    /// the notice when registering records this one.
    notice_version: u32,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn valid_username(name: &str) -> bool {
    (USERNAME_MIN..=USERNAME_MAX).contains(&name.chars().count())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn valid_password(password: &str) -> bool {
    (PASSWORD_MIN..=PASSWORD_MAX).contains(&password.chars().count())
}

impl Store {
    pub fn open(path: &Path, keys: Keys) -> Result<Self, StoreError> {
        Self::with(Connection::open(path)?, keys)
    }

    pub fn in_memory(keys: Keys) -> Result<Self, StoreError> {
        Self::with(Connection::open_in_memory()?, keys)
    }

    fn with(db: Connection, keys: Keys) -> Result<Self, StoreError> {
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA secure_delete = ON;
             PRAGMA busy_timeout = 5000;
             CREATE TABLE IF NOT EXISTS users (
                 id INTEGER PRIMARY KEY,
                 username_index BLOB NOT NULL UNIQUE,
                 username_cipher BLOB NOT NULL,
                 password_hash TEXT NOT NULL,
                 key_version INTEGER NOT NULL DEFAULT 1,
                 created_at INTEGER NOT NULL,
                 notice_acknowledged_at INTEGER NOT NULL,
                 guest INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 token_hash BLOB PRIMARY KEY,
                 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 created_at INTEGER NOT NULL,
                 last_used_at INTEGER NOT NULL,
                 expires_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS friends (
                 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 friend_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                 accepted INTEGER NOT NULL,
                 created_at INTEGER NOT NULL,
                 PRIMARY KEY (user_id, friend_id)
             );",
        )?;
        // A database made by an older server gets the columns added since.
        // Accounts that were there count as used today, so none is erased at
        // once, and as having seen the first version of the notice.
        let has = |column: &str| -> Result<bool, StoreError> {
            let count: i64 = db.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = ?1",
                params![column],
                |row| row.get(0),
            )?;
            Ok(count > 0)
        };
        if !has("guest")? {
            db.execute("ALTER TABLE users ADD COLUMN guest INTEGER NOT NULL DEFAULT 0", [])?;
        }
        if !has("last_active_at")? {
            db.execute("ALTER TABLE users ADD COLUMN last_active_at INTEGER NOT NULL DEFAULT 0", [])?;
            db.execute("UPDATE users SET last_active_at = ?1", params![now()])?;
        }
        if !has("notice_version")? {
            db.execute("ALTER TABLE users ADD COLUMN notice_version INTEGER NOT NULL DEFAULT 1", [])?;
        }
        Ok(Self {
            db: Mutex::new(db),
            keys,
            failures: Mutex::new(Backoff::default()),
            clock: Clock::system(),
            notice_version: 1,
        })
    }

    /// Sets the version of the privacy notice the server shows now.
    #[must_use]
    pub fn with_notice_version(mut self, version: u32) -> Self {
        self.notice_version = version;
        self
    }

    pub fn notice_version(&self) -> u32 {
        self.notice_version
    }

    /// Measures the waits between wrong passwords with `clock`.
    #[must_use]
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Erases what has run out: sessions past their end, guests without a
    /// session, and accounts nobody has used for a year.
    pub fn sweep(&self) -> Result<(), StoreError> {
        let db = self.db()?;
        Self::sweep_in(&db, now())
    }

    fn sweep_in(db: &Connection, at: i64) -> Result<(), StoreError> {
        db.execute("DELETE FROM sessions WHERE expires_at <= ?1 OR last_used_at <= ?2", params![at, at - IDLE_SECONDS])?;
        // A guest lives as long as its session.
        db.execute("DELETE FROM users WHERE guest = 1 AND id NOT IN (SELECT user_id FROM sessions)", [])?;
        db.execute("DELETE FROM users WHERE last_active_at <= ?1", params![at - INACTIVE_SECONDS])?;
        Ok(())
    }

    /// The version of the privacy notice a user last confirmed.
    pub fn accepted_notice(&self, user: &User) -> Result<u32, StoreError> {
        let db = self.db()?;
        Ok(db.query_row("SELECT notice_version FROM users WHERE id = ?1", params![user.id], |row| row.get(0))?)
    }

    /// The user confirms they read the privacy notice the server shows now.
    pub fn accept_notice(&self, user: &User) -> Result<(), StoreError> {
        let db = self.db()?;
        db.execute(
            "UPDATE users SET notice_version = ?1, notice_acknowledged_at = ?2 WHERE id = ?3",
            params![self.notice_version, now(), user.id],
        )?;
        Ok(())
    }

    fn db(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StoreError> {
        self.db.lock().map_err(|_| StoreError::Database)
    }

    fn open_session(&self, db: &Connection, user_id: i64) -> Result<String, StoreError> {
        let (token, hash) = crypto::new_token();
        let at = now();
        Self::sweep_in(db, at)?;
        db.execute("UPDATE users SET last_active_at = ?1 WHERE id = ?2", params![at, user_id])?;
        db.execute(
            "INSERT INTO sessions (token_hash, user_id, created_at, last_used_at, expires_at) VALUES (?1, ?2, ?3, ?3, ?4)",
            params![hash.as_slice(), user_id, at, at + SESSION_SECONDS],
        )?;
        Ok(token)
    }

    /// Hashes a password. The caller limits how many hashes run at once
    /// (`limits::HashGate`), and no database lock is held meanwhile, since
    /// each hash is slow on purpose.
    fn hash(&self, password: &str) -> String {
        self.keys.hash_password(password)
    }

    /// Checks a password for the name with blind index `index`, against its
    /// stored hash, or against none for a name that does not exist. After a
    /// few wrong passwords for a name, each further try has to wait longer,
    /// whether or not the name exists.
    fn check_password(&self, index: &[u8; 32], password: &str, stored: Option<&str>) -> Result<(), StoreError> {
        let failures = || self.failures.lock().map_err(|_| StoreError::Database);
        failures()?.check(index, self.clock.now()).map_err(|wait| StoreError::TooManyAttempts(retry_seconds(wait)))?;
        let ok = match stored {
            Some(hash) => self.keys.verify_password(password, hash),
            None => {
                // Spend the same effort for a name that does not exist.
                let _ = self.hash(password);
                false
            }
        };
        let mut failures = failures()?;
        if ok {
            failures.clear(index);
            Ok(())
        } else {
            failures.fail(*index, self.clock.now());
            Err(StoreError::WrongCredentials)
        }
    }

    /// Checks the password of a logged-in user, under the same waits as logging in.
    fn check_own_password(&self, user: &User, password: &str) -> Result<(), StoreError> {
        let stored: String =
            self.db()?.query_row("SELECT password_hash FROM users WHERE id = ?1", params![user.id], |r| r.get(0))?;
        self.check_password(&self.keys.username_index(&user.username), password, Some(&stored))
    }

    /// Makes an account and logs it in. Returns the session token.
    /// `notice_acknowledged`: the player confirmed they read the privacy notice.
    pub fn register(&self, username: &str, password: &str, notice_acknowledged: bool) -> Result<String, StoreError> {
        if !notice_acknowledged {
            return Err(StoreError::NoticeNotAcknowledged);
        }
        if !valid_username(username) {
            return Err(StoreError::BadUsername);
        }
        if !valid_password(password) {
            return Err(StoreError::BadPassword);
        }
        let index = self.keys.username_index(username);
        let hash = self.hash(password);
        let db = self.db()?;
        let at = now();
        let inserted = db.execute(
            "INSERT OR IGNORE INTO users (username_index, username_cipher, password_hash, created_at,
                 notice_acknowledged_at, notice_version, last_active_at)
             VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?4)",
            params![index.as_slice(), self.keys.encrypt(username, &index), hash, at, self.notice_version],
        )?;
        if inserted == 0 {
            return Err(StoreError::UsernameTaken);
        }
        self.open_session(&db, db.last_insert_rowid())
    }

    /// Makes a guest: a made-up name, no password, one session. Returns the
    /// session token and the name. The guest is erased when the session ends.
    pub fn register_guest(&self, notice_acknowledged: bool) -> Result<(String, String), StoreError> {
        if !notice_acknowledged {
            return Err(StoreError::NoticeNotAcknowledged);
        }
        const LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let db = self.db()?;
        let at = now();
        for _ in 0..20 {
            let mut bytes = [0u8; 5];
            rand::fill(&mut bytes);
            let tail: String = bytes.iter().map(|b| LETTERS[usize::from(*b) % LETTERS.len()] as char).collect();
            let name = format!("Guest_{tail}");
            let index = self.keys.username_index(&name);
            // "!" is not a password hash, so no password ever logs a guest in.
            let inserted = db.execute(
                "INSERT OR IGNORE INTO users (username_index, username_cipher, password_hash, created_at,
                     notice_acknowledged_at, guest, notice_version, last_active_at)
                 VALUES (?1, ?2, '!', ?3, ?3, 1, ?4, ?3)",
                params![index.as_slice(), self.keys.encrypt(&name, &index), at, self.notice_version],
            )?;
            if inserted == 1 {
                let id = db.last_insert_rowid();
                // Opening the session sweeps guests without one, so the
                // session row has to exist first.
                let (token, hash) = crypto::new_token();
                db.execute(
                    "INSERT INTO sessions (token_hash, user_id, created_at, last_used_at, expires_at) VALUES (?1, ?2, ?3, ?3, ?4)",
                    params![hash.as_slice(), id, at, at + SESSION_SECONDS],
                )?;
                return Ok((token, name));
            }
        }
        Err(StoreError::Database)
    }

    /// Checks a password and opens a session. A wrong name and a wrong
    /// password give the same answer, and both cost the same time.
    pub fn login(&self, username: &str, password: &str) -> Result<(String, String), StoreError> {
        let index = self.keys.username_index(username);
        let found: Option<(i64, Vec<u8>, String)> = self
            .db()?
            .query_row(
                "SELECT id, username_cipher, password_hash FROM users WHERE username_index = ?1",
                params![index.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        self.check_password(&index, password, found.as_ref().map(|(_, _, hash)| hash.as_str()))?;
        let (id, cipher, _) = found.ok_or(StoreError::WrongCredentials)?;
        let name = self.keys.decrypt(&cipher, &index).ok_or(StoreError::Database)?;
        let db = self.db()?;
        Ok((self.open_session(&db, id)?, name))
    }

    /// Ends a session. A guest has only the one, so the guest goes with it.
    pub fn logout(&self, token: &str) -> Result<(), StoreError> {
        let hash = crypto::token_hash(token);
        let db = self.db()?;
        db.execute(
            "DELETE FROM users WHERE guest = 1 AND id IN (SELECT user_id FROM sessions WHERE token_hash = ?1)",
            params![hash.as_slice()],
        )?;
        db.execute("DELETE FROM sessions WHERE token_hash = ?1", params![hash.as_slice()])?;
        Ok(())
    }

    /// The user a live session token belongs to.
    pub fn user_of(&self, token: &str) -> Result<User, StoreError> {
        let hash = crypto::token_hash(token);
        let db = self.db()?;
        let at = now();
        let found: Option<(i64, Vec<u8>, Vec<u8>, bool)> = db
            .query_row(
                "SELECT users.id, users.username_cipher, users.username_index, users.guest FROM sessions
                 JOIN users ON users.id = sessions.user_id
                 WHERE sessions.token_hash = ?1 AND sessions.expires_at > ?2 AND sessions.last_used_at > ?3",
                params![hash.as_slice(), at, at - IDLE_SECONDS],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (id, cipher, index, guest) = found.ok_or(StoreError::NotLoggedIn)?;
        db.execute("UPDATE sessions SET last_used_at = ?1 WHERE token_hash = ?2", params![at, hash.as_slice()])?;
        db.execute(
            "UPDATE users SET last_active_at = ?1 WHERE id = ?2 AND last_active_at <= ?3",
            params![at, id, at - ACTIVITY_STEP_SECONDS],
        )?;
        Ok(User { id, username: self.keys.decrypt(&cipher, &index).ok_or(StoreError::Database)?, guest })
    }

    fn name_of(&self, db: &Connection, id: i64) -> Result<String, StoreError> {
        let (cipher, index): (Vec<u8>, Vec<u8>) = db.query_row(
            "SELECT username_cipher, username_index FROM users WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        self.keys.decrypt(&cipher, &index).ok_or(StoreError::Database)
    }

    fn id_of(&self, db: &Connection, username: &str) -> Result<i64, StoreError> {
        let index = self.keys.username_index(username);
        db.query_row("SELECT id FROM users WHERE username_index = ?1", params![index.as_slice()], |r| r.get(0))
            .optional()?
            .ok_or(StoreError::NoSuchUser)
    }

    /// The exact spelling of a username, if a player has it. Only a whole
    /// name can be looked up; there is no way to list or browse players.
    pub fn find(&self, username: &str) -> Result<String, StoreError> {
        let db = self.db()?;
        let id = self.id_of(&db, username)?;
        self.name_of(&db, id)
    }

    pub fn change_password(&self, user: &User, old: &str, new: &str) -> Result<(), StoreError> {
        if user.guest {
            return Err(StoreError::GuestsCannot);
        }
        if !valid_password(new) {
            return Err(StoreError::BadPassword);
        }
        self.check_own_password(user, old)?;
        let hash = self.hash(new);
        let db = self.db()?;
        db.execute("UPDATE users SET password_hash = ?1 WHERE id = ?2", params![hash, user.id])?;
        // Every other device has to log in again.
        db.execute("DELETE FROM sessions WHERE user_id = ?1", params![user.id])?;
        Ok(())
    }

    /// Gives the account another name, if it is free. Friends keep the link.
    pub fn change_username(&self, user: &User, password: &str, new: &str) -> Result<(), StoreError> {
        if user.guest {
            return Err(StoreError::GuestsCannot);
        }
        if !valid_username(new) {
            return Err(StoreError::BadUsername);
        }
        self.check_own_password(user, password)?;
        let db = self.db()?;
        let index = self.keys.username_index(new);
        let taken: Option<i64> = db
            .query_row("SELECT id FROM users WHERE username_index = ?1", params![index.as_slice()], |r| r.get(0))
            .optional()?;
        if taken.is_some_and(|id| id != user.id) {
            return Err(StoreError::UsernameTaken);
        }
        db.execute(
            "UPDATE users SET username_index = ?1, username_cipher = ?2 WHERE id = ?3",
            params![index.as_slice(), self.keys.encrypt(new, &index), user.id],
        )?;
        Ok(())
    }

    /// Erases the account and everything tied to it: sessions and friend links.
    /// A guest has no password, so theirs is not asked for.
    pub fn delete_account(&self, user: &User, password: &str) -> Result<(), StoreError> {
        if !user.guest {
            self.check_own_password(user, password)?;
        }
        self.db()?.execute("DELETE FROM users WHERE id = ?1", params![user.id])?;
        Ok(())
    }

    /// Asks `username` to be friends, or accepts if they already asked.
    pub fn request_friend(&self, user: &User, username: &str) -> Result<(), StoreError> {
        let db = self.db()?;
        let other = self.id_of(&db, username)?;
        if other == user.id {
            return Err(StoreError::Yourself);
        }
        let they_asked: bool = db
            .query_row("SELECT 1 FROM friends WHERE user_id = ?1 AND friend_id = ?2", params![other, user.id], |_| Ok(true))
            .optional()?
            .unwrap_or(false);
        let at = now();
        if they_asked {
            db.execute("UPDATE friends SET accepted = 1 WHERE user_id = ?1 AND friend_id = ?2", params![other, user.id])?;
            db.execute(
                "INSERT OR REPLACE INTO friends (user_id, friend_id, accepted, created_at) VALUES (?1, ?2, 1, ?3)",
                params![user.id, other, at],
            )?;
        } else {
            db.execute(
                "INSERT OR IGNORE INTO friends (user_id, friend_id, accepted, created_at) VALUES (?1, ?2, 0, ?3)",
                params![user.id, other, at],
            )?;
        }
        Ok(())
    }

    /// Accepts a request `username` sent.
    pub fn accept_friend(&self, user: &User, username: &str) -> Result<(), StoreError> {
        let db = self.db()?;
        let other = self.id_of(&db, username)?;
        let asked: bool = db
            .query_row(
                "SELECT 1 FROM friends WHERE user_id = ?1 AND friend_id = ?2 AND accepted = 0",
                params![other, user.id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !asked {
            return Err(StoreError::NoRequest);
        }
        drop(db);
        self.request_friend(user, username)
    }

    /// Ends a friendship, withdraws a request or turns one down.
    pub fn remove_friend(&self, user: &User, username: &str) -> Result<(), StoreError> {
        let db = self.db()?;
        let other = self.id_of(&db, username)?;
        db.execute(
            "DELETE FROM friends WHERE (user_id = ?1 AND friend_id = ?2) OR (user_id = ?2 AND friend_id = ?1)",
            params![user.id, other],
        )?;
        Ok(())
    }

    pub fn friends(&self, user: &User) -> Result<Friends, StoreError> {
        let db = self.db()?;
        let names = |sql: &str| -> Result<Vec<String>, StoreError> {
            let mut statement = db.prepare(sql)?;
            let ids: Vec<i64> = statement.query_map(params![user.id], |row| row.get(0))?.collect::<Result<_, _>>()?;
            let mut names = ids.into_iter().map(|id| self.name_of(&db, id)).collect::<Result<Vec<_>, _>>()?;
            names.sort_by_key(|name| name.to_lowercase());
            Ok(names)
        };
        Ok(Friends {
            friends: names("SELECT friend_id FROM friends WHERE user_id = ?1 AND accepted = 1")?,
            incoming: names("SELECT user_id FROM friends WHERE friend_id = ?1 AND accepted = 0")?,
            outgoing: names("SELECT friend_id FROM friends WHERE user_id = ?1 AND accepted = 0")?,
        })
    }

    /// The friends of the player with this username.
    pub fn friends_of_name(&self, username: &str) -> Result<Vec<String>, StoreError> {
        let id = {
            let db = self.db()?;
            self.id_of(&db, username)?
        };
        Ok(self.friends(&User { id, username: username.to_string(), guest: false })?.friends)
    }

    pub fn are_friends(&self, user: &User, username: &str) -> bool {
        self.friends(user).is_ok_and(|f| f.friends.iter().any(|name| name.eq_ignore_ascii_case(username)))
    }

    /// Everything held about a user, for them to read or take elsewhere.
    pub fn export(&self, user: &User) -> Result<Export, StoreError> {
        let friends = self.friends(user)?;
        let db = self.db()?;
        let (created_at, notice_acknowledged_at, notice_version, last_active_at): (i64, i64, u32, i64) = db.query_row(
            "SELECT created_at, notice_acknowledged_at, notice_version, last_active_at FROM users WHERE id = ?1",
            params![user.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        let mut statement =
            db.prepare("SELECT created_at, expires_at FROM sessions WHERE user_id = ?1 AND expires_at > ?2")?;
        let sessions = statement
            .query_map(params![user.id, now()], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(Export {
            username: user.username.clone(),
            guest: user.guest,
            created_at,
            notice_acknowledged_at,
            notice_version,
            last_active_at,
            friends,
            sessions,
            note: "Times are seconds since 1970-01-01 UTC. Your password is stored only as a one-way hash and cannot be exported. Matches are not stored. Your internet address is held only in the server's memory, for at most an hour, to stop abuse; it is not linked to your account.",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::Params;

    fn store() -> Store {
        let keys = Keys::new(&[3; crypto::KEY_BYTES], Params::new(64, 1, 1, None).unwrap());
        Store::in_memory(keys).unwrap()
    }

    fn user(store: &Store, name: &str) -> User {
        let token = store.register(name, "a long password", true).unwrap();
        store.user_of(&token).unwrap()
    }

    #[test]
    fn registering_checks_the_name_the_password_and_the_privacy_notice() {
        let store = store();
        assert_eq!(store.register("Minerva", "a long password", false), Err(StoreError::NoticeNotAcknowledged));
        assert_eq!(store.register("ab", "a long password", true), Err(StoreError::BadUsername));
        assert_eq!(store.register("has space", "a long password", true), Err(StoreError::BadUsername));
        assert_eq!(store.register("Minerva", "fourteen chars", true), Err(StoreError::BadPassword));
        store.register("Minerva", "a long password", true).unwrap();
    }

    #[test]
    fn a_username_is_unique_whatever_its_case() {
        let store = store();
        store.register("Minerva", "a long password", true).unwrap();
        assert_eq!(store.register("minerva", "another password", true), Err(StoreError::UsernameTaken));
        assert_eq!(store.find("MINERVA").as_deref(), Ok("Minerva"), "found by any case, shown as registered");
        assert_eq!(store.find("Nobody"), Err(StoreError::NoSuchUser));
    }

    #[test]
    fn logging_in_needs_the_right_password_and_gives_a_working_session() {
        let store = store();
        store.register("Minerva", "a long password", true).unwrap();
        assert_eq!(store.login("Minerva", "a wrong password"), Err(StoreError::WrongCredentials));
        assert_eq!(store.login("Nobody", "a long password"), Err(StoreError::WrongCredentials), "same answer for no such name");
        let (token, name) = store.login("minerva", "a long password").unwrap();
        assert_eq!(name, "Minerva");
        assert_eq!(store.user_of(&token).unwrap().username, "Minerva");
        store.logout(&token).unwrap();
        assert_eq!(store.user_of(&token), Err(StoreError::NotLoggedIn));
        assert_eq!(store.user_of("not a token"), Err(StoreError::NotLoggedIn));
    }

    #[test]
    fn after_a_few_wrong_passwords_each_further_try_has_to_wait_a_growing_time() {
        use crate::limits::{FREE_FAILURES, wait_after};
        let clock = Clock::manual();
        let store = store().with_clock(clock.clone());
        let token = store.register("Minerva", "a long password", true).unwrap();
        for _ in 0..FREE_FAILURES {
            assert_eq!(store.login("Minerva", "a wrong password"), Err(StoreError::WrongCredentials));
        }
        assert_eq!(store.login("Minerva", "a long password"), Err(StoreError::TooManyAttempts(1)), "even the right one waits");
        // Once the wait is over the right password works and the count is cleared.
        clock.advance(wait_after(FREE_FAILURES));
        store.login("Minerva", "a long password").unwrap();
        assert!(store.failures.lock().unwrap().is_empty());

        // A name that does not exist waits the same way.
        for _ in 0..FREE_FAILURES {
            assert_eq!(store.login("Nobody", "a wrong password"), Err(StoreError::WrongCredentials));
        }
        assert_eq!(store.login("Nobody", "a wrong password"), Err(StoreError::TooManyAttempts(1)));

        // A logged-in user's own password is guarded the same way.
        let user = store.user_of(&token).unwrap();
        for _ in 0..FREE_FAILURES {
            assert_eq!(store.change_username(&user, "a wrong password", "Minnie"), Err(StoreError::WrongCredentials));
        }
        assert_eq!(store.delete_account(&user, "a long password"), Err(StoreError::TooManyAttempts(1)));
    }

    #[test]
    fn a_guest_plays_under_a_made_up_name_and_is_erased_with_its_session() {
        let store = store();
        assert_eq!(store.register_guest(false), Err(StoreError::NoticeNotAcknowledged));
        let (token, name) = store.register_guest(true).unwrap();
        assert!(name.starts_with("Guest_") && name.len() == 11);
        let guest = store.user_of(&token).unwrap();
        assert!(guest.guest);
        assert_eq!(store.find(&name).as_deref(), Ok(name.as_str()), "others can find a guest by name");
        assert_eq!(store.login(&name, "any password at all"), Err(StoreError::WrongCredentials), "a guest has no password");
        assert_eq!(store.change_password(&guest, "", "a long password"), Err(StoreError::GuestsCannot));
        assert_eq!(store.change_username(&guest, "", "Minerva"), Err(StoreError::GuestsCannot));

        // Friends work for a guest as for anyone.
        let other = user(&store, "Minerva");
        store.request_friend(&guest, "Minerva").unwrap();
        store.accept_friend(&other, &name).unwrap();
        assert!(store.are_friends(&other, &name));

        store.logout(&token).unwrap();
        assert_eq!(store.find(&name), Err(StoreError::NoSuchUser), "logging out erases the guest");
        assert_eq!(store.friends(&other).unwrap(), Friends::default());
    }

    #[test]
    fn a_guest_whose_session_ran_out_is_swept_away_and_an_account_is_not() {
        let store = store();
        let (_, name) = store.register_guest(true).unwrap();
        store.register("Minerva", "a long password", true).unwrap();
        store.db().unwrap().execute("UPDATE sessions SET expires_at = 0", []).unwrap();
        // The next session anyone opens does the sweeping.
        store.login("Minerva", "a long password").unwrap();
        assert_eq!(store.find(&name), Err(StoreError::NoSuchUser));
        assert_eq!(store.find("Minerva").as_deref(), Ok("Minerva"));
    }

    #[test]
    fn a_guest_can_erase_itself_without_a_password() {
        let store = store();
        let (token, name) = store.register_guest(true).unwrap();
        let guest = store.user_of(&token).unwrap();
        store.delete_account(&guest, "").unwrap();
        assert_eq!(store.find(&name), Err(StoreError::NoSuchUser));
    }

    #[test]
    fn an_account_unused_for_a_year_is_erased_and_a_used_one_stays() {
        let store = store();
        let old = user(&store, "Minerva");
        let kept = user(&store, "Archie");
        store.request_friend(&old, "Archie").unwrap();
        store
            .db()
            .unwrap()
            .execute("UPDATE users SET last_active_at = last_active_at - ?1 WHERE id = ?2", params![INACTIVE_SECONDS, old.id])
            .unwrap();
        store.sweep().unwrap();
        assert_eq!(store.find("Minerva"), Err(StoreError::NoSuchUser));
        assert_eq!(store.find("Archie").as_deref(), Ok("Archie"));
        assert_eq!(store.friends(&kept).unwrap(), Friends::default(), "the erased account's links go with it");
    }

    #[test]
    fn using_a_session_counts_as_activity() {
        let store = store();
        let token = store.register("Minerva", "a long password", true).unwrap();
        store.db().unwrap().execute("UPDATE users SET last_active_at = 5", []).unwrap();
        let minerva = store.user_of(&token).unwrap();
        assert!(store.export(&minerva).unwrap().last_active_at > 5);
    }

    #[test]
    fn a_changed_privacy_notice_has_to_be_confirmed_again() {
        let store = store().with_notice_version(2);
        let minerva = user(&store, "Minerva");
        assert_eq!(store.accepted_notice(&minerva), Ok(2), "registering confirms the notice shown now");
        let store = store.with_notice_version(3);
        assert_eq!(store.accepted_notice(&minerva), Ok(2));
        store.accept_notice(&minerva).unwrap();
        assert_eq!(store.accepted_notice(&minerva), Ok(3));
        assert_eq!(store.export(&minerva).unwrap().notice_version, 3);
    }

    #[test]
    fn a_session_ends_when_it_has_not_been_used_for_too_long() {
        let store = store();
        let token = store.register("Minerva", "a long password", true).unwrap();
        store.user_of(&token).unwrap();
        store.db().unwrap().execute("UPDATE sessions SET last_used_at = last_used_at - ?1", params![IDLE_SECONDS + 1]).unwrap();
        assert_eq!(store.user_of(&token), Err(StoreError::NotLoggedIn));
    }

    #[test]
    fn a_username_can_be_changed_to_a_free_one_and_friends_stay() {
        let store = store();
        let (a, b) = (user(&store, "Minerva"), user(&store, "Archie"));
        store.request_friend(&a, "Archie").unwrap();
        store.accept_friend(&b, "Minerva").unwrap();
        assert_eq!(store.change_username(&a, "a wrong password", "Athena"), Err(StoreError::WrongCredentials));
        assert_eq!(store.change_username(&a, "a long password", "archie"), Err(StoreError::UsernameTaken));
        store.change_username(&a, "a long password", "Athena").unwrap();
        assert_eq!(store.find("Minerva"), Err(StoreError::NoSuchUser));
        assert_eq!(store.friends(&b).unwrap().friends, vec!["Athena"]);
        store.login("Athena", "a long password").unwrap();
    }

    #[test]
    fn nothing_readable_about_a_user_is_in_the_database() {
        let store = store();
        store.register("Minerva", "a long password", true).unwrap();
        let db = store.db().unwrap();
        let (index, cipher, hash): (Vec<u8>, Vec<u8>, String) = db
            .query_row("SELECT username_index, username_cipher, password_hash FROM users", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        for stored in [&index, &cipher, &hash.clone().into_bytes()] {
            assert!(!stored.windows(7).any(|w| w.eq_ignore_ascii_case(b"minerva")));
            assert!(!stored.windows(15).any(|w| w == b"a long password"));
        }
        assert!(hash.starts_with("$argon2id$"));
        let token_hashes: Vec<Vec<u8>> =
            db.prepare("SELECT token_hash FROM sessions").unwrap().query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
        assert_eq!(token_hashes.len(), 1);
        assert_eq!(token_hashes[0].len(), 32, "a hash of the token, not the token");
    }

    #[test]
    fn friends_need_a_request_and_an_acceptance() {
        let store = store();
        let (a, b) = (user(&store, "Minerva"), user(&store, "Archie"));
        assert_eq!(store.request_friend(&a, "Minerva"), Err(StoreError::Yourself));
        assert_eq!(store.request_friend(&a, "Nobody"), Err(StoreError::NoSuchUser));
        assert_eq!(store.accept_friend(&b, "Minerva"), Err(StoreError::NoRequest));

        store.request_friend(&a, "archie").unwrap();
        assert_eq!(store.friends(&a).unwrap().outgoing, vec!["Archie"]);
        assert_eq!(store.friends(&b).unwrap().incoming, vec!["Minerva"]);
        assert!(!store.are_friends(&a, "Archie"));

        store.accept_friend(&b, "Minerva").unwrap();
        assert!(store.are_friends(&a, "Archie") && store.are_friends(&b, "Minerva"));
        assert!(store.friends(&a).unwrap().outgoing.is_empty());

        store.remove_friend(&b, "Minerva").unwrap();
        assert!(!store.are_friends(&a, "Archie") && !store.are_friends(&b, "Minerva"));
    }

    #[test]
    fn a_user_can_export_everything_held_about_them() {
        let store = store();
        let (a, b) = (user(&store, "Minerva"), user(&store, "Archie"));
        store.request_friend(&a, "Archie").unwrap();
        store.accept_friend(&b, "Minerva").unwrap();
        let export = store.export(&a).unwrap();
        assert_eq!(export.username, "Minerva");
        assert_eq!(export.friends.friends, vec!["Archie"]);
        assert_eq!(export.sessions.len(), 1);
        assert!(export.created_at > 0 && export.notice_acknowledged_at > 0);
    }

    #[test]
    fn deleting_an_account_erases_it_its_sessions_and_its_friend_links() {
        let store = store();
        let token = store.register("Minerva", "a long password", true).unwrap();
        let a = store.user_of(&token).unwrap();
        let b = user(&store, "Archie");
        store.request_friend(&a, "Archie").unwrap();
        store.accept_friend(&b, "Minerva").unwrap();

        assert_eq!(store.delete_account(&a, "a wrong password"), Err(StoreError::WrongCredentials));
        store.delete_account(&a, "a long password").unwrap();
        assert_eq!(store.user_of(&token), Err(StoreError::NotLoggedIn));
        assert_eq!(store.find("Minerva"), Err(StoreError::NoSuchUser));
        assert_eq!(store.friends(&b).unwrap(), Friends::default());
        let rows: i64 = store.db().unwrap().query_row("SELECT (SELECT COUNT(*) FROM sessions WHERE user_id = ?1) + (SELECT COUNT(*) FROM friends WHERE user_id = ?1 OR friend_id = ?1)", params![a.id], |r| r.get(0)).unwrap();
        assert_eq!(rows, 0);
        // The name is free again.
        store.register("Minerva", "another password", true).unwrap();
    }

    #[test]
    fn changing_the_password_needs_the_old_one_and_ends_other_sessions() {
        let store = store();
        let token = store.register("Minerva", "a long password", true).unwrap();
        let a = store.user_of(&token).unwrap();
        assert_eq!(store.change_password(&a, "a wrong password", "a newer password"), Err(StoreError::WrongCredentials));
        assert_eq!(store.change_password(&a, "a long password", "too short"), Err(StoreError::BadPassword));
        store.change_password(&a, "a long password", "a newer password").unwrap();
        assert_eq!(store.user_of(&token), Err(StoreError::NotLoggedIn));
        assert_eq!(store.login("Minerva", "a long password"), Err(StoreError::WrongCredentials));
        store.login("Minerva", "a newer password").unwrap();
    }
}
