//! The question bank: questions read from `data/questions/<category>.toml`
//! (format in `docs/questions.md`), and the question source that draws from it.
//!
//! Unreviewed questions are drawn too, so they can be reviewed in play. The
//! engine asks for a difficulty; when a category has no fresh question of
//! it, the nearest difficulty is taken instead. A category with no question
//! left that neither player has seen falls back to the placeholder sums, so
//! a match never runs dry.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use owl_core::question::{OPTIONS, word_count};
use owl_core::{Category, Difficulty, LobbyRng, Question, QuestionId, QuestionSource};
use serde::{Deserialize, Deserializer};

use crate::placeholder::{PlaceholderQuestions, Texts};

/// One question as the bank holds it.
#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub id: String,
    #[serde(default)]
    pub subcategory: String,
    /// `easy`, `medium` or `hard`. A question that does not say is medium.
    #[serde(default = "medium", deserialize_with = "difficulty")]
    pub difficulty: Difficulty,
    pub text: String,
    pub answer: String,
    pub wrong: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub check: String,
    #[serde(default)]
    pub reviewed: bool,
    /// A generated stand-in, to be replaced by a written question.
    #[serde(default)]
    pub placeholder: bool,
}

fn medium() -> Difficulty {
    Difficulty::Medium
}

fn difficulty<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Difficulty, D::Error> {
    match String::deserialize(deserializer)?.as_str() {
        "easy" => Ok(Difficulty::Easy),
        "medium" => Ok(Difficulty::Medium),
        "hard" => Ok(Difficulty::Hard),
        other => Err(serde::de::Error::custom(format!("unknown difficulty {other}"))),
    }
}

/// The difficulties to try for a wanted one, nearest first.
fn nearest(wanted: Difficulty) -> [Difficulty; 3] {
    use Difficulty::{Easy, Hard, Medium};
    match wanted {
        Easy => [Easy, Medium, Hard],
        Medium => [Medium, Easy, Hard],
        Hard => [Hard, Medium, Easy],
    }
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    question: Vec<Entry>,
}

/// Every question, by category index.
#[derive(Debug, Default)]
pub struct QuestionBank {
    by_category: Vec<Vec<Entry>>,
}

/// Bank questions get ids of their own; placeholder sums count up from 1,
/// so they never meet. Ids stay well under 2^53: the client reads JSON
/// numbers as doubles, and a larger id would round onto its neighbours.
const BANK_IDS: u64 = 1_000_000_000;
const PER_CATEGORY: u64 = 1_000_000;

impl QuestionBank {
    /// Reads `dir/questions/*.toml`. Each file is named after a category slug
    /// in `categories`. A missing folder gives an empty bank.
    pub fn load(dir: impl AsRef<Path>, categories: &[String]) -> Result<Self, String> {
        let folder = dir.as_ref().join("questions");
        let mut by_category = vec![Vec::new(); categories.len()];
        let Ok(listing) = fs::read_dir(&folder) else {
            return Ok(Self { by_category });
        };
        let mut paths: Vec<_> = listing.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        let mut ids = BTreeSet::new();
        for path in paths
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        {
            let slug = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let index = categories
                .iter()
                .position(|c| c == slug)
                .ok_or_else(|| format!("{} is not named after a category", path.display()))?;
            let raw = fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let file: File =
                toml::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
            for entry in file.question {
                check(&entry)
                    .map_err(|why| format!("{}: question {}: {why}", path.display(), entry.id))?;
                if !ids.insert(entry.id.clone()) {
                    return Err(format!(
                        "{}: question id {} is used twice",
                        path.display(),
                        entry.id
                    ));
                }
                by_category[index].push(entry);
            }
        }
        Ok(Self { by_category })
    }

    /// How many questions a category holds.
    pub fn count(&self, category: Category) -> usize {
        self.by_category
            .get(usize::from(category.0))
            .map_or(0, Vec::len)
    }

