//! Cards, heroes and tunables, loaded from the `data/` directory.
//!
//! Everything is validated at load time: an unknown effect, category or card
//! reference is an error here, never a surprise in the middle of a match.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{CardDefId, Category, HeroId};

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("cannot parse {path}: {source}")]
    Parse { path: PathBuf, source: Box<toml::de::Error> },
    #[error("invalid content: {0}")]
    Invalid(String),
}

/// Every number that is not on a card or a hero. See `data/tunables.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tunables {
    pub max_tavern_level: u32,
    pub gold_start: u32,
    pub gold_per_turn: u32,
    pub gold_cap: u32,
    pub gold_carries_over: bool,
    pub card_cost: u32,
    pub sell_value: u32,
    pub reroll_cost: u32,
    /// Cost to reach level 2, 3, ... before the per-turn discount.
    pub upgrade_costs: Vec<u32>,
    /// Shop size at tavern level 1, 2, ...
    pub shop_size: Vec<u32>,
    /// Copies of each card in the shared pool, for tier 1, 2, ...
    pub pool_copies: Vec<u32>,
    pub board_slots: u32,
    pub hand_size: u32,
    pub damage_base: u32,
    pub max_battle_damage: u32,
    pub overtime_turn: u32,
    /// The first turn whose regular battle damage is multiplied. See
    /// `Tunables::escalation_percent`.
    pub escalation_turn: u32,
    /// The battle damage multiplier, in percent, for `escalation_turn` and
    /// each turn after it; the last entry repeats.
    pub escalation_percent: Vec<u32>,
    pub endgame_min_turn: u32,
    pub split_threshold: u32,
    pub house_questions: u32,
    pub weighted_questions: u32,
    pub answer_time: u32,
    pub answer_time_bonus_cap: u32,
    pub endgame_answer_time: u32,
    /// The most reflect a player can have, in percent, unless their hero
    /// sets its own cap with a `reflect_cap` param.
    pub reflect_cap: u32,
    pub category_bonus_cards: u32,
    pub category_bonus_damage: u32,
    pub category_bonus_damage_golden: u32,
    pub zero_zero_cap: u32,
    pub golden_reward_choices: u32,
    /// Percent chance that a question's base difficulty is easy, medium, hard.
    pub difficulty_odds: [u32; 3],
    pub potion_chance_base: u32,
    pub potion_chance_per_turn: u32,
    pub skull_healer_weight: u32,
    pub herodotus_shield: u32,
    /// Heroes dealt to each player to pick from in hero select.
    pub hero_offer_size: u32,
    pub hero_select_seconds: u32,
    pub shop_seconds: u32,
    /// Time to read a question, its answers and its category before the
    /// answers open and the answer time starts: the base, plus this much per word of the question
    /// and its four answers, capped. See `Tunables::reading_ms`.
    pub reading_base_ms: u32,
    pub reading_per_word_ms: u32,
    pub reading_max_ms: u32,
    /// Pause on the revealed answer before the next question.
    pub answer_reveal_ms: u32,
    /// How late an answer may arrive after its deadline and still count.
    pub answer_grace_ms: u32,
    /// How far behind the match an eliminated player watches the others.
    pub spectate_delay_ms: u32,
    pub categories: Vec<String>,
}

/// One thing a card does. A closed list: a new kind of effect is a new
/// variant here, its implementation, and a test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "effect")]
pub enum Effect {
    /// Chance that one of the holder's weighted questions is the card's category.
    CategoryWeight { percent: u32 },
    Armor { amount: u32 },
    AnswerTime { seconds: u32 },
    DamageOnWin { amount: u32 },
    Reflect { percent: u32 },
    TieBreaker,
    ZeroZero { damage: u32 },
    GoldOnCorrect { min_correct: u32, gold: u32 },
    /// On a loss, add this card to the holder's hand.
    ShieldOnLoss { card: String },
    /// Activated: remove one wrong answer, this many times per battle.
    RemoveWrong { uses: u32 },
    DamageMultiplier { percent: u32 },
    ExtraDamageOnLoss { amount: u32 },
    ReflectPierce { percent: u32 },
    ArmorPierce { amount: u32 },
    StreakDamage { per_win: u32, max: u32 },
    SplitBonus { points: u32 },
    PerfectDouble,
    EndgameFewerOptions,
    EndgameTime { seconds: u32 },
    EndgameShield,
    Oracle { per_battle: u32, endgame_total: u32 },
    AllCategories,
    DamageCap { amount: u32 },
    HealPerTurn { amount: u32 },
    /// Used from the hand: heal, then the card leaves the game.
    HealOnce { amount: u32 },
    HuntHealer,
    Arrow { damage: u32 },
    Skull,
    /// When the golden version is made, add this card to the hand.
    GrantOnGolden { card: String },
    /// When the golden version is made, heal the owner and everyone else by
    /// a fraction of their starting HP.
    GoldenHeal { owner_div: u32, others_div: u32 },
    /// Moves the difficulty of questions in the holder's battle by `steps`
    /// (positive is harder), for both players. With `all` it touches every
    /// question; without, only those of the holder's top category.
    DifficultyShift { steps: i32, all: bool },
    /// The reward card: placing it opens the golden reward pick, and the
    /// card is gone.
    GoldenReward,
}

