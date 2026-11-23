//! Limits that keep one client from using up the server.
//!
//! Everything a stranger can ask for without being logged in costs the
//! server something: a password hash takes memory and a whole core, and a
//! new player is a row on disk. These limits make a flood wait or be
//! refused instead of starving the Pi, which also hosts other things.
//!
//! - Wrong passwords for one name make each further try wait longer
//!   (`Backoff`, used by the store, keyed by the name's blind index).
//! - Each client address has a few password tries and a few new players
//!   per window (`Window`), and a few open game connections (`Connections`).
//! - Every password hash goes through one gate (`HashGate`) with a short
//!   queue; past it a request is refused at once.
//!
//! Client addresses are kept only here, in memory, for at most the window
//! they count towards. They are never written to disk or to a log.
//!
//! Every map here has a fixed largest size. When one is full, expired
//! entries go first, then the entry with the fewest hits: losing it can only
//! help that client, while the busiest clients stay counted.

use std::collections::HashMap;
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// ---- Wrong passwords for one name -------------------------------------------

/// Wrong passwords for one name before each further try has to wait.
pub const FREE_FAILURES: u32 = 3;
/// The first wait, after `FREE_FAILURES` wrong passwords. Each further
/// wrong password doubles it.
pub const FIRST_WAIT: Duration = Duration::from_secs(1);
/// The longest wait between tries. Waits grow instead of locking the name,
/// so nobody can lock a friend out for good by guessing.
pub const MAX_WAIT: Duration = Duration::from_secs(5 * 60);
/// Wrong passwords older than this are forgotten.
pub const FORGET_FAILURES: Duration = Duration::from_secs(30 * 60);
/// The most names whose wrong passwords are remembered at once.
pub const MAX_NAMES: usize = 10_000;

// ---- Per client address -------------------------------------------------------

/// Password checks one address may ask for per window: logging in, and
/// changing the password or the name or deleting the account.
pub const PASSWORD_TRIES_PER_ADDRESS: u32 = 20;
pub const PASSWORD_TRIES_WINDOW: Duration = Duration::from_secs(10 * 60);
/// New accounts and guests one address may make per window.
pub const NEW_PLAYERS_PER_ADDRESS: u32 = 5;
/// New accounts and guests the whole server makes per window, a safety cap
/// above any one address's share.
pub const NEW_PLAYERS_TOTAL: u32 = 60;
pub const NEW_PLAYERS_WINDOW: Duration = Duration::from_secs(60 * 60);
/// Game connections open at once from one address.
pub const SOCKETS_PER_ADDRESS: usize = 10;
/// Game connections open at once in all.
pub const SOCKETS_TOTAL: usize = 300;
/// The most addresses each limit remembers at once.
pub const MAX_ADDRESSES: usize = 10_000;
/// IPv6 addresses count by their first 64 bits: one home or phone network
/// gets a whole /64 and could otherwise pick a new address for each try.
pub const IPV6_PREFIX_BITS: u32 = 64;

// ---- Password hashing -------------------------------------------------------

/// Password hashes running at once. Each takes 19 MiB and a core; the Pi has
/// four cores and runs other things too.
pub const HASHES_AT_ONCE: usize = 2;
/// Requests that may wait for a hash. Past this one is refused at once.
pub const HASH_QUEUE: usize = 8;
/// What a refused request is told to wait before it tries again.
pub const BUSY_RETRY: Duration = Duration::from_secs(2);

/// The setting that says the server sits behind a Cloudflare tunnel, so the
/// client's address is in the `CF-Connecting-IP` header.
pub const TRUST_CF_IP_SETTING: &str = "OWL_TRUST_CF_IP";
/// The header Cloudflare puts the client's address in.
pub const CF_CONNECTING_IP: &str = "cf-connecting-ip";

/// Where the limits get the time from: the system, or a clock that tests move.
#[derive(Clone)]
pub struct Clock(Option<Arc<Mutex<Instant>>>);

impl Clock {
    pub fn system() -> Self {
        Self(None)
    }

    /// A clock that stands still until `advance` moves it.
    pub fn manual() -> Self {
        Self(Some(Arc::new(Mutex::new(Instant::now()))))
    }

    pub fn now(&self) -> Instant {
        match &self.0 {
            None => Instant::now(),
            Some(at) => at.lock().map_or_else(|poisoned| *poisoned.into_inner(), |at| *at),
        }
    }

    /// Moves a manual clock forward. Does nothing to the system clock.
    pub fn advance(&self, by: Duration) {
        if let Some(at) = &self.0
            && let Ok(mut at) = at.lock()
        {
            *at += by;
        }
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::system()
    }
}