    /// The entry behind a question id, if it came from the bank.
    pub fn entry(&self, id: QuestionId) -> Option<&Entry> {
        let n = id.0.checked_sub(BANK_IDS)?;
        self.by_category
            .get((n / PER_CATEGORY) as usize)?
            .get((n % PER_CATEGORY) as usize)
    }

    fn id(category: Category, index: usize) -> QuestionId {
        QuestionId(BANK_IDS + u64::from(category.0) * PER_CATEGORY + index as u64)
    }
}

fn check(entry: &Entry) -> Result<(), String> {
    if entry.wrong.len() != usize::from(OPTIONS) - 1 {
        return Err(format!("needs exactly {} wrong options", OPTIONS - 1));
    }
    let mut seen = BTreeSet::new();
    for option in std::iter::once(&entry.answer).chain(&entry.wrong) {
        if option.trim().is_empty() || !seen.insert(option.trim().to_lowercase()) {
            return Err("options must be filled in and all different".into());
        }
    }
    if entry.text.trim().is_empty() {
        return Err("the text is empty".into());
    }
    Ok(())
}

/// Draws questions from the bank, and placeholder sums when a category has
/// nothing left.
pub struct BankQuestions {
    bank: Arc<QuestionBank>,
    /// Picks among the questions and orders the wrong options; separate from
    /// the lobby's generator, which only picks the correct option.
    picks: LobbyRng,
    texts: Texts,
    fallback: PlaceholderQuestions,
}

impl BankQuestions {
    pub fn new(bank: Arc<QuestionBank>, seed: u64, texts: Texts) -> Self {
        Self {
            bank,
            picks: LobbyRng::from_seed(seed ^ 0xB4_4E),
            fallback: PlaceholderQuestions::new(seed, texts.clone()),
            texts,
        }
    }
}

