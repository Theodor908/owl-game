//! Who is online, the lobbies they gather in, and the matches those run.
//!
//! One `Hub` holds the connected players and the open lobbies behind a
//! lock. Each running match is its own task that owns its `owl_lobby::Lobby`
//! and nothing else, so a match never touches another match's state. The
//! server holds no rules: a match task forwards commands to the lobby and
//! sends each player what the lobby says they may see.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use owl_core::{Command, Content, LobbyRng, PlayerId};
use owl_lobby::bot::{Bot, KNOWLEDGE, STRATEGIES};
use owl_lobby::bank::{BankQuestions, QuestionBank};
use owl_lobby::placeholder::Texts;
use owl_lobby::{Lobby, Seat};
use owl_protocol::{ServerMessage, events_json, view_json};
use tokio::sync::{Mutex, mpsc};

/// The most players in one lobby.
pub const MAX_PLAYERS: usize = 6;
/// How often a match looks at its clock.
const TICK: Duration = Duration::from_millis(50);
/// How long a player whose connection dropped keeps their place in a lobby
/// that has not started. Fixed in the server, not a game rule: phones drop
/// their connection soon after the game goes to the background.
pub const LOBBY_GRACE: Duration = Duration::from_secs(3 * 60);
/// How often players away for longer than `LOBBY_GRACE` are let go.
pub const AWAY_SWEEP_EVERY: Duration = Duration::from_secs(10);
/// Names for bots that fill empty seats.
const BOT_NAMES: [&str; 6] = ["Minerva", "Archie", "Tawny", "Sage", "Strix", "Quill"];

pub type Outbox = mpsc::UnboundedSender<ServerMessage>;

/// A connected player.
struct Client {
    name: String,
    outbox: Outbox,
    /// Tells this connection apart from an older one of the same player.
    connection: u64,
    /// The code of the lobby the player is in.
    lobby: Option<String>,
}

/// What a match task is asked to do.
pub enum GameInput {
    Command { seat: PlayerId, id: u64, command: Command },
    /// Send this seat its whole view again.
    Sync { seat: PlayerId },
    /// The seat's connection dropped: it is no longer ready to end the
    /// shop. Sent to the match as `SetReady` on their behalf; a refusal is ignored.
    Dropped { seat: PlayerId },
}

struct Room {
    /// Lower-cased name of the player who opened the lobby.
    owner: String,
    /// Display names, in seat order.
    members: Vec<String>,
    invited: Vec<String>,
    /// Seats the owner gave to bots.
    bots: usize,
    /// The running match and the names in its seats, bots included.
    game: Option<(mpsc::UnboundedSender<GameInput>, Vec<String>)>,
}

#[derive(Default)]
pub struct Hub {
    /// By lower-cased username.
    online: HashMap<String, Client>,
    /// By lobby code.
    rooms: HashMap<String, Room>,
    /// Lobby members whose connection dropped, by lower-cased username, and
    /// since when. Kept in memory only.
    away: HashMap<String, Instant>,
    connections: u64,
}

fn key(name: &str) -> String {
    name.to_lowercase()
}

fn new_code() -> String {
    // No letters or digits that are easy to mix up when read aloud.
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 6];
    rand::fill(&mut bytes);
    bytes.iter().map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char).collect()
}

impl Hub {
    fn send(&self, name: &str, message: ServerMessage) {
        if let Some(client) = self.online.get(&key(name)) {
            let _ = client.outbox.send(message);
        }
    }

    fn room_state(&self, code: &str) -> Option<ServerMessage> {
        let room = self.rooms.get(code)?;
        let owner = room.members.iter().find(|m| key(m) == room.owner).cloned().unwrap_or_default();
        Some(ServerMessage::LobbyState {
            code: code.to_string(),
            owner,
            members: room.members.clone(),
            invited: room.invited.clone(),
            bots: room.bots,
            max: MAX_PLAYERS,
            away: room.members.iter().filter(|m| self.away.contains_key(&key(m))).cloned().collect(),
        })
    }

    fn broadcast_room(&self, code: &str) {
        let (Some(state), Some(room)) = (self.room_state(code), self.rooms.get(code)) else { return };
        for member in &room.members {
            self.send(member, state.clone());
        }
    }

    /// The code of the lobby that has a player among its members.
    fn room_of(&self, name: &str) -> Option<String> {
        self.rooms.iter().find(|(_, room)| room.members.iter().any(|m| key(m) == key(name))).map(|(code, _)| code.clone())
    }

    /// A player connected. An older connection of the same player is dropped.
    /// A player who was away is back in their lobby, or their seat.
    /// Returns the number of this connection.
    pub fn connect(&mut self, name: &str, outbox: Outbox) -> u64 {
        self.connections += 1;
        self.away.remove(&key(name));
        let lobby = self.online.get(&key(name)).and_then(|old| old.lobby.clone()).or_else(|| self.room_of(name));
        self.online.insert(key(name), Client { name: name.to_string(), outbox, connection: self.connections, lobby });
        self.connections
    }