/// Removes the entry `rank` puts lowest, if `map` is at `capacity`.
fn make_room<K: Clone + Hash + Eq, V, R: Ord>(map: &mut HashMap<K, V>, capacity: usize, rank: impl Fn(&V) -> R) {
    if map.len() < capacity {
        return;
    }
    if let Some(key) = map.iter().min_by_key(|(_, value)| rank(value)).map(|(key, _)| key.clone()) {
        map.remove(&key);
    }
}

/// Wrong passwords per key, each further try waiting twice as long as the last.
pub struct Backoff<K> {
    /// How many wrong passwords, and when the last one was.
    entries: HashMap<K, (u32, Instant)>,
}

impl<K> Default for Backoff<K> {
    fn default() -> Self {
        Self { entries: HashMap::new() }
    }
}

/// How long the next try must wait after `count` wrong passwords.
pub fn wait_after(count: u32) -> Duration {
    if count < FREE_FAILURES {
        return Duration::ZERO;
    }
    FIRST_WAIT.saturating_mul(1 << (count - FREE_FAILURES).min(16)).min(MAX_WAIT)
}

impl<K: Clone + Hash + Eq> Backoff<K> {
    /// `Err` with the time left if `key` has to wait before its next try.
    pub fn check(&mut self, key: &K, now: Instant) -> Result<(), Duration> {
        self.entries.retain(|_, (_, last)| now < *last + FORGET_FAILURES);
        match self.entries.get(key) {
            Some(&(count, last)) if now < last + wait_after(count) => Err(last + wait_after(count) - now),
            _ => Ok(()),
        }
    }

    pub fn fail(&mut self, key: K, now: Instant) {
        if !self.entries.contains_key(&key) {
            make_room(&mut self.entries, MAX_NAMES, |&(count, last)| (count, last));
        }
        let entry = self.entries.entry(key).or_insert((0, now));
        *entry = (entry.0 + 1, now);
    }

    /// The right password: the count starts again.
    pub fn clear(&mut self, key: &K) {
        self.entries.remove(key);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// At most `limit` hits per key in each window of `length`, the window
/// starting at a key's first hit.
pub struct Window<K> {
    limit: u32,
    length: Duration,
    capacity: usize,
    /// When each key's window began, and its hits in it.
    entries: HashMap<K, (Instant, u32)>,
}

impl<K: Clone + Hash + Eq> Window<K> {
    pub fn new(limit: u32, length: Duration, capacity: usize) -> Self {
        Self { limit, length, capacity, entries: HashMap::new() }
    }

    /// Counts one hit for `key`. `Err` with the time until its window ends
    /// if the window has had its share; a refused hit is not counted.
    pub fn hit(&mut self, key: K, now: Instant) -> Result<(), Duration> {
        let length = self.length;
        if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity {
            self.entries.retain(|_, (start, _)| now < *start + length);
            make_room(&mut self.entries, self.capacity, |&(_, hits)| hits);
        }
        let entry = self.entries.entry(key).or_insert((now, 0));
        if now >= entry.0 + length {
            *entry = (now, 0);
        }
        if entry.1 >= self.limit {
            return Err(entry.0 + length - now);
        }
        entry.1 += 1;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Open game connections, per address and in all.
pub struct Connections {
    per_address: usize,
    total: usize,
    open: Mutex<HashMap<IpAddr, usize>>,
}

/// One open connection. Dropping it frees its place.
pub struct ConnectionPlace {
    connections: Arc<Connections>,
    address: IpAddr,
}

impl Connections {
    pub fn new(per_address: usize, total: usize) -> Arc<Self> {
        Arc::new(Self { per_address, total, open: Mutex::new(HashMap::new()) })
    }

    /// A place for one more connection from `address`, if it has one left.
    pub fn open(self: &Arc<Self>, address: IpAddr) -> Option<ConnectionPlace> {
        let mut open = self.open.lock().ok()?;
        let total: usize = open.values().sum();
        let mine = open.get(&address).copied().unwrap_or(0);
        if mine >= self.per_address || total >= self.total {
            return None;
        }
        open.insert(address, mine + 1);
        Some(ConnectionPlace { connections: self.clone(), address })
    }
}

impl Drop for ConnectionPlace {
    fn drop(&mut self) {
        if let Ok(mut open) = self.connections.open.lock()
            && let Some(count) = open.get_mut(&self.address)
        {
            *count -= 1;
            if *count == 0 {
                open.remove(&self.address);
            }
        }
    }
}

/// The server is hashing as much as it allows and the queue is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

/// At most a few password hashes at once, and a short queue for the rest.
pub struct HashGate {
    permits: Arc<Semaphore>,
    waiting: AtomicUsize,
    queue: usize,
}

/// A place in the queue, given up when the wait ends either way.
struct InQueue<'a>(&'a AtomicUsize);

impl Drop for InQueue<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl HashGate {
    pub fn new(at_once: usize, queue: usize) -> Self {
        Self { permits: Arc::new(Semaphore::new(at_once)), waiting: AtomicUsize::new(0), queue }
    }

    /// Waits for a turn to hash. `Busy` at once if the queue is full. The
    /// turn lasts as long as the permit: move it into the work that hashes.
    pub async fn enter(&self) -> Result<OwnedSemaphorePermit, Busy> {
        if let Ok(permit) = self.permits.clone().try_acquire_owned() {
            return Ok(permit);
        }
        let ahead = self.waiting.fetch_add(1, Ordering::SeqCst);
        let _place = InQueue(&self.waiting);
        if ahead >= self.queue {
            return Err(Busy);
        }
        self.permits.clone().acquire_owned().await.map_err(|_| Busy)
    }

    /// Requests waiting for a turn now.
    pub fn waiting(&self) -> usize {
        self.waiting.load(Ordering::SeqCst)
    }
}

/// The key an address counts under: IPv4 as it is (also when written as
/// IPv6), IPv6 by its first `IPV6_PREFIX_BITS`.
pub fn address_key(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let mask = u128::MAX << (128 - IPV6_PREFIX_BITS);
                IpAddr::V6(Ipv6Addr::from(u128::from(v6) & mask))
            }
        },
    }
}