impl QuestionSource for BankQuestions {
    fn draw(
        &mut self,
        category: Category,
        difficulty: Difficulty,
        seen_a: &BTreeSet<QuestionId>,
        seen_b: &BTreeSet<QuestionId>,
        rng: &mut LobbyRng,
    ) -> Question {
        let entries = self
            .bank
            .by_category
            .get(usize::from(category.0))
            .map_or(&[][..], Vec::as_slice);
        let fresh: Vec<usize> = (0..entries.len())
            .filter(|&i| {
                let id = QuestionBank::id(category, i);
                !seen_a.contains(&id) && !seen_b.contains(&id)
            })
            .collect();
        // The wanted difficulty if a fresh question of it is left, else the nearest.
        let of_difficulty = nearest(difficulty).into_iter().find_map(|level| {
            let matching: Vec<usize> = fresh.iter().copied().filter(|&i| entries[i].difficulty == level).collect();
            (!matching.is_empty()).then_some(matching)
        });
        let Some(fresh) = of_difficulty else {
            return self.fallback.draw(category, difficulty, seen_a, seen_b, rng);
        };
        let index = fresh[self.picks.below(fresh.len() as u32) as usize];
        let entry = &entries[index];
        let id = QuestionBank::id(category, index);
        let correct = rng.below(u32::from(OPTIONS)) as u8;
        let mut wrong = entry.wrong.clone();
        self.picks.shuffle(&mut wrong);
        let mut wrong = wrong.into_iter();
        let options = (0..OPTIONS)
            .map(|i| {
                if i == correct {
                    entry.answer.clone()
                } else {
                    wrong.next().unwrap_or_default()
                }
            })
            .collect::<Vec<String>>();
        let words = word_count(&entry.text, options.iter().map(String::as_str));
        self.texts.insert(id, (entry.text.clone(), options));
        Question {
            id,
            category,
            difficulty: entry.difficulty,
            correct,
            words,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, difficulty: Difficulty) -> Entry {
        Entry {
            id: id.into(),
            subcategory: String::new(),
            difficulty,
            text: format!("Question {id}?"),
            answer: "Right".into(),
            wrong: vec!["A".into(), "B".into(), "C".into()],
            source: String::new(),
            check: String::new(),
            reviewed: false,
            placeholder: false,
        }
    }

    fn bank() -> QuestionBank {
        QuestionBank {
            by_category: vec![vec![entry("x-1", Difficulty::Easy), entry("x-2", Difficulty::Easy)], vec![]],
        }
    }

    /// Draws from category 0 until the sums take over, and returns the
    /// difficulties of the bank questions in the order they came.
    fn drain(bank: QuestionBank, wanted: Difficulty) -> Vec<Difficulty> {
        let texts = Texts::default();
        let mut source = BankQuestions::new(Arc::new(bank), 3, texts.clone());
        let mut rng = LobbyRng::from_seed(9);
        let mut seen = BTreeSet::new();
        let mut drawn = Vec::new();
        loop {
            let q = source.draw(Category(0), wanted, &seen, &BTreeSet::new(), &mut rng);
            if texts.get(q.id).unwrap().0.starts_with("What is") {
                assert_eq!(q.difficulty, wanted, "a sum is of the wanted difficulty");
                return drawn;
            }
            assert!(seen.insert(q.id), "asked twice");
            drawn.push(q.difficulty);
        }
    }

    #[test]
    fn a_question_of_the_wanted_difficulty_is_drawn_while_one_is_left() {
        use Difficulty::{Easy, Hard, Medium};
        let all = || QuestionBank {
            by_category: vec![vec![
                entry("x-1", Easy),
                entry("x-2", Medium),
                entry("x-3", Hard),
                entry("x-4", Hard),
            ]],
        };
        // Then the nearest: for medium, easy before hard; for easy, medium
        // before hard; for hard, medium before easy. Then the sums.
        assert_eq!(drain(all(), Medium), [Medium, Easy, Hard, Hard]);
        assert_eq!(drain(all(), Easy), [Easy, Medium, Hard, Hard]);
        assert_eq!(drain(all(), Hard), [Hard, Hard, Medium, Easy]);
    }

    #[test]
    fn a_question_without_a_difficulty_is_medium_and_an_unknown_one_is_refused() {
        let question = |extra: &str| {
            format!("id = \"x-1\"\ntext = \"Q?\"\nanswer = \"A\"\nwrong = [\"B\", \"C\", \"D\"]\n{extra}")
        };
        let plain: Entry = toml::from_str(&question("")).unwrap();
        assert_eq!(plain.difficulty, Difficulty::Medium);
        let hard: Entry = toml::from_str(&question("difficulty = \"hard\"")).unwrap();
        assert_eq!(hard.difficulty, Difficulty::Hard);
        assert!(toml::from_str::<Entry>(&question("difficulty = \"fiendish\"")).is_err());
    }

    #[test]
    fn the_answer_sits_at_the_correct_option_and_a_seen_question_is_not_asked_again() {
        let texts = Texts::default();
        let mut source = BankQuestions::new(Arc::new(bank()), 3, texts.clone());
        let mut rng = LobbyRng::from_seed(9);
        let mut seen = BTreeSet::new();
        for _ in 0..2 {
            let q = source.draw(Category(0), Difficulty::Easy, &seen, &BTreeSet::new(), &mut rng);
            let (_, options) = texts.get(q.id).unwrap();
            assert_eq!(options[usize::from(q.correct)], "Right");
            assert!(seen.insert(q.id), "asked twice");
        }
        // Both seen: the sums take over.
        let q = source.draw(Category(0), Difficulty::Easy, &seen, &BTreeSet::new(), &mut rng);
        assert!(texts.get(q.id).unwrap().0.starts_with("What is"));
    }

    #[test]
    fn ids_survive_a_trip_through_a_double() {
        for category in [0u8, 9, 255] {
            for index in [0usize, 1, 999_999] {
                let id = QuestionBank::id(Category(category), index).0;
                assert_eq!(id as f64 as u64, id);
                assert_ne!((id + 1) as f64 as u64, id);
            }
        }
    }

    #[test]
    fn an_empty_category_falls_back_to_sums() {
        let texts = Texts::default();
        let mut source = BankQuestions::new(Arc::new(bank()), 3, texts.clone());
        let q = source.draw(
            Category(1),
            Difficulty::Medium,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &mut LobbyRng::from_seed(1),
        );
        assert!(texts.get(q.id).unwrap().0.starts_with("What is"));
    }

    #[test]
    fn the_shipped_questions_load() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");
        let content = owl_core::Content::load(dir).unwrap();
        QuestionBank::load(dir, &content.tunables.categories).unwrap();
    }