impl Effect {
    /// The default golden version: numbers doubled. Effects with no number,
    /// or where doubling makes no sense, are unchanged; a card that needs
    /// something else declares `golden_effects` in its data file.
    pub fn doubled(&self) -> Effect {
        use Effect::*;
        match self.clone() {
            CategoryWeight { percent } => CategoryWeight { percent: percent * 2 },
            Armor { amount } => Armor { amount: amount * 2 },
            AnswerTime { seconds } => AnswerTime { seconds: seconds * 2 },
            DamageOnWin { amount } => DamageOnWin { amount: amount * 2 },
            Reflect { percent } => Reflect { percent: percent * 2 },
            ZeroZero { damage } => ZeroZero { damage: damage * 2 },
            GoldOnCorrect { min_correct, gold } => GoldOnCorrect { min_correct, gold: gold * 2 },
            DamageMultiplier { percent } => DamageMultiplier { percent: percent * 2 },
            ExtraDamageOnLoss { amount } => ExtraDamageOnLoss { amount: amount * 2 },
            ReflectPierce { percent } => ReflectPierce { percent: percent * 2 },
            ArmorPierce { amount } => ArmorPierce { amount: amount * 2 },
            StreakDamage { per_win, max } => StreakDamage { per_win: per_win * 2, max: max * 2 },
            SplitBonus { points } => SplitBonus { points: points * 2 },
            other => other,
        }
    }

    pub fn is_endgame(&self) -> bool {
        matches!(self, Effect::EndgameFewerOptions | Effect::EndgameTime { .. } | Effect::EndgameShield)
    }

