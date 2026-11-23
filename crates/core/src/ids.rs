//! Identifier newtypes, so a player index can never be passed where a card id is expected.

use serde::{Deserialize, Serialize};

macro_rules! id {
    ($(#[$doc:meta])* $name:ident($inner:ty)) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub $inner);
    };
}

id!(/// A seat in the lobby; also the index into `GameState::players`.
    PlayerId(u8));
id!(/// A card type, for example Whetstone. Index into `Content::cards`.
    CardDefId(u16));
id!(/// One physical card in one lobby.
    CardInstanceId(u32));
id!(/// A question in the question bank.
    QuestionId(u64));
id!(/// A broad question category. Index into `Content::categories`.
    Category(u8));
id!(/// A hero. Index into `Content::heroes`.
    HeroId(u8));

impl PlayerId {
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}
