//! Owl rules engine.
//!
//! Pure and deterministic: no IO beyond loading content, no clock, and all
//! randomness comes from the lobby's seeded [`rng::LobbyRng`]. Commands go in
//! through [`engine::apply`], events come out.

pub mod api;
pub mod battle;
pub mod content;
pub mod damage;
pub mod endgame;
pub mod engine;
pub mod heroes;
pub mod ids;
pub mod pairing;
pub mod question;
pub mod rng;
pub mod shop;
pub mod state;
pub mod stats;

pub use api::{AbilityAction, Command, DamageSource, Event, Help, RuleError};
pub use content::{Content, ContentError, Effect, Tunables};
pub use engine::{Env, apply};
pub use ids::{CardDefId, CardInstanceId, Category, HeroId, PlayerId, QuestionId};
pub use question::{Difficulty, Question, QuestionSource, SyntheticQuestions};
pub use rng::LobbyRng;
pub use state::{GameState, Phase, Player};