    /// Cards with these effects are used from the hand, never placed.
    pub fn is_used_from_hand(&self) -> bool {
        matches!(self, Effect::HealOnce { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Category,
    Armor,
    Time,
    Damage,
    Reflect,
    Tie,
    Economy,
    Defense,
    Help,
    Bet,
    Counter,
    Weights,
    Endgame,
    Legendary,
    Healing,
    Difficulty,
    Token,
}

#[derive(Debug, Clone, Serialize)]
pub struct CardDef {
    pub id: String,
    pub name: String,
    /// 1..=max tavern level, or 0 for cards that never appear in the normal shop.
    pub tier: u32,
    pub cost: u32,
    pub kind: CardKind,
    pub text: String,
    /// What the golden version does, in words. Present exactly when `can_golden`.
    pub golden_text: Option<String>,
    pub category: Option<Category>,
    pub can_golden: bool,
    /// Copies in the shared pool. 0 marks a token: created by effects, never pooled.
    pub copies: u32,
    pub in_shop: bool,
    pub max_on_board: Option<u32>,
    /// Three of this card combine into one of that card.
    pub merges_into: Option<CardDefId>,
    pub effects: Vec<Effect>,
    pub golden_effects: Vec<Effect>,
}

impl CardDef {
    pub fn effects_for(&self, golden: bool) -> &[Effect] {
        if golden { &self.golden_effects } else { &self.effects }
    }

    pub fn is_token(&self) -> bool {
        self.copies == 0
    }

    pub fn has(&self, pred: impl Fn(&Effect) -> bool) -> bool {
        self.effects.iter().any(pred)
    }
}

/// Which per-hero module implements a hero's ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroKind {
    Herodotus,
    Darwin,
    Benz,
    Lumiere,
    Newton,
    Nietzsche,
    Shakespeare,
    Herophilus,
    MarcoPolo,
    Pheidippides,
    Cleopatra,
    MarieCurie,
    /// No ability at all: the temporary placeholder heroes.
    NoAbility,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeroDef {
    pub id: String,
    pub name: String,
    pub title: String,
    pub category: Category,
    pub hp: u32,
    pub category_bonus: u32,
    pub kind: HeroKind,
    params: BTreeMap<String, u32>,
}

impl HeroDef {
    /// A hero number from the data file. Presence is checked at load time.
    pub fn param(&self, key: &str) -> u32 {
        self.params.get(key).copied().unwrap_or(0)
    }

    /// The most reflect this hero's player can have, in percent: the hero's
    /// own `reflect_cap` param, or the tunable that holds for everyone else.
    pub fn reflect_cap(&self, tunables: &Tunables) -> u32 {
        self.params.get(REFLECT_CAP).copied().unwrap_or(tunables.reflect_cap)
    }
}

/// The hero param that overrides the tunable reflect cap.
const REFLECT_CAP: &str = "reflect_cap";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHero {
    id: String,
    name: String,
    title: String,
    category: String,
    hp: u32,
    category_bonus: u32,
    kind: HeroKind,
    #[serde(default)]
    params: BTreeMap<String, u32>,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCard {
    id: String,
    name: String,
    tier: u32,
    cost: Option<u32>,
    kind: CardKind,
    text: String,
    golden_text: Option<String>,
    #[serde(default)]
    per_category: bool,
    #[serde(default = "yes")]
    golden: bool,
    #[serde(default)]
    unique: bool,
    copies: Option<u32>,
    #[serde(default = "yes")]
    shop: bool,
    max_on_board: Option<u32>,
    merges_into: Option<String>,
    #[serde(default)]
    effects: Vec<Effect>,
    golden_effects: Option<Vec<Effect>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Content {
    pub tunables: Tunables,
    pub heroes: Vec<HeroDef>,
    pub cards: Vec<CardDef>,
    by_name: BTreeMap<String, CardDefId>,
    pub potion: CardDefId,
    pub skull: CardDefId,
    /// The reward card that stands for an unpicked golden reward.
    pub golden_reward: CardDefId,
}

impl Content {
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, ContentError> {
        let dir = dir.as_ref();
        let tunables: Tunables = parse(&dir.join("tunables.toml"))?;
        check_tunables(&tunables)?;

        let mut heroes = Vec::new();
        for path in toml_files(&dir.join("heroes"))? {
            let raw: RawHero = parse(&path)?;
            let category = category_of(&tunables, &raw.category)?;
            for key in required_params(raw.kind) {
                if !raw.params.contains_key(*key) {
                    return Err(invalid(format!("hero {} is missing param {key}", raw.id)));
                }
            }
            if raw.params.get(REFLECT_CAP).is_some_and(|&cap| cap > 100) {
                return Err(invalid(format!("hero {}: {REFLECT_CAP} must be at most 100", raw.id)));
            }
            heroes.push(HeroDef {
                id: raw.id,
                name: raw.name,
                title: raw.title,
                category,
                hp: raw.hp,
                category_bonus: raw.category_bonus,
                kind: raw.kind,
                params: raw.params,
            });
        }

        let mut cards = Vec::new();
        let mut merges = Vec::new();
        for path in toml_files(&dir.join("cards"))? {
            let raw: RawCard = parse(&path)?;
            let tier_ok = raw.tier <= tunables.max_tavern_level;
            if !tier_ok || (raw.shop && raw.tier == 0) {
                return Err(invalid(format!("card {} has an invalid tier", raw.id)));
            }
            let copies = match (raw.copies, raw.unique) {
                (Some(n), _) => n,
                (None, true) => 1,
                (None, false) => tunables.pool_copies[(raw.tier.max(1) - 1) as usize],
            };
            // A card that can be golden says what golden does; one that cannot, does not.
            let can_golden = raw.golden && !raw.unique;
            if can_golden != raw.golden_text.is_some() {
                let problem = if can_golden { "can be golden but has no golden_text" } else { "cannot be golden but has a golden_text" };
                return Err(invalid(format!("card {} {problem}", raw.id)));
            }
            let golden_effects = raw
                .golden_effects
                .clone()
                .unwrap_or_else(|| raw.effects.iter().map(Effect::doubled).collect());
            let variants: Vec<Option<Category>> = if raw.per_category {
                (0..tunables.categories.len()).map(|i| Some(Category(i as u8))).collect()
            } else {
                vec![None]
            };
            for category in variants {
                let (id, name) = match category {
                    Some(c) => {
                        let slug = &tunables.categories[usize::from(c.0)];
                        (format!("{}_{slug}", raw.id), format!("{} ({slug})", raw.name))
                    }
                    None => (raw.id.clone(), raw.name.clone()),
                };
                merges.push(raw.merges_into.clone());
                cards.push(CardDef {
                    id,
                    name,
                    tier: raw.tier,
                    cost: raw.cost.unwrap_or(tunables.card_cost),
                    kind: raw.kind,
                    text: raw.text.clone(),
                    golden_text: raw.golden_text.clone(),
                    category,
                    can_golden,
                    copies,
                    in_shop: raw.shop,
                    max_on_board: raw.max_on_board,
                    merges_into: None,
                    effects: raw.effects.clone(),
                    golden_effects: golden_effects.clone(),
                });
            }
        }

        let mut by_name = BTreeMap::new();
        for (i, card) in cards.iter().enumerate() {
            if by_name.insert(card.id.clone(), CardDefId(i as u16)).is_some() {
                return Err(invalid(format!("duplicate card id {}", card.id)));
            }
        }
        let find = |name: &str| {
            by_name.get(name).copied().ok_or_else(|| invalid(format!("unknown card {name}")))
        };
        for (card, target) in cards.iter_mut().zip(merges) {
            if let Some(target) = target {
                card.merges_into = Some(find(&target)?);
            }
        }
        for card in &cards {
            for effect in card.effects.iter().chain(&card.golden_effects) {
                if let Effect::ShieldOnLoss { card } | Effect::GrantOnGolden { card } = effect {
                    find(card)?;
                }
            }
        }
        let potion = find("potion")?;
        let skull = find("skull")?;
        let golden_reward = find("golden_reward")?;
        if heroes.is_empty() {
            return Err(invalid("no heroes".into()));
        }
        Ok(Self { tunables, heroes, cards, by_name, potion, skull, golden_reward })
    }

    pub fn card(&self, id: CardDefId) -> &CardDef {
        &self.cards[usize::from(id.0)]
    }

    pub fn card_id(&self, name: &str) -> Option<CardDefId> {
        self.by_name.get(name).copied()
    }

    pub fn hero(&self, id: HeroId) -> &HeroDef {
        &self.heroes[usize::from(id.0)]
    }

    pub fn hero_id(&self, name: &str) -> Option<HeroId> {
        self.heroes.iter().position(|h| h.id == name).map(|i| HeroId(i as u8))
    }

    pub fn category_count(&self) -> u32 {
        self.tunables.categories.len() as u32
    }
}

fn required_params(kind: HeroKind) -> &'static [&'static str] {
    match kind {
        HeroKind::NoAbility => &[],
        HeroKind::Herodotus | HeroKind::Darwin | HeroKind::Benz => &["uses"],
        HeroKind::Lumiere => &["cost"],
        HeroKind::Newton => &["questions", "removed", "cost", "chance"],
        HeroKind::Nietzsche => &["cost", "options"],
        HeroKind::Shakespeare => &["skull_chance", "skulls", "skip_above", "skips"],
        HeroKind::Herophilus => &["reflect", REFLECT_CAP],
        HeroKind::MarcoPolo => &["cost"],
        HeroKind::Pheidippides => &["per_correct", "max"],
        HeroKind::Cleopatra => &["cap_percent"],
        HeroKind::MarieCurie => &["armor_percent"],
    }
}

fn check_tunables(t: &Tunables) -> Result<(), ContentError> {
    let levels = t.max_tavern_level as usize;
    let ok = levels >= 1
        && t.shop_size.len() == levels
        && t.pool_copies.len() == levels
        && t.upgrade_costs.len() == levels - 1
        && !t.categories.is_empty()
        && t.categories.len() <= usize::from(u8::MAX)
        && t.weighted_questions >= 2
        && t.hero_offer_size >= 1;
    if !ok {
        return Err(invalid("tunables: list lengths do not match max_tavern_level".into()));
    }
    if t.reflect_cap > 100 {
        return Err(invalid("tunables: reflect_cap must be at most 100".into()));
    }
    if t.difficulty_odds.iter().sum::<u32>() != 100 {
        return Err(invalid("tunables: difficulty_odds must add up to 100".into()));
    }
    if t.escalation_percent.is_empty() || t.escalation_percent.iter().any(|&p| p < 100) {
        return Err(invalid("tunables: escalation_percent must list at least one value, each at least 100".into()));
    }
    if t.reading_base_ms > t.reading_max_ms || t.reading_max_ms > MAX_READING_MS {
        return Err(invalid(format!(
            "tunables: reading_base_ms must not exceed reading_max_ms, which must be at most {MAX_READING_MS}"
        )));
    }
    Ok(())
}

impl Tunables {
    /// The regular battle damage multiplier on `turn`, in percent: 100
    /// before `escalation_turn`, then one entry of `escalation_percent` per
    /// turn, the last one repeating.
    pub fn escalation_percent(&self, turn: u32) -> u32 {
        let Some(step) = turn.checked_sub(self.escalation_turn) else { return 100 };
        let last = self.escalation_percent.len().saturating_sub(1);
        self.escalation_percent.get((step as usize).min(last)).copied().unwrap_or(100)
    }

    /// `amount` multiplied by the escalation of `turn`, rounded down, and at
    /// least 1 when `amount` is at least 1.
    pub fn escalate(&self, amount: u32, turn: u32) -> u32 {
        if amount == 0 {
            return 0;
        }
        (amount.saturating_mul(self.escalation_percent(turn)) / 100).max(1)
    }

    /// How long a question of `words` words (its text and four answers
    /// together) is read before its answers open. Both players read the
    /// same question, so both get the same time.
    pub fn reading_ms(&self, words: u32) -> u32 {
        self.reading_per_word_ms.saturating_mul(words).saturating_add(self.reading_base_ms).min(self.reading_max_ms)
    }
}

/// The longest reading time the tunables may set. Anything longer is a typo
/// (seconds written as milliseconds the other way round, say), not a choice.
const MAX_READING_MS: u32 = 30_000;

fn category_of(t: &Tunables, slug: &str) -> Result<Category, ContentError> {
    t.categories
        .iter()
        .position(|c| c == slug)
        .map(|i| Category(i as u8))
        .ok_or_else(|| invalid(format!("unknown category {slug}")))
}

fn invalid(message: String) -> ContentError {
    ContentError::Invalid(message)
}

/// The `.toml` files of a directory, sorted so that ids are stable across machines.
fn toml_files(dir: &Path) -> Result<Vec<PathBuf>, ContentError> {
    let io = |source| ContentError::Io { path: dir.to_path_buf(), source };
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn parse<T: DeserializeOwned>(path: &Path) -> Result<T, ContentError> {
    let text = fs::read_to_string(path)
        .map_err(|source| ContentError::Io { path: path.to_path_buf(), source })?;
    toml::from_str(&text)
        .map_err(|source| ContentError::Parse { path: path.to_path_buf(), source: Box::new(source) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tunables() -> Tunables {
        parse(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/tunables.toml"))).expect("tunables parse")
    }

    #[test]
    fn a_short_question_is_read_for_the_base_and_a_little_per_word() {
        let t = tunables();
        assert_eq!((t.reading_base_ms, t.reading_per_word_ms, t.reading_max_ms), (6000, 150, 10000));
        // "What is the capital of France?" and four one-word answers.
        let words = crate::question::word_count("What is the capital of France?", ["Paris", "Lyon", "Nice", "Lille"]);
        assert_eq!(words, 10);
        assert_eq!(t.reading_ms(words), 6000 + 10 * 150);
        assert_eq!(t.reading_ms(0), 6000);
    }

    #[test]
    fn a_long_question_is_read_for_at_most_the_cap() {
        let t = tunables();
        assert_eq!(t.reading_ms(26), 9900);
        assert_eq!(t.reading_ms(27), 10000);
        assert_eq!(t.reading_ms(200), 10000);
        assert_eq!(t.reading_ms(u32::MAX), 10000);
    }

    #[test]
    fn reading_time_may_be_zero_but_not_absurdly_long() {
        let mut t = tunables();
        (t.reading_base_ms, t.reading_per_word_ms, t.reading_max_ms) = (0, 0, 0);
        assert!(check_tunables(&t).is_ok(), "no reading time is allowed");
        assert_eq!(t.reading_ms(40), 0);
        t.reading_max_ms = MAX_READING_MS;
        assert!(check_tunables(&t).is_ok());
        t.reading_max_ms = MAX_READING_MS + 1;
        assert!(check_tunables(&t).is_err());
        t.reading_max_ms = 5000;
        t.reading_base_ms = 6000;
        assert!(check_tunables(&t).is_err(), "the base cannot exceed the cap");
    }

    #[test]
    fn escalation_factors_must_be_listed_and_at_least_one_hundred_percent() {
        let mut t = tunables();
        assert!(check_tunables(&t).is_ok());
        t.escalation_percent = vec![];
        assert!(check_tunables(&t).is_err(), "at least one factor");
        t.escalation_percent = vec![150, 90];
        assert!(check_tunables(&t).is_err(), "escalation never lowers damage");
        t.escalation_percent = vec![100];
        assert!(check_tunables(&t).is_ok(), "100 turns escalation off");
    }
}
