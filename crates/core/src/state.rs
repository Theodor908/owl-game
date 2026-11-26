//! The whole state of one lobby, as plain serializable data.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::api::RuleError;
use crate::content::{Content, Effect};
use crate::heroes;
use crate::ids::{CardDefId, CardInstanceId, Category, HeroId, PlayerId, QuestionId};
use crate::question::Question;
use crate::rng::LobbyRng;
use crate::stats::BattleStats;

/// One card definition inside a card instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPart {
    pub def: CardDefId,
    pub golden: bool,
    /// Pool copies this part was made from; that many return to the pool.
    pub copies: u32,
    /// A golden part whose reward has not been offered yet: it is offered
    /// the first time the card is on the board.
    #[serde(default)]
    pub reward_due: bool,
}

/// One physical card. A plain card has one part; a Benz combined card has several.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardInstance {
    pub id: CardInstanceId,
    pub parts: Vec<CardPart>,
    pub armor_left: u32,
    /// Disabled for the current battle by a skull.
    pub disabled: bool,
}

impl CardInstance {
    /// True for a single, non-golden copy of `def`: the only kind that merges.
    pub fn is_plain(&self, def: CardDefId) -> bool {
        matches!(&self.parts[..], [part] if part.def == def && !part.golden)
    }

    pub fn has_def(&self, def: CardDefId) -> bool {
        self.parts.iter().any(|p| p.def == def)
    }

    pub fn effects<'a>(&'a self, content: &'a Content) -> impl Iterator<Item = &'a Effect> + 'a {
        self.parts.iter().flat_map(move |p| content.card(p.def).effects_for(p.golden).iter())
    }

    pub fn has_effect(&self, content: &Content, pred: impl Fn(&Effect) -> bool) -> bool {
        self.effects(content).any(pred)
    }

    /// True when any part of the card leaves the board after one battle.
    pub fn lasts_one_battle(&self, content: &Content) -> bool {
        self.parts.iter().any(|p| content.card(p.def).one_battle)
    }
}

/// Per-hero ability bookkeeping. Each hero module uses the fields it needs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroState {
    pub uses: u32,
    /// Newton: the turn in which his shop last offered the sword; 0 before the first offer.
    pub sword_offer_turn: u32,
    /// The turn in which a once-a-turn ability was last used; 0 before the first use.
    pub used_turn: u32,
    pub house_options: Vec<Category>,
    pub house_choice: Option<(Category, Category)>,
    pub recorded: Option<Question>,
    /// The hero's current stage, moved on by cards that advance it; 0 before the first.
    #[serde(default)]
    pub stage: u32,
    /// A one-battle boost bought with an ability, cleared when the battle ends.
    #[serde(default)]
    pub boosted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastBattle {
    /// `None` when the opponent was a ghost.
    pub opponent: Option<PlayerId>,
    pub hp_lost: u32,
    pub questions: Vec<Question>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub hero: Option<HeroId>,
    /// The heroes this player may pick from, in offer order. Private to the player.
    pub hero_offer: Vec<HeroId>,
    pub hp: u32,
    pub max_hp: u32,
    /// Armor that is not on a card and lasts for one battle.
    pub temp_armor: u32,
    pub gold: u32,
    pub tavern: u32,
    pub upgrade_discount: u32,
    pub hand: Vec<CardInstance>,
    pub board: Vec<CardInstance>,
    pub shop: Vec<CardDefId>,
    /// Offers shown beside the shop: potion, skull.
    pub extras: Vec<CardDefId>,
    pub frozen: bool,
    /// Pending golden rewards, oldest first.
    pub rewards: Vec<Vec<CardDefId>>,
    pub alive: bool,
    pub placement: Option<u32>,
    pub win_streak: u32,
    pub bonus_gold: u32,
    pub asked: u32,
    pub correct: u32,
    pub seen: BTreeSet<QuestionId>,
    pub last_ghost_turn: u32,
    pub hero_state: HeroState,
    pub acted_this_shop: bool,
    pub locked_out: bool,
    /// Ready to end the shop early. Cleared by any other shop command and at the start of a turn.
    #[serde(default)]
    pub ready: bool,
    pub skip_battle: bool,
    pub endgame_choice: Option<CardInstanceId>,
    pub heal_blocked: bool,
    pub healer_disabled_turn: Option<u32>,
    pub taken_this_battle: u32,
    pub last_battle: Option<LastBattle>,
}

impl Player {
    fn new(id: PlayerId, hero_offer: Vec<HeroId>) -> Self {
        Self {
            id,
            hero: None,
            hero_offer,
            hp: 0,
            max_hp: 0,
            temp_armor: 0,
            gold: 0,
            tavern: 1,
            upgrade_discount: 0,
            hand: Vec::new(),
            board: Vec::new(),
            shop: Vec::new(),
            extras: Vec::new(),
            frozen: false,
            rewards: Vec::new(),
            alive: true,
            placement: None,
            win_streak: 0,
            bonus_gold: 0,
            asked: 0,
            correct: 0,
            seen: BTreeSet::new(),
            last_ghost_turn: 0,
            hero_state: HeroState::default(),
            acted_this_shop: false,
            locked_out: false,
            ready: false,
            skip_battle: false,
            endgame_choice: None,
            heal_blocked: false,
            healer_disabled_turn: None,
            taken_this_battle: 0,
            last_battle: None,
        }
    }