    /// Sends a returning player where they are: their lobby and, if a match is on, their view.
    /// The others in a lobby that has not started see that they are back.
    pub fn resume(&self, name: &str) {
        let Some(code) = self.online.get(&key(name)).and_then(|c| c.lobby.clone()) else { return };
        let Some(room) = self.rooms.get(&code) else { return };
        if room.game.is_none() {
            self.broadcast_room(&code);
        } else if let Some(state) = self.room_state(&code) {
            self.send(name, state);
        }
        if let Some((game, seats)) = &room.game
            && let Some(seat) = seats.iter().position(|s| key(s) == key(name))
        {
            self.send(name, ServerMessage::MatchStarted { seats: seats.clone(), you: seat as u8 });
            let _ = game.send(GameInput::Sync { seat: PlayerId(seat as u8) });
        }
    }

    /// A connection closed. The player is away: their place in a lobby is
    /// kept for `LOBBY_GRACE` (see `sweep_away`), and their seat in a
    /// running match for as long as it runs. In a match they are no longer
    /// ready to end the shop.
    pub fn disconnect(&mut self, name: &str, connection: u64) {
        if self.online.get(&key(name)).is_none_or(|c| c.connection != connection) {
            return;
        }
        if let Some((game, seat)) = self.seat_of(name) {
            let _ = game.send(GameInput::Dropped { seat });
        }
        let Some(code) = self.online.remove(&key(name)).and_then(|c| c.lobby) else { return };
        let Some(room) = self.rooms.get(&code) else { return };
        self.away.insert(key(name), Instant::now());
        if room.game.is_none() {
            self.broadcast_room(&code);
        }
    }

    /// Lets go of every player away for longer than `LOBBY_GRACE`: they
    /// leave their lobby as if they had left it themselves. A seat in a
    /// running match is kept until the match ends.
    pub fn sweep_away(&mut self, now: Instant) {
        let due: Vec<String> = self
            .away
            .iter()
            .filter(|(_, since)| now.saturating_duration_since(**since) >= LOBBY_GRACE)
            .map(|(name, _)| name.clone())
            .collect();
        for name in due {
            match self.room_of(&name) {
                Some(code) if self.rooms.get(&code).is_some_and(|room| room.game.is_some()) => {}
                Some(code) => self.drop_member(&code, &name),
                None => {
                    self.away.remove(&name);
                }
            }
        }
    }

    /// A lobby's match ended at `now`; the lobby stays open for another.
    /// Members still away get the full grace from now, as if they had
    /// dropped from the lobby just then, and everyone sees who is away.
    pub fn match_ended(&mut self, code: &str, now: Instant) {
        let Some(room) = self.rooms.get_mut(code) else { return };
        room.game = None;
        for member in &room.members {
            if let Some(since) = self.away.get_mut(&key(member)) {
                *since = now;
            }
        }
        self.broadcast_room(code);
    }

    /// Drops a player's connection and their place anywhere: after logout or account deletion.
    pub fn remove(&mut self, name: &str) {
        self.leave(name);
        self.online.remove(&key(name));
        if self.away.remove(&key(name)).is_some()
            && let Some(code) = self.room_of(name)
        {
            self.drop_member(&code, name);
        }
    }

    pub fn is_online(&self, name: &str) -> bool {
        self.online.contains_key(&key(name))
    }