/// The address a request came from. With `trust_cf_ip` that is the one in
/// `CF-Connecting-IP` when it holds one; otherwise, and without the setting,
/// the address of the connection itself. Unknown when neither is there.
pub fn client_address(headers: &HeaderMap, peer: Option<SocketAddr>, trust_cf_ip: bool) -> IpAddr {
    let from_cloudflare = || {
        headers.get(CF_CONNECTING_IP).and_then(|value| value.to_str().ok()).and_then(|value| value.trim().parse().ok())
    };
    let address = if trust_cf_ip { from_cloudflare() } else { None };
    address_key(address.or(peer.map(|peer| peer.ip())).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)))
}

/// A refusal and how long to wait before trying again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// Too many password checks from this address.
    PasswordTries(Duration),
    /// Too many new players from this address.
    NewPlayersHere(Duration),
    /// Too many new players across the server.
    NewPlayersEverywhere(Duration),
}

/// Every limit the server keeps, with the clock and settings they use.
pub struct Limits {
    pub clock: Clock,
    /// The server sits behind Cloudflare and takes the client's address
    /// from `CF-Connecting-IP`.
    pub trust_cf_ip: bool,
    pub hashes: HashGate,
    pub sockets: Arc<Connections>,
    password_tries: Mutex<Window<IpAddr>>,
    new_players: Mutex<Window<IpAddr>>,
    new_players_total: Mutex<Window<()>>,
}

impl Limits {
    pub fn new(clock: Clock, trust_cf_ip: bool) -> Self {
        Self {
            clock,
            trust_cf_ip,
            hashes: HashGate::new(HASHES_AT_ONCE, HASH_QUEUE),
            sockets: Connections::new(SOCKETS_PER_ADDRESS, SOCKETS_TOTAL),
            password_tries: Mutex::new(Window::new(PASSWORD_TRIES_PER_ADDRESS, PASSWORD_TRIES_WINDOW, MAX_ADDRESSES)),
            new_players: Mutex::new(Window::new(NEW_PLAYERS_PER_ADDRESS, NEW_PLAYERS_WINDOW, MAX_ADDRESSES)),
            new_players_total: Mutex::new(Window::new(NEW_PLAYERS_TOTAL, NEW_PLAYERS_WINDOW, 1)),
        }
    }

    pub fn client_address(&self, headers: &HeaderMap, peer: Option<SocketAddr>) -> IpAddr {
        client_address(headers, peer, self.trust_cf_ip)
    }

    /// Counts one password check from `address`.
    pub fn password_try(&self, address: IpAddr) -> Result<(), Refused> {
        let now = self.clock.now();
        let mut tries = self.password_tries.lock().map_err(|_| Refused::PasswordTries(BUSY_RETRY))?;
        tries.hit(address, now).map_err(Refused::PasswordTries)
    }

