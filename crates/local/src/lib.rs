//! The offline host: a Godot extension that runs one lobby inside the app,
//! one human against bots, using the same `lobby` and `core` as the server.
//!
//! Everything crosses the boundary as JSON text in the same shapes the online
//! protocol will carry, so the client code above `Transport` is identical
//! online and offline. No rule lives here.

// The class derive expands to field initialisers clippy would shorten.
#![allow(clippy::redundant_field_names)]

use std::sync::Arc;

use godot::prelude::*;
use owl_core::{Command, Content, LobbyRng, PlayerId};
use owl_lobby::bot::{Bot, KNOWLEDGE, STRATEGIES};
use owl_lobby::{Lobby, Seat};
use serde_json::Value;

use owl_lobby::bank::{BankQuestions, QuestionBank};
use owl_lobby::placeholder::Texts;

struct OwlLocal;

#[gdextension]
unsafe impl ExtensionLibrary for OwlLocal {}

/// The seat of the human player in an offline lobby.
const HUMAN: PlayerId = PlayerId(0);
const SEATS: u8 = 6;

/// One offline lobby. Create it, load the content, start a match, then call
/// `tick` every frame and `drain` to read what happened.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct LocalLobby {
    content: Option<Arc<Content>>,
    questions: Arc<QuestionBank>,
    lobby: Option<Lobby>,
    texts: Texts,
    base: Base<RefCounted>,
}

#[godot_api]
impl LocalLobby {
    /// Loads cards, heroes and tunables from a directory on disk.
    /// Returns an empty string on success, or the reason it failed.
    #[func]
    fn load_content(&mut self, dir: GString) -> GString {
        match Content::load(dir.to_string()) {
            Ok(content) => match QuestionBank::load(dir.to_string(), &content.tunables.categories) {
                Ok(questions) => {
                    self.questions = Arc::new(questions);
                    self.content = Some(Arc::new(content));
                    GString::new()
                }
                Err(error) => GString::from(&error),
            },
            Err(error) => GString::from(&error.to_string()),
        }
    }

    /// Cards, heroes, categories and tunables as JSON, for display.
    #[func]
    fn content_json(&self) -> GString {
        let json = self.content.as_ref().and_then(|c| serde_json::to_string(c.as_ref()).ok());
        GString::from(&json.unwrap_or_default())
    }

    /// Starts a lobby of one human and five bots. Returns false if the
    /// content has not been loaded or has too few heroes for the seats.
    #[func]
    fn start(&mut self, seed: i64, now_ms: i64) -> bool {
        let Some(content) = self.content.clone() else { return false };
        let seed = seed as u64;
        let mut setup = LobbyRng::from_seed(seed ^ 0x5EA7);
        let mut seats = vec![Seat::Human];
        for seat in 1..SEATS {
            let knowledge = KNOWLEDGE[setup.below(KNOWLEDGE.len() as u32) as usize];
            let strategy = STRATEGIES[setup.below(STRATEGIES.len() as u32) as usize];
            let bot = Bot::new(PlayerId(seat), knowledge, strategy, &content, setup.next_u64());
            seats.push(Seat::Bot(Box::new(bot)));
        }
        self.texts = Texts::default();
        let questions = BankQuestions::new(self.questions.clone(), seed, self.texts.clone());
        self.lobby = Lobby::new(content, seats, seed, Box::new(questions), now_ms as u64).ok();
        self.lobby.is_some()
    }

    /// Sends one command from the human player, as JSON.
    /// Returns an empty string if it was accepted, or the reason it was not.
    #[func]
    fn send(&mut self, now_ms: i64, command_json: GString) -> GString {
        let Some(lobby) = self.lobby.as_mut() else { return GString::from("no lobby is running") };
        let command: Command = match serde_json::from_str(&command_json.to_string()) {
            Ok(command) => command,
            Err(error) => return GString::from(&format!("bad command: {error}")),
        };
        match lobby.handle(now_ms as u64, HUMAN, command) {
            Ok(()) => GString::new(),
            Err(error) => GString::from(&error.to_string()),
        }
    }

    /// Advances the lobby to the given time: bots act, deadlines fire.
    #[func]
    fn tick(&mut self, now_ms: i64) {
        if let Some(lobby) = self.lobby.as_mut() {
            lobby.tick(now_ms as u64);
        }
    }

    /// Events for the human player since the last call, as a JSON array.
    /// A shown question carries its text and options, never its answer.
    #[func]
    fn drain(&mut self) -> GString {
        let Some(lobby) = self.lobby.as_mut() else { return GString::from("[]") };
        let items = owl_protocol::events_json(lobby.drain(HUMAN), &self.texts);
        GString::from(&Value::Array(items).to_string())
    }

    /// Everything the human player may see right now, as JSON.
    #[func]
    fn view(&self) -> GString {
        let Some(lobby) = self.lobby.as_ref() else { return GString::from("null") };
        let view = owl_protocol::view_json(&lobby.view(HUMAN), &self.texts);
        GString::from(&view.to_string())
    }

    #[func]
    fn is_finished(&self) -> bool {
        self.lobby.as_ref().is_some_and(Lobby::is_finished)
    }
}

/// Checks that a downloaded pack or library was published by the game's
/// makers. The copy of this class inside the installed game is the one the
/// loader asks, before anything downloaded is used (`client/scripts/boot.gd`).
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct ReleaseCheck {
    base: Base<RefCounted>,
}

#[godot_api]
impl ReleaseCheck {
    /// True if `entry_json`, an entry of the server's `version.json`, is
    /// signed with the release key. The entry names the file's SHA-256 and
    /// size; comparing those with the file is the caller's part.
    #[func]
    fn signed(&self, entry_json: GString) -> bool {
        owl_release::signed_by_release_key(&entry_json.to_string())
    }
}
