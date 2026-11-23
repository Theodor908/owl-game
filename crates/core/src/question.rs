//! What the engine needs to know about a question, and where questions come from.
//!
//! The engine never sees question text. It only needs the id, the category
//! and which of the four options is correct.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ids::{Category, QuestionId};
use crate::rng::LobbyRng;

/// Every question is multiple choice with this many options.
pub const OPTIONS: u8 = 4;

/// How hard a question is. Regular battles roll a base difficulty for each
/// question and cards move it; see `battle::wanted_difficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    /// Moves the difficulty by `steps` (positive is harder), stopping at
    /// easy and at hard.
    pub fn shifted(self, steps: i32) -> Difficulty {
        let last = Self::ALL.len() as i32 - 1;
        Self::ALL[(self as i32).saturating_add(steps).clamp(0, last) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub id: QuestionId,
    pub category: Category,
    /// The difficulty of the question that was drawn. A source that has
    /// nothing left of the wanted difficulty gives the nearest one.
    pub difficulty: Difficulty,
    pub correct: u8,
}

/// A question bank. The prototype reads files; the real game uses a database.
pub trait QuestionSource {
    /// Draws a question of `category` that is in neither `seen` set, of the
    /// wanted `difficulty` if the source has one left.
    fn draw(
        &mut self,
        category: Category,
        difficulty: Difficulty,
        seen_a: &BTreeSet<QuestionId>,
        seen_b: &BTreeSet<QuestionId>,
        rng: &mut LobbyRng,
    ) -> Question;
}

/// An endless supply of placeholder questions, for tests and the simulator.
#[derive(Debug, Default)]
pub struct SyntheticQuestions {
    next: u64,
}

impl QuestionSource for SyntheticQuestions {
    fn draw(
        &mut self,
        category: Category,
        difficulty: Difficulty,
        _seen_a: &BTreeSet<QuestionId>,
        _seen_b: &BTreeSet<QuestionId>,
        rng: &mut LobbyRng,
    ) -> Question {
        self.next += 1;
        Question { id: QuestionId(self.next), category, difficulty, correct: rng.below(u32::from(OPTIONS)) as u8 }
    }
}