    /// Counts one new account or guest from `address`.
    pub fn new_player(&self, address: IpAddr) -> Result<(), Refused> {
        let now = self.clock.now();
        let mut here = self.new_players.lock().map_err(|_| Refused::NewPlayersEverywhere(BUSY_RETRY))?;
        here.hit(address, now).map_err(Refused::NewPlayersHere)?;
        let mut total = self.new_players_total.lock().map_err(|_| Refused::NewPlayersEverywhere(BUSY_RETRY))?;
        total.hit((), now).map_err(Refused::NewPlayersEverywhere)
    }
}

/// Whole seconds to wait, never 0: what `Retry-After` says.
pub fn retry_seconds(wait: Duration) -> u64 {
    (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1)
}

/// A wait in plain words: "8 seconds", "4 minutes".
pub fn wait_text(seconds: u64) -> String {
    match seconds {
        1 => "1 second".into(),
        0..120 => format!("{seconds} seconds"),
        _ => format!("{} minutes", seconds.div_ceil(60)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wait_after_wrong_passwords_doubles_and_stops_growing() {
        assert_eq!((wait_after(0), wait_after(FREE_FAILURES - 1)), (Duration::ZERO, Duration::ZERO));
        let secs = |count| wait_after(count).as_secs();
        assert_eq!((secs(FREE_FAILURES), secs(FREE_FAILURES + 1), secs(FREE_FAILURES + 2)), (1, 2, 4));
        assert_eq!(wait_after(500), MAX_WAIT, "a name is never locked for good");
    }

    #[test]
    fn a_window_counts_hits_until_it_ends_and_never_grows_past_its_size() {
        let start = Instant::now();
        let mut window = Window::new(2, Duration::from_secs(10), 3);
        assert_eq!((window.hit("a", start), window.hit("a", start)), (Ok(()), Ok(())));
        assert_eq!(window.hit("a", start + Duration::from_secs(4)), Err(Duration::from_secs(6)));
        assert_eq!(window.hit("a", start + Duration::from_secs(10)), Ok(()), "a new window");
        window.hit("a", start + Duration::from_secs(10)).unwrap();
        for key in ["b", "c", "d", "e"] {
            window.hit(key, start + Duration::from_secs(10)).unwrap();
        }
        assert_eq!(window.len(), 3, "the map stays at its size");
        assert_eq!(window.hit("a", start + Duration::from_secs(11)), Err(Duration::from_secs(9)), "the busiest key is kept");
    }

    #[test]
    fn a_backoff_forgets_old_failures_and_never_grows_past_its_size() {
        let start = Instant::now();
        let mut backoff = Backoff::default();
        for _ in 0..FREE_FAILURES {
            backoff.fail(1, start);
        }
        assert_eq!(backoff.check(&1, start), Err(FIRST_WAIT));
        assert_eq!(backoff.check(&1, start + FIRST_WAIT), Ok(()));
        assert_eq!(backoff.check(&2, start), Ok(()));
        backoff.check(&1, start + FORGET_FAILURES).unwrap();
        assert!(backoff.is_empty(), "forgotten after a while");
        for key in 0..=MAX_NAMES {
            backoff.fail(key, start);
        }
        assert_eq!(backoff.len(), MAX_NAMES);
    }

    #[test]
    fn ipv6_addresses_count_by_their_network() {
        let one: IpAddr = "2001:db8:1:2:aaaa::1".parse().unwrap();
        let two: IpAddr = "2001:db8:1:2:bbbb::9".parse().unwrap();
        let other: IpAddr = "2001:db8:1:3::1".parse().unwrap();
        assert_eq!(address_key(one), address_key(two));
        assert_ne!(address_key(one), address_key(other));
        let mapped: IpAddr = "::ffff:192.0.2.7".parse().unwrap();
        assert_eq!(address_key(mapped), "192.0.2.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn waits_are_told_in_whole_seconds_and_plain_words() {
        assert_eq!(retry_seconds(Duration::from_millis(1500)), 2);
        assert_eq!(retry_seconds(Duration::ZERO), 1);
        assert_eq!(retry_seconds(Duration::from_secs(3)), 3);
        assert_eq!((wait_text(1), wait_text(8), wait_text(300)), ("1 second".into(), "8 seconds".into(), "5 minutes".into()));
    }

    #[test]
    fn connections_are_capped_per_address_and_freed_when_dropped() {
        let connections = Connections::new(2, 3);
        let (a, b): (IpAddr, IpAddr) = ("192.0.2.1".parse().unwrap(), "192.0.2.2".parse().unwrap());
        let first = connections.open(a).unwrap();
        let _second = connections.open(a).unwrap();
        assert!(connections.open(a).is_none());
        let _third = connections.open(b).unwrap();
        assert!(connections.open(b).is_none(), "the total is reached");
        drop(first);
        assert!(connections.open(a).is_some());
    }
}
