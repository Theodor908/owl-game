//! Placeholder questions: sums, asked when the question bank (`bank`) has
//! nothing left in a category. Used by both hosts.
//!
//! Trivia must never be made up, so a category that runs dry asks sums:
//! they are correct by construction. The category shown is only a label.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use owl_core::question::{OPTIONS, word_count};
use owl_core::{Category, Difficulty, LobbyRng, Question, QuestionId, QuestionSource};

type Text = (String, Vec<String>);

/// Question text by id, shared between the question source inside the lobby
/// and the host that hands text to the clients.
#[derive(Clone, Default)]
pub struct Texts(Arc<Mutex<BTreeMap<QuestionId, Text>>>);

impl Texts {
    pub fn get(&self, id: QuestionId) -> Option<Text> {
        self.0.lock().ok()?.get(&id).cloned()
    }

    pub(crate) fn insert(&self, id: QuestionId, text: Text) {
        if let Ok(mut map) = self.0.lock() {
            map.insert(id, text);
        }
    }
}

pub struct PlaceholderQuestions {
    next: u64,
    /// Chooses the numbers. Separate from the lobby's generator, which only
    /// picks the correct option, exactly as the simulator's source does.
    numbers: LobbyRng,
    texts: Texts,
}

impl PlaceholderQuestions {
    pub fn new(seed: u64, texts: Texts) -> Self {
        Self { next: 0, numbers: LobbyRng::from_seed(seed ^ 0x51E5), texts }
    }
}

impl QuestionSource for PlaceholderQuestions {
    fn draw(
        &mut self,
        category: Category,
        difficulty: Difficulty,
        _seen_a: &BTreeSet<QuestionId>,
        _seen_b: &BTreeSet<QuestionId>,
        rng: &mut LobbyRng,
    ) -> Question {
        self.next += 1;
        let id = QuestionId(self.next);
        let correct = rng.below(u32::from(OPTIONS)) as u8;

        let a = 12 + self.numbers.below(78);
        let b = 12 + self.numbers.below(78);
        let answer = a + b;
        // Wrong options close enough to need the sum, and all different.
        let mut offsets = [-10i64, -2, -1, 1, 2, 10];
        self.numbers.shuffle(&mut offsets);
        let mut wrong = offsets.iter().map(|d| i64::from(answer) + d);
        let options: Vec<String> = (0..OPTIONS)
            .map(|i| if i == correct { i64::from(answer) } else { wrong.next().unwrap_or(0) })
            .map(|value| value.to_string())
            .collect();
        let text = format!("What is {a} + {b}?");
        let words = word_count(&text, options.iter().map(String::as_str));
        self.texts.insert(id, (text, options));
        Question { id, category, difficulty, correct, words }
    }
}
