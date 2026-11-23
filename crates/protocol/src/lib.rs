//! The messages a client and a host exchange, as JSON.
//!
//! The same shapes travel over the WebSocket online and across the extension
//! boundary offline, so the client above its `Transport` cannot tell which
//! host it is talking to. See `docs/protocol.md`.

use owl_core::{Command, Event};
use owl_lobby::placeholder::Texts;
use owl_lobby::{Outgoing, PlayerView};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Bumped on every change a client could notice. The server refuses a
/// client that says another number.
pub const VERSION: u32 = 5;

/// Events for one player, each with the text of a question it shows. The
/// correct answer is never part of it.
pub fn events_json(outgoing: Vec<Outgoing>, texts: &Texts) -> Vec<Value> {
    outgoing
        .into_iter()
        .map(|item| {
            let mut value = serde_json::to_value(&item).unwrap_or(Value::Null);
            if let Event::QuestionShown { question, .. } = item.event
                && let Some((text, options)) = texts.get(question)
            {
                value["question"] = json!({ "text": text, "options": options });
            }
            value
        })
        .collect()
}

/// Everything one player may see, with the text of any question that is
/// open: their own, and those of the battles they are watching.
pub fn view_json(view: &PlayerView, texts: &Texts) -> Value {
    let mut value = serde_json::to_value(view).unwrap_or(Value::Null);
    let open = view.battle.as_ref().map(|b| b.question);
    if let Some((text, options)) = open.and_then(|q| texts.get(q)) {
        value["battle"]["question_text"] = json!({ "text": text, "options": options });
    }
    for (index, watched) in view.spectate.iter().enumerate() {
        let question = watched.battle.as_ref().map(|b| b.question);
        if let Some((text, options)) = question.and_then(|q| texts.get(q)) {
            value["spectate"][index]["battle"]["question_text"] = json!({ "text": text, "options": options });
        }
    }
    value
}

/// What a client sends over the WebSocket.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// The first message on every connection: protocol version and session token.
    Hello { v: u32, token: String },
    /// Opens a lobby with the sender as its owner.
    CreateLobby,
    /// The owner invites a friend who is online.
    Invite { username: String },
    /// Joins a lobby by its code: one the sender was invited to, or one whose
    /// owner told them the code.
    JoinLobby { code: String },
    /// The owner puts a bot in an empty seat, or takes one out again.
    AddBot,
    RemoveBot,
    /// Turns an invitation down, so the lobby stops waiting for the sender.
    DeclineInvite { code: String },
    LeaveLobby,
    /// The owner starts the match with the players and bots of the lobby.
    /// `fill_with_bots` gives every seat that is still empty to a bot.
    StartMatch { fill_with_bots: bool },
    /// A game command. `id` comes back in `Accepted` or `Rejected`.
    Command { id: u64, command: Command },
    /// Asks for a fresh snapshot, after a gap in `seq`.
    Sync,
}

/// What the server sends over the WebSocket.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// The session is good. `content` is the cards, heroes and tunables.
    Welcome { username: String, content: Value, now: u64 },
    /// The lobby the player is in, whenever it changes.
    LobbyState { code: String, owner: String, members: Vec<String>, invited: Vec<String>, bots: usize, max: usize },
    /// A friend invites the player to their lobby.
    Invited { from: String, code: String },
    /// The player is no longer in a lobby: it closed, or they left.
    LobbyClosed,
    /// The match begins. `seats[i]` is the name in seat `i`; bots have names too.
    MatchStarted { seats: Vec<String>, you: u8 },
    /// The player's whole view. Sent at the start, on reconnect and after `Sync`.
    Snapshot { view: Value, now: u64 },
    /// What happened since the last message, in order, and the view after it.
    Events { events: Vec<Value>, view: Value, now: u64 },
    Accepted { id: u64 },
    Rejected { id: u64, reason: String },
    /// Something the client asked for outside a match was refused.
    Error { reason: String },
}

/// One message on the wire: `{"v":1,"seq":7,"type":"Events","payload":{...}}`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Envelope<M> {
    pub v: u32,
    /// Counts the sender's messages, so the other side can see a gap.
    pub seq: u64,
    #[serde(flatten)]
    pub message: M,
}

impl<M> Envelope<M> {
    pub fn new(seq: u64, message: M) -> Self {
        Self { v: VERSION, seq, message }
    }
}