    pub fn create(&mut self, name: &str) -> Result<(), &'static str> {
        let me = self.online.get(&key(name)).ok_or("you are not connected")?;
        if me.lobby.is_some() {
            return Err("you are already in a lobby");
        }
        let code = loop {
            let code = new_code();
            if !self.rooms.contains_key(&code) {
                break code;
            }
        };
        let display = me.name.clone();
        self.rooms.insert(code.clone(), Room { owner: key(name), members: vec![display], invited: Vec::new(), bots: 0, game: None });
        if let Some(client) = self.online.get_mut(&key(name)) {
            client.lobby = Some(code.clone());
        }
        self.broadcast_room(&code);
        Ok(())
    }

    /// The owner invites a friend. Whether they are friends is the caller's to check.
    pub fn invite(&mut self, name: &str, friend: &str) -> Result<(), &'static str> {
        let code = self.online.get(&key(name)).and_then(|c| c.lobby.clone()).ok_or("you are not in a lobby")?;
        let target = self.online.get(&key(friend)).map(|c| c.name.clone()).ok_or("that player is not online")?;
        let room = self.rooms.get_mut(&code).ok_or("that lobby is gone")?;
        if room.owner != key(name) {
            return Err("only the owner of the lobby invites");
        }
        if room.game.is_some() {
            return Err("the match has already started");
        }
        if room.members.iter().any(|m| key(m) == key(friend)) {
            return Err("that player is already in the lobby");
        }
        if room.members.len() + room.invited.len() + room.bots >= MAX_PLAYERS
            && !room.invited.iter().any(|i| key(i) == key(friend))
        {
            return Err("the lobby is full");
        }
        if !room.invited.iter().any(|i| key(i) == key(friend)) {
            room.invited.push(target.clone());
        }
        let from = self.online.get(&key(name)).map(|c| c.name.clone()).unwrap_or_default();
        self.send(&target, ServerMessage::Invited { from, code: code.clone() });
        self.broadcast_room(&code);
        Ok(())
    }

    pub fn join(&mut self, name: &str, code: &str) -> Result<(), &'static str> {
        let code = code.to_uppercase();
        let me = self.online.get(&key(name)).ok_or("you are not connected")?;
        if me.lobby.is_some() {
            return Err("you are already in a lobby");
        }
        let display = me.name.clone();
        // The code is the way in: an invitation carries it, and the owner
        // can also tell it to someone who has no account to be invited by.
        let room = self.rooms.get_mut(&code).ok_or("there is no lobby with that code")?;
        if room.game.is_some() {
            return Err("the match has already started");
        }
        if room.members.len() + room.bots >= MAX_PLAYERS {
            return Err("the lobby is full");
        }
        room.invited.retain(|invited| key(invited) != key(name));
        room.members.push(display);
        if let Some(client) = self.online.get_mut(&key(name)) {
            client.lobby = Some(code.clone());
        }
        self.broadcast_room(&code);
        Ok(())
    }

    /// The owner changes the number of bots in the lobby by one, up or down.
    pub fn change_bots(&mut self, name: &str, add: bool) -> Result<(), &'static str> {
        let code = self.online.get(&key(name)).and_then(|c| c.lobby.clone()).ok_or("you are not in a lobby")?;
        let room = self.rooms.get_mut(&code).ok_or("that lobby is gone")?;
        if room.owner != key(name) {
            return Err("only the owner of the lobby changes its bots");
        }
        if room.game.is_some() {
            return Err("the match has already started");
        }
        if add {
            if room.members.len() + room.invited.len() + room.bots >= MAX_PLAYERS {
                return Err("the lobby is full");
            }
            room.bots += 1;
        } else {
            if room.bots == 0 {
                return Err("there is no bot to remove");
            }
            room.bots -= 1;
        }
        self.broadcast_room(&code);
        Ok(())
    }

    /// Turns an invitation down: the player is no longer listed as invited.
    pub fn decline(&mut self, name: &str, code: &str) {
        let code = code.to_uppercase();
        let Some(room) = self.rooms.get_mut(&code) else { return };
        let before = room.invited.len();
        room.invited.retain(|invited| key(invited) != key(name));
        if room.invited.len() != before {
            self.broadcast_room(&code);
        }
    }

    /// Leaves the lobby. When the owner leaves, the lobby closes for everyone.
    pub fn leave(&mut self, name: &str) {
        let Some(code) = self.online.get_mut(&key(name)).and_then(|c| c.lobby.take()) else { return };
        self.send(name, ServerMessage::LobbyClosed);
        self.drop_member(&code, name);
    }

    /// Takes a player out of a lobby's members. The lobby closes for
    /// everyone when they were its owner or its last member.
    fn drop_member(&mut self, code: &str, name: &str) {
        self.away.remove(&key(name));
        let Some(room) = self.rooms.get_mut(code) else { return };
        room.members.retain(|m| key(m) != key(name));
        let closes = room.owner == key(name) || room.members.is_empty();
        if !closes {
            self.broadcast_room(code);
            return;
        }
        // Dropping the room drops the match's inbox, which ends the match task.
        let Some(room) = self.rooms.remove(code) else { return };
        for member in room.members {
            self.away.remove(&key(&member));
            if let Some(client) = self.online.get_mut(&key(&member)) {
                client.lobby = None;
                let _ = client.outbox.send(ServerMessage::LobbyClosed);
            }
        }
    }

    /// The match a player is seated in: its inbox and their seat.
    pub fn seat_of(&self, name: &str) -> Option<(mpsc::UnboundedSender<GameInput>, PlayerId)> {
        let code = self.online.get(&key(name))?.lobby.as_ref()?;
        let (game, seats) = self.rooms.get(code)?.game.as_ref()?;
        let seat = seats.iter().position(|s| key(s) == key(name))?;
        Some((game.clone(), PlayerId(seat as u8)))
    }
}