    /// Armor that currently protects the player. Disabled cards do not count.
    pub fn armor(&self) -> u32 {
        self.temp_armor + self.board.iter().filter(|c| !c.disabled).map(|c| c.armor_left).sum::<u32>()
    }

    /// All of the player's cards, board first.
    pub fn cards(&self) -> impl Iterator<Item = &CardInstance> {
        self.board.iter().chain(&self.hand)
    }

    pub fn find_card(&self, id: CardInstanceId) -> Option<&CardInstance> {
        self.cards().find(|c| c.id == id)
    }

    /// Removes a card from the hand or the board.
    pub fn take_card(&mut self, id: CardInstanceId) -> Option<CardInstance> {
        if let Some(i) = self.board.iter().position(|c| c.id == id) {
            return Some(self.board.remove(i));
        }
        let i = self.hand.iter().position(|c| c.id == id)?;
        Some(self.hand.remove(i))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    HeroSelect,
    Shop,
    Battle,
    Endgame,
    Finished,
}

/// One side of a battle: a player, or a ghost standing in for one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fighter {
    /// `None` for a ghost.
    pub player: Option<PlayerId>,
    pub ghost_of: Option<PlayerId>,
    /// Correct answers and questions asked of the ghost's source so far.
    pub ghost_accuracy: (u32, u32),
    pub stats: BattleStats,
    pub answers: Vec<bool>,
    /// Outer `None`: not locked in yet. Inner `None`: no answer given.
    pub locked: Option<Option<u8>>,
    /// Bit mask of options removed for this fighter on the current question.
    pub removed: u8,
    pub second_opinion_left: u32,
    pub sword_left: u32,
    pub oracle_left: u32,
}

impl Fighter {
    pub fn correct_count(&self) -> u32 {
        self.answers.iter().filter(|&&a| a).count() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSlot {
    pub question: Question,
    /// Which fighter's weights chose the category; `None` for a house question.
    pub owner: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleState {
    pub fighters: [Fighter; 2],
    pub plan: Vec<PlanSlot>,
    pub current: usize,
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndgameEffect {
    FewerOptions,
    Time(u32),
    Shield,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndgameState {
    pub players: [PlayerId; 2],
    /// Damage dealt by the current question. Starts at 1.
    pub step: u32,
    pub question: Option<Question>,
    pub locked: [Option<Option<u8>>; 2],
    pub removed: [u8; 2],
    pub effects: [Option<EndgameEffect>; 2],
    pub shield_used: [bool; 2],
    pub oracle_left: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub players: Vec<Player>,
    pub phase: Phase,
    pub turn: u32,
    /// Copies of each card left in the shared pool, indexed by `CardDefId`.
    pub pool: Vec<u32>,
    pub next_instance: u32,
    pub battles: Vec<BattleState>,
    pub endgame: Option<EndgameState>,
    /// `last_met[a][b]`: the turn players a and b last fought, 0 if never.
    pub last_met: Vec<Vec<u32>>,
    pub golden_healer_made: bool,
    /// Both endgame players died on the same question with equal HP: both
    /// won, and the placements only order them on the podium.
    pub shared_win: bool,
}

impl GameState {
    /// A new lobby in hero select, with every player's hero offer dealt.
    /// Fails if there are too few heroes to give each player a full offer.
    pub fn new(players: usize, content: &Content, rng: &mut LobbyRng) -> Result<Self, RuleError> {
        let offers = heroes::deal_offers(players, content, rng)?;
        Ok(Self {
            players: offers.into_iter().enumerate().map(|(i, offer)| Player::new(PlayerId(i as u8), offer)).collect(),
            phase: Phase::HeroSelect,
            turn: 0,
            pool: content.cards.iter().map(|c| c.copies).collect(),
            next_instance: 0,
            battles: Vec::new(),
            endgame: None,
            last_met: vec![vec![0; players]; players],
            golden_healer_made: false,
            shared_win: false,
        })
    }

    pub fn alive(&self) -> impl Iterator<Item = usize> + '_ {
        self.players.iter().enumerate().filter(|(_, p)| p.alive).map(|(i, _)| i)
    }

    pub fn alive_count(&self) -> usize {
        self.alive().count()
    }

    /// Creates a card instance. Taking the copies out of the pool is the caller's job.
    pub fn new_instance(&mut self, def: CardDefId, golden: bool, copies: u32, content: &Content) -> CardInstance {
        let id = CardInstanceId(self.next_instance);
        self.next_instance += 1;
        let armor_left = content
            .card(def)
            .effects_for(golden)
            .iter()
            .map(|e| if let Effect::Armor { amount } = e { *amount } else { 0 })
            .sum();
        CardInstance { id, parts: vec![CardPart { def, golden, copies, reward_due: false }], armor_left, disabled: false }
    }

    /// Puts one copy of a card definition back in the shared pool. Tokens are not pooled.
    pub fn return_def(&mut self, def: CardDefId, copies: u32, content: &Content) {
        if !content.card(def).is_token() {
            self.pool[usize::from(def.0)] += copies;
        }
    }

    pub fn return_instance(&mut self, card: &CardInstance, content: &Content) {
        for part in &card.parts {
            self.return_def(part.def, part.copies, content);
        }
    }
}
