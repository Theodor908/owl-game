//! The engine's public vocabulary: commands in, events out, and the errors
//! a rejected command can produce.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{CardDefId, CardInstanceId, Category, HeroId, PlayerId, QuestionId};
use crate::question::Difficulty;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Picks a hero from the player's own offer.
    PickHero { player: PlayerId, hero: HeroId },
    /// Ends hero select. Players who have not picked get a random hero from their offer.
    StartMatch,

    Buy { player: PlayerId, index: usize },
    /// Buys from the extra offers beside the shop (potion, skull).
    BuyExtra { player: PlayerId, index: usize },
    Sell { player: PlayerId, card: CardInstanceId },
    Place { player: PlayerId, card: CardInstanceId },
    /// Uses a card from the hand (potion, skull).
    UseCard { player: PlayerId, card: CardInstanceId },
    Reroll { player: PlayerId },
    ToggleFreeze { player: PlayerId },
    UpgradeTavern { player: PlayerId },
    PickReward { player: PlayerId, index: usize },
    UseAbility { player: PlayerId, action: AbilityAction },
    ChooseEndgameEffect { player: PlayerId, card: CardInstanceId },
    /// Marks the player ready, or not, to end the shop early. Any other shop
    /// command by the player clears it.
    SetReady { player: PlayerId, ready: bool },
    /// The shop deadline passed. Sent by the host.
    EndShop,

    UseHelp { player: PlayerId, help: Help },
    /// `None` is an explicit pass.
    SubmitAnswer { player: PlayerId, choice: Option<u8> },
    /// The answer deadline passed. Sent by the host. `battle` is ignored in the endgame.
    QuestionTimeout { battle: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Help {
    SecondOpinion,
    FlamingSword,
    Oracle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityAction {
    /// Herodotus: rewrite the last battle into a win.
    Rewrite,
    /// Darwin: make a golden card from two copies.
    DarwinMerge { def: CardDefId },
    /// Benz: combine two cards into one slot.
    Combine { a: CardInstanceId, b: CardInstanceId },
    /// Lumiere: replay a film question from the last battle.
    Record { question: QuestionId },
    /// Nietzsche: pay to be shown category options for the house questions.
    Ubermensch,
    /// Nietzsche: pick two of the options shown.
    ChooseHouse { first: Category, second: Category },
    /// Shakespeare.
    ToBeOrNotToBe,
    /// Newton: buy the sword his shop is offering this turn.
    BuySword,
    /// Marco Polo: pay to swap a random shop card for one a tier higher.
    SilkRoad,
    /// Beethoven: one of the payments that unlock Notes in his shop.
    FrenziedNotes,
    /// Goethe: pay to raise his Poetry bonus for the next battle.
    PoetryBoost,
    /// Cervantes: a Knight and a Windmill for the hand.
    DonQuixote,
    /// Homer: one of the three payments that unlock Bards in every shop.
    BardOfTheCenturies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageSource {
    Battle,
    TieBreaker,
    ZeroZero,
    Overtime,
    Reflect,
    Arrow,
    Ability,
    EndgameStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    HeroPicked { player: PlayerId, hero: HeroId },
    TurnStarted { turn: u32 },
    CardBought { player: PlayerId, card: CardInstanceId, def: CardDefId },
    CardSold { player: PlayerId, card: CardInstanceId },
    CardPlaced { player: PlayerId, card: CardInstanceId },
    CardUsed { player: PlayerId, card: CardInstanceId },
    CardGained { player: PlayerId, card: CardInstanceId, def: CardDefId },
    ShopRolled { player: PlayerId },
    TavernUpgraded { player: PlayerId, level: u32 },
    GoldenMade { player: PlayerId, card: CardInstanceId, def: CardDefId },
    CardsCombined { player: PlayerId, card: CardInstanceId },
    RewardOffered { player: PlayerId, choices: Vec<CardDefId> },
    Healed { player: PlayerId, amount: u32 },
    AbilityUsed { player: PlayerId },
    HouseOptionsShown { player: PlayerId, options: Vec<Category> },
    ReadyChanged { player: PlayerId, ready: bool },

    BattleStarted { battle: usize, a: PlayerId, b: Option<PlayerId>, ghost_of: Option<PlayerId> },
    CardDisabled { player: PlayerId, card: CardInstanceId },
    CategoryRevealed { battle: usize, category: Category, difficulty: Difficulty },
    QuestionShown { battle: usize, question: QuestionId, time_a: u32, time_b: u32 },
    OptionsRemoved { player: PlayerId, removed_mask: u8 },
    AnswerRevealed { battle: usize, correct_option: u8, a_correct: bool, b_correct: bool },
    BattleScored { battle: usize, a_correct: u32, b_correct: u32 },
    DamageDealt {
        to: PlayerId,
        source: DamageSource,
        amount: u32,
        armor_absorbed: u32,
        hp_lost: u32,
    },
    ShieldBroken { player: PlayerId, card: CardInstanceId },
    PlayerRevived { player: PlayerId },
    PlayerEliminated { player: PlayerId, placement: u32 },
    EndgameStarted { a: PlayerId, b: PlayerId },
    /// Both finalists won; `MatchFinished` then names the one placed first.
    SharedVictory { a: PlayerId, b: PlayerId },
    MatchFinished { winner: PlayerId },
}

/// Why a command was refused. The state is unchanged when one is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RuleError {
    #[error("not allowed in this phase")]
    WrongPhase,
    #[error("unknown player")]
    UnknownPlayer,
    #[error("player is eliminated")]
    PlayerDead,
    #[error("player can take no more actions this round")]
    LockedOut,
    #[error("that hero is not in your offer")]
    HeroNotOffered,
    #[error("you have already picked a hero")]
    HeroAlreadyPicked,
    #[error("not enough heroes to deal every player an offer")]
    NotEnoughHeroes,
    #[error("no such offer, option or card")]
    BadIndex,
    #[error("not enough gold")]
    NotEnoughGold,
    #[error("hand is full")]
    HandFull,
    #[error("board is full")]
    BoardFull,
    #[error("only a limited number of this card can be on the board")]
    BoardLimit,
    #[error("card not found")]
    CardNotFound,
    #[error("this card cannot be used that way")]
    NotUsable,
    #[error("already used")]
    AlreadyUsed,
    #[error("tavern is already at the highest level")]
    MaxTavern,
    #[error("the hero has no such ability")]
    NoSuchAbility,
    #[error("ability requirements are not met")]
    AbilityUnavailable,
    #[error("no charges left")]
    NoCharges,
    #[error("already used this round; it can be used again next turn")]
    UsedThisRound,
    #[error("player is not in a battle")]
    NotInBattle,
    #[error("answer already locked in")]
    AlreadyAnswered,
    #[error("invalid answer")]
    InvalidAnswer,
}