    #[test]
    fn bank_loads_all_categories() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");
        let content = owl_core::Content::load(dir).unwrap();
        let categories = &content.tunables.categories;
        for slug in ["music", "poetry", "prose"] {
            assert!(categories.iter().any(|c| c == slug), "{slug} is a category");
        }
        let bank = QuestionBank::load(dir, categories).unwrap();
        for (i, slug) in categories.iter().enumerate() {
            let entries = &bank.by_category[i];
            // The written categories hold close to a hundred each; the
            // generated ones exactly a hundred.
            assert!(entries.len() >= 90, "{slug} holds {} questions", entries.len());
            if entries.iter().any(|e| e.placeholder) {
                assert_eq!(entries.len(), 100, "{slug}");
                assert!(entries.iter().all(|e| e.placeholder && e.source == "generated" && !e.reviewed));
            }
        }
    }

    #[test]
    fn a_question_without_placeholder_is_not_one() {
        let raw = "id = \"x-1\"\ntext = \"Q?\"\nanswer = \"A\"\nwrong = [\"B\", \"C\", \"D\"]\n";
        assert!(!toml::from_str::<Entry>(raw).unwrap().placeholder);
        let marked = format!("{raw}placeholder = true\n");
        assert!(toml::from_str::<Entry>(&marked).unwrap().placeholder);
    }

    #[test]
    fn a_question_counts_the_words_of_its_text_and_its_four_answers() {
        let texts = Texts::default();
        let mut source = BankQuestions::new(Arc::new(bank()), 3, texts.clone());
        let q = source.draw(Category(0), Difficulty::Easy, &BTreeSet::new(), &BTreeSet::new(), &mut LobbyRng::from_seed(9));
        // "Question x-1?" and "Right", "A", "B", "C".
        assert_eq!(q.words, 6);
        let sum = source.draw(Category(1), Difficulty::Easy, &BTreeSet::new(), &BTreeSet::new(), &mut LobbyRng::from_seed(9));
        // "What is 12 + 34?" and four numbers.
        assert_eq!(sum.words, 9);
    }

    /// Prints the reading times the shipped bank gives, for tuning:
    /// `cargo test -p owl-lobby shipped_reading_times -- --nocapture`.
    #[test]
    fn shipped_reading_times_stay_within_the_tunables() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");
        let content = owl_core::Content::load(dir).unwrap();
        let bank = QuestionBank::load(dir, &content.tunables.categories).unwrap();
        let t = &content.tunables;
        let mut times: Vec<u32> = bank
            .by_category
            .iter()
            .flatten()
            .map(|e| {
                let options = std::iter::once(e.answer.as_str()).chain(e.wrong.iter().map(String::as_str));
                t.reading_ms(word_count(&e.text, options))
            })
            .collect();
        times.sort_unstable();
        assert!(times.iter().all(|&ms| (t.reading_base_ms..=t.reading_max_ms).contains(&ms)));
        if let (Some(min), Some(max)) = (times.first(), times.last()) {
            let capped = times.iter().filter(|&&ms| ms == t.reading_max_ms).count();
            println!(
                "{} questions: reading min {min} ms, median {} ms, max {max} ms, {capped} at the cap",
                times.len(),
                times[times.len() / 2]
            );
        }
    }
}