/// The owner starts the match of their lobby. Empty seats go to bots if asked.
pub async fn start_match(
    hub: &Arc<Mutex<Hub>>,
    content: &Arc<Content>,
    questions: &Arc<QuestionBank>,
    name: &str,
    fill_with_bots: bool,
) -> Result<(), &'static str> {
    let mut guard = hub.lock().await;
    let code = guard.online.get(&key(name)).and_then(|c| c.lobby.clone()).ok_or("you are not in a lobby")?;
    let room = guard.rooms.get_mut(&code).ok_or("that lobby is gone")?;
    if room.owner != key(name) {
        return Err("only the owner of the lobby starts the match");
    }
    if room.game.is_some() {
        return Err("the match has already started");
    }
    let humans = room.members.len();
    let total = if fill_with_bots { MAX_PLAYERS } else { (humans + room.bots).min(MAX_PLAYERS) };
    if total < 2 {
        return Err("a match needs at least two players; invite a friend or add a bot");
    }

    let mut seed_bytes = [0u8; 8];
    rand::fill(&mut seed_bytes);
    let seed = u64::from_le_bytes(seed_bytes);
    let mut setup = LobbyRng::from_seed(seed ^ 0x5EA7);
    let mut names = room.members.clone();
    let mut seats: Vec<Seat> = (0..humans).map(|_| Seat::Human).collect();
    for seat in humans..total {
        let knowledge = KNOWLEDGE[setup.below(KNOWLEDGE.len() as u32) as usize];
        let strategy = STRATEGIES[setup.below(STRATEGIES.len() as u32) as usize];
        seats.push(Seat::Bot(Box::new(Bot::new(PlayerId(seat as u8), knowledge, strategy, content, setup.next_u64()))));
        let free = BOT_NAMES.iter().find(|bot| !names.iter().any(|n| key(n).starts_with(&key(bot))));
        names.push(format!("{} (bot)", free.copied().unwrap_or("Owl")));
    }
    let texts = Texts::default();
    let questions = BankQuestions::new(questions.clone(), seed, texts.clone());
    let lobby = Lobby::new(content.clone(), seats, seed, Box::new(questions), 0)
        .map_err(|_| "there are not enough heroes for that many players")?;

    let (inbox, inputs) = mpsc::unbounded_channel();
    room.game = Some((inbox, names.clone()));
    for (seat, member) in names.iter().take(humans).enumerate() {
        guard.send(member, ServerMessage::MatchStarted { seats: names.clone(), you: seat as u8 });
    }
    drop(guard);
    tokio::spawn(run_match(hub.clone(), code, lobby, texts, names, humans, inputs));
    Ok(())
}

/// One match, from its first tick to its last event.
async fn run_match(
    hub: Arc<Mutex<Hub>>,
    code: String,
    mut lobby: Lobby,
    texts: Texts,
    names: Vec<String>,
    humans: usize,
    mut inputs: mpsc::UnboundedReceiver<GameInput>,
) {
    let started = Instant::now();
    let now = || started.elapsed().as_millis() as u64;
    let mut clock = tokio::time::interval(TICK);
    // Everyone starts from a full view.
    let mut wants_snapshot: Vec<bool> = vec![true; humans];
    loop {
        let mut replies: Vec<(usize, ServerMessage)> = Vec::new();
        tokio::select! {
            input = inputs.recv() => match input {
                // The lobby closed: the match ends with it.
                None => return,
                Some(GameInput::Sync { seat }) => {
                    if let Some(flag) = wants_snapshot.get_mut(seat.index()) {
                        *flag = true;
                    }
                }
                Some(GameInput::Dropped { seat }) => {
                    // Refused outside the shop, where there is nothing to take back.
                    let _ = lobby.handle(now(), seat, Command::SetReady { player: seat, ready: false });
                }
                Some(GameInput::Command { seat, id, command }) => {
                    let reply = match lobby.handle(now(), seat, command) {
                        Ok(()) => ServerMessage::Accepted { id },
                        Err(error) => ServerMessage::Rejected { id, reason: error.to_string() },
                    };
                    replies.push((seat.index(), reply));
                }
            },
            _ = clock.tick() => lobby.tick(now()),
        }

        let at = now();
        for seat in 0..humans {
            let player = PlayerId(seat as u8);
            let events = events_json(lobby.drain(player), &texts);
            if std::mem::take(&mut wants_snapshot[seat]) {
                replies.push((seat, ServerMessage::Snapshot { view: view_json(&lobby.view(player), &texts), now: at }));
            } else if !events.is_empty() || replies.iter().any(|(to, _)| *to == seat) {
                replies.push((seat, ServerMessage::Events { events, view: view_json(&lobby.view(player), &texts), now: at }));
            }
        }
        if !replies.is_empty() {
            let guard = hub.lock().await;
            for (seat, message) in replies {
                guard.send(&names[seat], message);
            }
        }
        if lobby.is_finished() {
            // The lobby stays open so the same friends can play again.
            hub.lock().await.match_ended(&code, Instant::now());
            return;
        }
    }
}
