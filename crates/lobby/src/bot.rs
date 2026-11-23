//! Bots: a knowledge profile plus a buying strategy. Used by the lobby to
//! fill seats and by the simulator for balance runs.
//!
//! A bot only ever acts through engine commands, exactly like a player.

use std::collections::BTreeSet;

use owl_core::content::{CardDef, HeroKind};
use owl_core::question::OPTIONS;
use owl_core::state::Player;
use owl_core::{
    AbilityAction, CardDefId, Category, Command, Content, Difficulty, Effect, Env, Event, GameState, Help, LobbyRng,
    Phase, PlayerId, Question, QuestionId, apply,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Knowledge {
    Low,
    Mid,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strategy {
    Specialist,
    Tank,
    Scorer,
    Reflect,
    Generalist,
}

pub const KNOWLEDGE: [Knowledge; 3] = [Knowledge::Low, Knowledge::Mid, Knowledge::High];
pub const STRATEGIES: [Strategy; 5] =
    [Strategy::Specialist, Strategy::Tank, Strategy::Scorer, Strategy::Reflect, Strategy::Generalist];

/// How much each strategy values each family of card: (category, armor, damage, reflect).
fn tastes(strategy: Strategy) -> (f32, f32, f32, f32) {
    match strategy {
        Strategy::Specialist => (1.0, 0.4, 0.6, 0.2),
        Strategy::Tank => (0.3, 1.0, 0.3, 0.5),
        Strategy::Scorer => (0.5, 0.3, 1.0, 0.1),
        Strategy::Reflect => (0.3, 0.7, 0.2, 1.0),
        Strategy::Generalist => (0.6, 0.6, 0.6, 0.4),
    }
}

pub struct Bot {
    pub id: PlayerId,
    pub knowledge: Knowledge,
    pub strategy: Strategy,
    /// Percent chance to truly know the answer, per category. A bot that does
    /// not know guesses among the options still showing.
    knows: Vec<u32>,
    strong: Vec<Category>,
    /// Questions whose answer the bot has already been shown.
    learned: BTreeSet<QuestionId>,
    rng: LobbyRng,
    /// Events caused by this bot, for the host to read and clear.
    pub events: Vec<Event>,
    /// Commands of this bot that the engine accepted, in order, for the match log.
    pub accepted: Vec<Command>,
}

impl Bot {
    pub fn new(id: PlayerId, knowledge: Knowledge, strategy: Strategy, content: &Content, seed: u64) -> Self {
        let mut rng = LobbyRng::from_seed(seed);
        let base = match knowledge {
            Knowledge::Low => 30,
            Knowledge::Mid => 50,
            Knowledge::High => 70,
        };
        let categories = content.category_count();
        let mut order: Vec<Category> = (0..categories as u8).map(Category).collect();
        rng.shuffle(&mut order);
        let strong: Vec<Category> = order.into_iter().take(2).collect();
        let knows =
            (0..categories as u8).map(|c| if strong.contains(&Category(c)) { base + 20 } else { base }).collect();
        Self { id, knowledge, strategy, knows, strong, learned: BTreeSet::new(), rng, events: Vec::new(), accepted: Vec::new() }
    }

    /// Picks from the bot's own offer: the hero that suits it if one does, otherwise either.
    pub fn pick_hero(&mut self, state: &mut GameState, env: &mut Env<'_>) {
        let content = env.content;
        let mut offer = self.me(state).hero_offer.clone();
        self.rng.shuffle(&mut offer);
        let wanted = |h: &owl_core::content::HeroDef| match self.strategy {
            Strategy::Reflect => h.kind == HeroKind::Herophilus,
            _ => h.category == self.strong[0],
        };
        offer.sort_by_key(|&hero| !wanted(content.hero(hero)));
        for hero in offer {
            if self.act(state, Command::PickHero { player: self.id, hero }, env) {
                return;
            }
        }
    }

    /// Sends a command as this bot. Returns whether the engine accepted it.
    fn act(&mut self, state: &mut GameState, command: Command, env: &mut Env<'_>) -> bool {
        match apply(state, command.clone(), env) {
            Ok(events) => {
                self.events.extend(events);
                self.accepted.push(command);
                true
            }
            Err(_) => false,
        }
    }

    fn me<'a>(&self, state: &'a GameState) -> &'a Player {
        &state.players[self.id.index()]
    }

    /// How much the bot wants a card. Around 1 is "worth a slot", higher is better.
    fn score(&self, def: CardDefId, state: &GameState, content: &Content) -> f32 {
        let card: &CardDef = content.card(def);
        let me = self.me(state);
        let (cat, armor, dmg, reflect) = tastes(self.strategy);
        let high = self.knowledge == Knowledge::High;
        let low = self.knowledge == Knowledge::Low;
        let owns = |pred: &dyn Fn(&Effect) -> bool| me.cards().any(|c| c.has_effect(content, pred));
        let mut total = 0.0;
        for effect in &card.effects {
            total += match *effect {
                Effect::CategoryWeight { percent } => {
                    let fits = card.category.is_some_and(|c| self.strong.contains(&c));
                    percent as f32 / 5.0 * cat * if fits { 1.0 } else { 0.1 }
                }
                Effect::Armor { amount } => amount as f32 / 1.5 * armor,
                Effect::AnswerTime { seconds } => 1.5 * seconds as f32,
                Effect::DamageOnWin { amount } => 2.5 * amount as f32 * dmg,
                Effect::Reflect { percent } => percent as f32 / 6.0 * reflect,
                Effect::TieBreaker => 1.5,
                Effect::ZeroZero { .. } => if low { 1.5 } else { 0.5 },
                Effect::GoldOnCorrect { .. } => if high { 3.0 } else if low { 0.5 } else { 2.0 },
                Effect::ShieldOnLoss { .. } => 2.5 * armor * if high { 0.5 } else { 1.0 },
                Effect::RemoveWrong { .. } => 4.0,
                Effect::DamageMultiplier { .. } => 4.0 * dmg * if low { 0.3 } else { 1.0 },
                Effect::ExtraDamageOnLoss { .. } => -1.0,
                Effect::ReflectPierce { .. } => 0.7,
                Effect::ArmorPierce { .. } => 1.5 * dmg,
                Effect::StreakDamage { .. } => 3.0 * dmg * if low { 0.3 } else { 1.0 },
                Effect::SplitBonus { .. } => 1.5 * cat,
                Effect::PerfectDouble => if high { 3.0 * dmg } else { 0.3 },
                Effect::EndgameFewerOptions | Effect::EndgameTime { .. } | Effect::EndgameShield => {
                    if owns(&Effect::is_endgame) { 0.0 } else { 2.0 }
                }
                Effect::Oracle { .. } => 7.0,
                Effect::AllCategories => 2.0 * cat,
                Effect::DamageCap { .. } => 4.0 * armor,
                Effect::HealPerTurn { .. } => {
                    if owns(&|e| matches!(e, Effect::HealPerTurn { .. })) { 0.0 } else { 3.0 }
                }
                Effect::HuntHealer => 0.3,
                Effect::Skull => 1.0,
                // Harder questions widen the gap between those who know and
                // those who guess: the knowledgeable raise, the rest lower.
                // A card that touches every question is worth more.
                Effect::DifficultyShift { steps, all } => {
                    let suits = if high { steps > 0 } else { low && steps < 0 };
                    let reach = if all { 1.5 } else { 1.0 };
                    if suits { reach * steps.abs() as f32 } else if high || low { -1.0 } else { 0.2 }
                }
                _ => 0.0,
            };
        }
        // Chase golden cards: a copy of something already owned is worth more.
        if card.can_golden {
            total += 1.5 * me.cards().filter(|c| c.is_plain(def)).count() as f32;
        }
        total
    }

    /// A golden card that has not given its reward yet is worth placing for that alone.
    fn card_score(&self, card: &owl_core::state::CardInstance, state: &GameState, content: &Content) -> f32 {
        card.parts
            .iter()
            .map(|p| {
                self.score(p.def, state, content) * if p.golden { 2.0 } else { 1.0 } + if p.reward_due { 3.0 } else { 0.0 }
            })
            .sum()
    }

    /// Plays one whole shop phase.
    pub fn shop(&mut self, state: &mut GameState, env: &mut Env<'_>) {
        let content = env.content;
        let t = &content.tunables;
        let player = self.id;
        if !self.me(state).alive {
            return;
        }
        self.hero_ability(state, env);
        let mut rerolls = 0;
        let mut unplaceable = BTreeSet::new();

        for _ in 0..60 {
            let me = &self.me(state).clone();
            if me.locked_out {
                return;
            }
            if let Some(choices) = me.rewards.first() {
                let best = best_index(choices.iter().map(|&d| self.score(d, state, content)));
                if self.act(state, Command::PickReward { player, index: best.unwrap_or(0) }, env) {
                    continue;
                }
            }

            // Cards used from the hand: potions when hurt, one skull per round.
            let usable = me.hand.iter().find(|c| {
                c.effects(content).any(|e| match e {
                    Effect::HealOnce { amount } => me.hp + amount <= me.max_hp,
                    _ => false,
                })
            });
            if let Some(card) = usable
                && self.act(state, Command::UseCard { player, card: card.id }, env) {
                    continue;
                }

            // A reward card opens its pick when placed and takes no slot.
            let is_reward = |e: &Effect| matches!(e, Effect::GoldenReward);
            let reward_card =
                me.hand.iter().find(|c| !unplaceable.contains(&c.id) && c.has_effect(content, is_reward));
            if let Some(card) = reward_card {
                if !self.act(state, Command::Place { player, card: card.id }, env) {
                    unplaceable.insert(card.id);
                }
                continue;
            }

            // Place the best card in hand, making room if it beats the worst on the board.
            let placeable = me
                .hand
                .iter()
                .filter(|c| !unplaceable.contains(&c.id))
                .filter(|c| !c.has_effect(content, |e| e.is_used_from_hand() || is_reward(e)))
                .map(|c| (c.id, self.card_score(c, state, content)))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((card, value)) = placeable {
                if me.board.len() < t.board_slots as usize {
                    if !self.act(state, Command::Place { player, card }, env) {
                        unplaceable.insert(card);
                    }
                    continue;
                }
                let worst = me
                    .board
                    .iter()
                    .map(|c| (c.id, self.card_score(c, state, content)))
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((worst, worst_value)) = worst
                    && value > worst_value + 0.5 && self.act(state, Command::Sell { player, card: worst }, env) {
                        continue;
                    }
                unplaceable.insert(card);
                continue;
            }

            // Level up roughly every other turn, as long as it is affordable.
            let wanted_level = (1 + state.turn / 2).min(t.max_tavern_level);
            if me.tavern < wanted_level && self.act(state, Command::UpgradeTavern { player }, env) {
                continue;
            }

            let extra = me.extras.iter().position(|&d| {
                let wants = if d == content.skull {
                    !me.hand.iter().any(|c| c.has_def(d))
                } else {
                    me.hp < me.max_hp
                };
                wants && content.card(d).cost <= me.gold
            });
            if let Some(index) = extra
                && self.act(state, Command::BuyExtra { player, index }, env) {
                    continue;
                }

            let offer = me
                .shop
                .iter()
                .enumerate()
                .filter(|&(_, &d)| content.card(d).cost <= me.gold)
                .map(|(i, &d)| (i, self.score(d, state, content)))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((index, value)) = offer
                && value >= 1.0 && self.act(state, Command::Buy { player, index }, env) {
                    continue;
                }

            if rerolls < 3 && me.gold >= t.reroll_cost + t.card_cost {
                rerolls += 1;
                if self.act(state, Command::Reroll { player }, env) {
                    continue;
                }
            }
            // Nothing worth buying: put spare gold into the tavern.
            if self.act(state, Command::UpgradeTavern { player }, env) {
                continue;
            }
            return;
        }
    }

    /// Uses the hero's activated ability when it is clearly worth it.
    fn hero_ability(&mut self, state: &mut GameState, env: &mut Env<'_>) {
        let content = env.content;
        let player = self.id;
        let me = self.me(state);
        let Some(hero) = me.hero.map(|h| content.hero(h)) else { return };
        // An ability limited per match works at most once a round.
        let limited = matches!(hero.kind, HeroKind::Darwin | HeroKind::Benz | HeroKind::Herodotus | HeroKind::Shakespeare);
        if limited && me.hero_state.used_turn == state.turn {
            return;
        }
        let action = match hero.kind {
            HeroKind::Herodotus => {
                let lost = me.last_battle.as_ref().is_some_and(|b| b.hp_lost > 0 && b.opponent.is_some());
                lost.then_some(AbilityAction::Rewrite)
            }
            HeroKind::Darwin => me
                .cards()
                .filter_map(|c| match &c.parts[..] {
                    [part] if !part.golden => Some(part.def),
                    _ => None,
                })
                .find(|&def| me.cards().filter(|c| c.is_plain(def)).count() >= 2)
                .map(|def| AbilityAction::DarwinMerge { def }),
            HeroKind::Benz => (me.board.len() >= content.tunables.board_slots as usize)
                .then(|| AbilityAction::Combine { a: me.board[0].id, b: me.board[1].id }),
            HeroKind::Lumiere => me
                .last_battle
                .iter()
                .flat_map(|b| &b.questions)
                .find(|q| q.category == hero.category)
                .map(|q| AbilityAction::Record { question: q.id }),
            HeroKind::Nietzsche => (me.gold >= hero.param("cost") + 3).then_some(AbilityAction::Ubermensch),
            HeroKind::Shakespeare => {
                let few_left = state.alive_count() <= hero.param("skip_above") as usize;
                let has_skull = me.hand.iter().any(|c| c.has_def(content.skull));
                let use_it = if few_left { !has_skull && self.rng.percent(25) } else { me.hp <= 6 };
                use_it.then_some(AbilityAction::ToBeOrNotToBe)
            }
            // The sword whenever the shop offers it and he can pay.
            HeroKind::Newton => (me.hero_state.sword_offer_turn == state.turn
                && me.hero_state.uses == 0
                && me.gold >= hero.param("cost"))
            .then_some(AbilityAction::BuySword),
            // The Silk Road when he can pay for the swap and still buy the card.
            HeroKind::MarcoPolo => (me.hero_state.used_turn != state.turn
                && !me.shop.is_empty()
                && me.gold >= hero.param("cost") + content.tunables.card_cost)
                .then_some(AbilityAction::SilkRoad),
            HeroKind::Herophilus
            | HeroKind::Pheidippides
            | HeroKind::Cleopatra
            | HeroKind::MarieCurie
            | HeroKind::NoAbility => None,
        };
        let Some(action) = action else { return };
        if !self.act(state, Command::UseAbility { player, action }, env) {
            return;
        }
        // Nietzsche then picks the two options he knows best.
        let mut options = self.me(state).hero_state.house_options.clone();
        if options.len() >= 2 {
            options.sort_by_key(|c| std::cmp::Reverse(self.knows[usize::from(c.0)]));
            let action = AbilityAction::ChooseHouse { first: options[0], second: options[1] };
            let _ = self.act(state, Command::UseAbility { player, action }, env);
        }
    }

    /// Decides whether the bot truly knows a question. An easy question is
    /// missed less often and a hard one more often, in proportion to how
    /// often the bot misses at all.
    fn recalls(&mut self, question: Question, extra_seconds: u32) -> bool {
        let chance = (self.knows[usize::from(question.category.0)] + 2 * extra_seconds).min(100);
        let miss = 100 - chance;
        let miss = match question.difficulty {
            Difficulty::Easy => miss * 7 / 10,
            Difficulty::Medium => miss,
            Difficulty::Hard => (miss * 13 / 10).min(100),
        };
        let known = self.learned.contains(&question.id) || self.rng.percent(100 - miss);
        self.learned.insert(question.id);
        known
    }

    fn guess(&mut self, removed: u8) -> u8 {
        let showing: Vec<u8> = (0..OPTIONS).filter(|o| removed & (1 << o) == 0).collect();
        showing[self.rng.below(showing.len() as u32) as usize]
    }

    /// Answers the current question if the bot is in a battle and has not locked in.
    pub fn answer(&mut self, state: &mut GameState, env: &mut Env<'_>) {
        let player = self.id;
        match state.phase {
            Phase::Battle => {
                let found = state.battles.iter().enumerate().filter(|(_, b)| !b.done).find_map(|(i, b)| {
                    b.fighters.iter().position(|f| f.player == Some(player) && f.locked.is_none()).map(|s| (i, s))
                });
                let Some((battle, side)) = found else { return };
                let b = &state.battles[battle];
                let question = b.plan[b.current].question;
                let fighter = &b.fighters[side];
                let (oracle, sword, second) = (fighter.oracle_left, fighter.sword_left, fighter.second_opinion_left);
                if self.recalls(question, fighter.stats.time_bonus) {
                    let _ = self.act(state, Command::SubmitAnswer { player, choice: Some(question.correct) }, env);
                    return;
                }
                if oracle > 0 && self.act(state, Command::UseHelp { player, help: Help::Oracle }, env) {
                    return;
                }
                if sword > 0 {
                    let _ = self.act(state, Command::UseHelp { player, help: Help::FlamingSword }, env);
                }
                if second > 0 {
                    let _ = self.act(state, Command::UseHelp { player, help: Help::SecondOpinion }, env);
                }
                let removed = state.battles[battle].fighters[side].removed;
                let choice = Some(self.guess(removed));
                let _ = self.act(state, Command::SubmitAnswer { player, choice }, env);
            }
            Phase::Endgame => {
                let Some(eg) = state.endgame.as_ref() else { return };
                let Some(side) = eg.players.iter().position(|&p| p == player) else { return };
                let (Some(question), None) = (eg.question, eg.locked[side]) else { return };
                let (oracle, removed) = (eg.oracle_left[side], eg.removed[side]);
                if self.recalls(question, 0) {
                    let _ = self.act(state, Command::SubmitAnswer { player, choice: Some(question.correct) }, env);
                    return;
                }
                if oracle > 0 && self.act(state, Command::UseHelp { player, help: Help::Oracle }, env)
                {
                    return;
                }
                let choice = Some(self.guess(removed));
                let _ = self.act(state, Command::SubmitAnswer { player, choice }, env);
            }
            _ => {}
        }
    }
}

fn best_index(scores: impl Iterator<Item = f32>) -> Option<usize> {
    scores.enumerate().max_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
}
