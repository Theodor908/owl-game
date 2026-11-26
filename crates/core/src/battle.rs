//! Regular battles: the question plan, answering, scoring and the turn's resolve step.

use std::collections::BTreeSet;

use crate::api::{DamageSource, Event, Help, RuleError};
use crate::content::{Content, Effect};
use crate::damage::{self, base_damage, win_damage};
use crate::engine::Env;
use crate::ids::{Category, PlayerId};
use crate::question::{Difficulty, OPTIONS, Question};
use crate::rng::LobbyRng;
use crate::state::{BattleState, Fighter, GameState, LastBattle, Phase, PlanSlot};
use crate::stats::{BattleStats, fold_stats_with};
use crate::{endgame, heroes, pairing, shop};

enum Opponent {
    Player(usize),
    /// A ghost built from this player's board and accuracy.
    Ghost(usize),
}

/// Closes the shop and starts every battle of the turn.
pub fn start_battles(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    for idx in 0..state.players.len() {
        if state.players[idx].alive {
            shop::settle_rewards(state, idx, env, events);
        }
        state.players[idx].taken_this_battle = 0;
    }

    let pairing = pairing::make_pairs(state, env.rng);
    state.battles.clear();
    state.phase = Phase::Battle;
    for &(a, b) in &pairing.pairs {
        state.last_met[a][b] = state.turn;
        state.last_met[b][a] = state.turn;
        apply_skull(state, a, b, env, events);
        apply_skull(state, b, a, env, events);
        apply_hunter(state, a, b, content);
        apply_hunter(state, b, a, content);
        let battle = build_battle(state, a, Opponent::Player(b), env);
        events.push(Event::BattleStarted {
            battle: state.battles.len(),
            a: PlayerId(a as u8),
            b: Some(PlayerId(b as u8)),
            ghost_of: None,
        });
        state.battles.push(battle);
    }
    if let Some((a, source)) = pairing.ghost {
        state.players[a].last_ghost_turn = state.turn;
        let battle = build_battle(state, a, Opponent::Ghost(source), env);
        events.push(Event::BattleStarted {
            battle: state.battles.len(),
            a: PlayerId(a as u8),
            b: None,
            ghost_of: Some(PlayerId(source as u8)),
        });
        state.battles.push(battle);
    }

    if state.battles.is_empty() {
        resolve_turn(state, env, events);
        return;
    }
    for battle in 0..state.battles.len() {
        open_question(state, battle, env, events);
    }
}

/// Each skull on a player's board disables one random card on the opponent's
/// board for this battle. A Healer is more likely to be picked than any other
/// card, no card is picked twice, and skulls cannot disable skulls.
fn apply_skull(state: &mut GameState, from: usize, to: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let is_healer = |e: &Effect| matches!(e, Effect::HealPerTurn { .. });
    let is_skull = |e: &Effect| matches!(e, Effect::Skull);
    let skulls = state.players[from].board.iter().filter(|c| c.has_effect(content, is_skull)).count();
    for _ in 0..skulls {
        let weights: Vec<u32> = state.players[to]
            .board
            .iter()
            .map(|c| match (c.disabled || c.has_effect(content, is_skull), c.has_effect(content, is_healer)) {
                (true, _) => 0,
                (false, true) => content.tunables.skull_healer_weight,
                (false, false) => 1,
            })
            .collect();
        let Some(i) = env.rng.pick_weighted(&weights) else { return };
        let turn = state.turn;
        let target = &mut state.players[to];
        target.board[i].disabled = true;
        if target.board[i].has_effect(content, is_healer) {
            target.heal_blocked = true;
            target.healer_disabled_turn = Some(turn);
        }
        events.push(Event::CardDisabled { player: target.id, card: target.board[i].id });
    }
}

/// A Hunter disables the opponent's Healer for its next heal.
fn apply_hunter(state: &mut GameState, from: usize, to: usize, content: &Content) {
    let active = |c: &&crate::state::CardInstance| !c.disabled;
    let hunts = state.players[from]
        .board
        .iter()
        .filter(active)
        .any(|c| c.has_effect(content, |e| matches!(e, Effect::HuntHealer)));
    let has_healer = state.players[to]
        .board
        .iter()
        .any(|c| c.has_effect(content, |e| matches!(e, Effect::HealPerTurn { .. })));
    if hunts && has_healer {
        state.players[to].heal_blocked = true;
        state.players[to].healer_disabled_turn = Some(state.turn);
    }
}

fn new_fighter(player: Option<usize>, ghost_of: Option<usize>, accuracy: (u32, u32), stats: BattleStats) -> Fighter {
    Fighter {
        player: player.map(|i| PlayerId(i as u8)),
        ghost_of: ghost_of.map(|i| PlayerId(i as u8)),
        ghost_accuracy: accuracy,
        second_opinion_left: stats.second_opinion_uses,
        sword_left: stats.sword_uses,
        oracle_left: stats.oracle_uses,
        stats,
        answers: Vec::new(),
        locked: None,
        removed: 0,
    }
}

/// Rolls a category from a player's weights: each percentage is that
/// category's chance, and the remainder is uniform over all categories.
pub fn roll_category(weights: &[u32], rng: &mut LobbyRng) -> Category {
    let roll = rng.below(100);
    let mut edge = 0;
    for (i, &weight) in weights.iter().enumerate() {
        edge += weight;
        if roll < edge {
            return Category(i as u8);
        }
    }
    Category(rng.below(weights.len() as u32) as u8)
}

/// Rolls a question's base difficulty from the tunable odds.
pub fn roll_difficulty(odds: &[u32; 3], rng: &mut LobbyRng) -> Difficulty {
    Difficulty::ALL[rng.pick_weighted(odds).unwrap_or(1)]
}

/// The difficulty a question of `category` should have: its base difficulty
/// moved by both players' cards. Their steps add up and cancel each other
/// first; only the result is capped at easy and hard.
pub fn wanted_difficulty(base: Difficulty, category: Category, a: &BattleStats, b: &BattleStats) -> Difficulty {
    let steps = |stats: &BattleStats| stats.difficulty.get(usize::from(category.0)).copied().unwrap_or(0);
    base.shifted(steps(a) + steps(b))
}

/// How many of the weighted questions go to side A. One side gets all but one
/// when its weight total leads by the threshold; otherwise they are shared evenly.
pub fn split(a: &BattleStats, b: &BattleStats, weighted: u32, threshold: u32) -> u32 {
    let (lead_a, lead_b) = (a.weight_total + a.split_bonus, b.weight_total + b.split_bonus);
    if lead_a >= lead_b + threshold {
        weighted - 1
    } else if lead_b >= lead_a + threshold {
        1
    } else {
        weighted / 2
    }
}

fn build_battle(state: &mut GameState, a: usize, opponent: Opponent, env: &mut Env<'_>) -> BattleState {
    let content = env.content;
    let t = &content.tunables;
    // A hero that silences its opponent turns off the other side's battle
    // passive; a ghost's side is its source player.
    let silences = |idx: usize| {
        state.players[idx].hero.is_some_and(|h| {
            let hero = content.hero(h);
            heroes::ability(hero.kind).silences_opponent(hero)
        })
    };
    let other = match opponent {
        Opponent::Player(b) | Opponent::Ghost(b) => b,
    };
    let (silenced_a, silenced_b) = (silences(other), silences(a));
    let stats_a = fold_stats_with(&state.players[a], content, !silenced_a);
    let (b, fighter_b) = match opponent {
        Opponent::Player(b) => {
            let stats = fold_stats_with(&state.players[b], content, !silenced_b);
            (Some(b), new_fighter(Some(b), None, (0, 0), stats))
        }
        Opponent::Ghost(source) => {
            let p = &state.players[source];
            (None, new_fighter(None, Some(source), (p.correct, p.asked), fold_stats_with(p, content, !silenced_b)))
        }
    };
    let fighter_a = new_fighter(Some(a), None, (0, 0), stats_a);
    let real = [Some(a), b];

    // House questions: random categories, unless a hero has chosen them.
    let mut house: Vec<Category> =
        (0..t.house_questions).map(|_| Category(env.rng.below(content.category_count()) as u8)).collect();
    for idx in real.into_iter().flatten() {
        if let Some((first, second)) = state.players[idx].hero_state.house_choice.take() {
            house = vec![first, second];
        }
    }

    let a_slots = split(&fighter_a.stats, &fighter_b.stats, t.weighted_questions, t.split_threshold);
    let mut slots: Vec<(Category, Option<u8>, Option<Question>)> = house.into_iter().map(|c| (c, None, None)).collect();
    for i in 0..t.weighted_questions {
        let (owner, stats) = if i < a_slots { (0, &fighter_a.stats) } else { (1, &fighter_b.stats) };
        slots.push((roll_category(&stats.weights, env.rng), Some(owner), None));
    }

    // A recorded question takes one of its owner's weighted slots.
    for (side, idx) in real.into_iter().enumerate() {
        let Some(idx) = idx else { continue };
        if let Some(question) = state.players[idx].hero_state.recorded.take()
            && let Some(slot) = slots.iter_mut().find(|s| s.1 == Some(side as u8) && s.2.is_none())
        {
            *slot = (question.category, slot.1, Some(question));
        }
    }
    env.rng.shuffle(&mut slots);

    let empty = BTreeSet::new();
    let mut plan = Vec::new();
    for (category, owner, preset) in slots {
        let question = preset.unwrap_or_else(|| {
            let seen_b = b.map_or(&empty, |b| &state.players[b].seen);
            let base = roll_difficulty(&t.difficulty_odds, env.rng);
            let difficulty = wanted_difficulty(base, category, &fighter_a.stats, &fighter_b.stats);
            env.questions.draw(category, difficulty, &state.players[a].seen, seen_b, env.rng)
        });
        for idx in real.into_iter().flatten() {
            state.players[idx].seen.insert(question.id);
        }
        plan.push(PlanSlot { question, owner });
    }
    BattleState { fighters: [fighter_a, fighter_b], plan, current: 0, done: false }
}

fn open_question(state: &mut GameState, battle: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let t = &env.content.tunables;
    let b = &mut state.battles[battle];
    let question = b.plan[b.current].question;
    for fighter in &mut b.fighters {
        fighter.removed = 0;
        fighter.locked = None;
        if fighter.player.is_none() {
            // A ghost answers at once, from its source player's accuracy so far.
            let (correct, asked) = fighter.ghost_accuracy;
            let right = if asked == 0 { env.rng.percent(50) } else { env.rng.chance(correct, asked) };
            fighter.locked = Some(right.then_some(question.correct));
        }
    }
    events.push(Event::CategoryRevealed { battle, category: question.category, difficulty: question.difficulty });
    events.push(Event::QuestionShown {
        battle,
        question: question.id,
        time_a: t.answer_time + b.fighters[0].stats.time_bonus,
        time_b: t.answer_time + b.fighters[1].stats.time_bonus,
    });
}

/// Finds the unfinished battle a player is in: (battle index, side).
fn locate(state: &GameState, player: PlayerId) -> Result<(usize, usize), RuleError> {
    state
        .battles
        .iter()
        .enumerate()
        .filter(|(_, b)| !b.done)
        .find_map(|(i, b)| b.fighters.iter().position(|f| f.player == Some(player)).map(|side| (i, side)))
        .ok_or(RuleError::NotInBattle)
}

/// Picks up to `count` wrong options that are still showing.
pub fn pick_wrong(correct: u8, removed: u8, count: u32, rng: &mut LobbyRng) -> u8 {
    let mut candidates: Vec<u8> = (0..OPTIONS).filter(|&o| o != correct && removed & (1 << o) == 0).collect();
    rng.shuffle(&mut candidates);
    candidates.into_iter().take(count as usize).fold(0, |mask, o| mask | (1 << o))
}

pub fn use_help(
    state: &mut GameState,
    player: PlayerId,
    help: Help,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let (battle, side) = locate(state, player)?;
    let b = &mut state.battles[battle];
    let question = b.plan[b.current].question;
    let fighter = &mut b.fighters[side];
    if fighter.locked.is_some() {
        return Err(RuleError::AlreadyAnswered);
    }
    let (charges, removes) = match help {
        Help::SecondOpinion => (&mut fighter.second_opinion_left, 1),
        Help::FlamingSword => (&mut fighter.sword_left, fighter.stats.sword_removes),
        Help::Oracle => (&mut fighter.oracle_left, 0),
    };
    if *charges == 0 {
        return Err(RuleError::NoCharges);
    }
    if help == Help::Oracle {
        *charges -= 1;
        fighter.locked = Some(Some(question.correct));
        reveal_if_ready(state, battle, env, events);
        return Ok(());
    }
    let mask = pick_wrong(question.correct, fighter.removed, removes, env.rng);
    if mask == 0 {
        return Err(RuleError::NotUsable);
    }
    *charges -= 1;
    fighter.removed |= mask;
    events.push(Event::OptionsRemoved { player, removed_mask: fighter.removed });
    // Newton's revelations are bought, so a used one is gone for good.
    if help == Help::FlamingSword {
        let uses = &mut state.players[player.index()].hero_state.uses;
        *uses = uses.saturating_sub(1);
    }
    Ok(())
}

pub fn submit(
    state: &mut GameState,
    player: PlayerId,
    choice: Option<u8>,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let (battle, side) = locate(state, player)?;
    let fighter = &mut state.battles[battle].fighters[side];
    if fighter.locked.is_some() {
        return Err(RuleError::AlreadyAnswered);
    }
    if let Some(option) = choice
        && (option >= OPTIONS || fighter.removed & (1 << option) != 0)
    {
        return Err(RuleError::InvalidAnswer);
    }
    fighter.locked = Some(choice);
    reveal_if_ready(state, battle, env, events);
    Ok(())
}

pub fn timeout(
    state: &mut GameState,
    battle: usize,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let b = state.battles.get_mut(battle).filter(|b| !b.done).ok_or(RuleError::BadIndex)?;
    for fighter in &mut b.fighters {
        fighter.locked.get_or_insert(None);
    }
    reveal_if_ready(state, battle, env, events);
    Ok(())
}

/// A question ends the moment both sides have locked in.
fn reveal_if_ready(state: &mut GameState, battle: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let b = &mut state.battles[battle];
    if b.fighters.iter().any(|f| f.locked.is_none()) {
        return;
    }
    let question = b.plan[b.current].question;
    let mut right = [false; 2];
    let mut real = [None; 2];
    for (side, fighter) in b.fighters.iter_mut().enumerate() {
        right[side] = fighter.locked == Some(Some(question.correct));
        fighter.answers.push(right[side]);
        real[side] = fighter.player;
    }
    b.current += 1;
    b.done = b.current == b.plan.len();
    let done = b.done;
    let scores = [b.fighters[0].correct_count(), b.fighters[1].correct_count()];

    for side in 0..2 {
        if let Some(player) = real[side] {
            let p = &mut state.players[player.index()];
            p.asked += 1;
            p.correct += u32::from(right[side]);
        }
    }
    events.push(Event::AnswerRevealed {
        battle,
        correct_option: question.correct,
        a_correct: right[0],
        b_correct: right[1],
    });

    if !done {
        open_question(state, battle, env, events);
        return;
    }
    events.push(Event::BattleScored { battle, a_correct: scores[0], b_correct: scores[1] });
    if state.battles.iter().all(|b| b.done) {
        resolve_turn(state, env, events);
    }
}

/// Damage per correct answer in the fighter's bonus categories.
fn category_bonus(fighter: &Fighter, plan: &[PlanSlot]) -> u32 {
    fighter
        .stats
        .bonus_categories
        .iter()
        .map(|&(category, damage)| {
            let right = plan
                .iter()
                .zip(&fighter.answers)
                .filter(|(slot, answer)| **answer && slot.question.category == category)
                .count() as u32;
            right * damage
        })
        .sum()
}

/// The multiplier on this turn's regular battle damage, in percent. The
/// endgame has its own rising damage and is never multiplied.
pub fn damage_percent(state: &GameState, content: &Content) -> u32 {
    match state.phase {
        Phase::Endgame | Phase::Finished => 100,
        _ => content.tunables.escalation_percent(state.turn),
    }
}

/// Runs once every battle of the turn is scored: damage, rewards, deaths,
/// and the move to the next turn, the endgame or the end of the match.
fn resolve_turn(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let hp_before: Vec<u32> = state.players.iter().map(|p| p.hp).collect();
    for battle in 0..state.battles.len() {
        resolve_battle(state, battle, env, events);
    }
    for p in &mut state.players {
        // Placed skulls last one round, whether or not their owner fought.
        p.board.retain(|c| !c.has_effect(content, |e| matches!(e, Effect::Skull)));
        p.skip_battle = false;
        p.temp_armor = 0;
        p.board.iter_mut().for_each(|c| c.disabled = false);
    }

    // Deaths are simultaneous. Among players dying together, the one who had
    // more HP before this resolve places higher.
    let mut dying: Vec<usize> = state.alive().filter(|&i| state.players[i].hp == 0).collect();
    dying.retain(|&idx| !heroes::on_would_die(state, idx, env, events));
    dying.sort_by_key(|&i| (hp_before[i], i));
    for idx in dying {
        let placement = state.alive_count() as u32;
        eliminate(state, idx, placement, content, events);
    }

    match state.alive_count() {
        0 | 1 => {
            let winner = state
                .players
                .iter_mut()
                .find(|p| p.alive || p.placement == Some(1))
                .expect("a lobby always has a best-placed player");
            winner.placement = Some(1);
            let winner = winner.id;
            state.phase = Phase::Finished;
            events.push(Event::MatchFinished { winner });
        }
        2 if state.turn >= content.tunables.endgame_min_turn => endgame::start(state, env, events),
        _ => shop::start_turn(state, env, events),
    }
}

pub fn eliminate(state: &mut GameState, idx: usize, placement: u32, content: &Content, events: &mut Vec<Event>) {
    let p = &mut state.players[idx];
    p.alive = false;
    p.placement = Some(placement);
    let player = p.id;
    let cards: Vec<_> = p.board.drain(..).chain(p.hand.drain(..)).collect();
    let offers: Vec<_> = p.shop.drain(..).chain(p.extras.drain(..)).chain(p.rewards.drain(..).flatten()).collect();
    for card in &cards {
        state.return_instance(card, content);
    }
    for def in offers {
        state.return_def(def, 1, content);
    }
    events.push(Event::PlayerEliminated { player, placement });
}

fn resolve_battle(state: &mut GameState, battle: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let t = &content.tunables;
    let b = state.battles[battle].clone();
    let [fa, fb] = &b.fighters;
    let idx = [fa.player.map(PlayerId::index), fb.player.map(PlayerId::index)];
    let score = [fa.correct_count(), fb.correct_count()];
    let hp_before = idx.map(|i| i.map_or(0, |i| state.players[i].hp));
    let streak = idx.map(|i| i.map_or(0, |i| state.players[i].win_streak));
    let total = b.plan.len() as u32;

    // The damage side `w` would deal to side `l` with the given margin.
    // Flat bonuses on a win: category cards, and a point for each of the
    // winner's own correct answers where the hero gives one (Pheidippides).
    let bonus = |w: usize| {
        let stats = &b.fighters[w].stats;
        category_bonus(&b.fighters[w], &b.plan) + (score[w] * stats.damage_per_correct).min(stats.damage_per_correct_max)
    };
    let base = |w: usize, margin: u32| base_damage(&b.fighters[w].stats, margin, bonus(w), streak[w], t);
    // Late in the match, battle, overtime and Arrow damage is multiplied,
    // after the per-battle cap. Reflect follows from the damage taken. The
    // tie cards keep their own small numbers.
    let turn = state.turn;
    let escalate = |amount: u32| t.escalate(amount, turn);
    let mut hit = |state: &mut GameState, from: usize, amount: u32, source: DamageSource| {
        let to = 1 - from;
        damage::deal(state, idx[from], &b.fighters[from].stats, idx[to], &b.fighters[to].stats, amount, source, content, events);
    };

    let winner = match score[0].cmp(&score[1]) {
        std::cmp::Ordering::Greater => Some(0),
        std::cmp::Ordering::Less => Some(1),
        std::cmp::Ordering::Equal => None,
    };
    if let Some(w) = winner {
        let l = 1 - w;
        let amount = win_damage(&b.fighters[w].stats, &b.fighters[l].stats, base(w, score[w] - score[l]), score[w] == total, t);
        let amount = (amount + b.fighters[w].stats.strike).min(t.max_battle_damage);
        hit(state, w, escalate(amount), DamageSource::Battle);
    } else {
        // A tie deals nothing by itself; tie cards and overtime create damage.
        let [sa, sb] = [&fa.stats, &fb.stats];
        if score[0] >= 1 && sa.tie_breaker != sb.tie_breaker {
            let holder = usize::from(sb.tie_breaker);
            hit(state, holder, (b.fighters[holder].stats.tavern / 2).max(1), DamageSource::TieBreaker);
        }
        if score[0] == 0 && (sa.zero_zero > 0) != (sb.zero_zero > 0) {
            let holder = usize::from(sb.zero_zero > 0);
            hit(state, holder, b.fighters[holder].stats.zero_zero, DamageSource::ZeroZero);
        }
        if state.turn >= t.overtime_turn {
            let higher = (0..2)
                .map(|w| (base(w, 0) * (100 + b.fighters[w].stats.multiplier_percent) / 100).min(t.max_battle_damage))
                .max()
                .unwrap_or(0);
            let half = escalate((higher / 2).max(1));
            hit(state, 0, half, DamageSource::Overtime);
            hit(state, 1, half, DamageSource::Overtime);
        }
    }

    // A strike hits whatever the result. The winner's is already in the battle damage.
    for side in (0..2).filter(|&side| winner != Some(side)) {
        let strike = b.fighters[side].stats.strike.min(t.max_battle_damage);
        if strike > 0 {
            hit(state, side, escalate(strike), DamageSource::Battle);
        }
    }

    // Arrows hit whatever the result, then vanish.
    for side in 0..2 {
        let arrow = b.fighters[side].stats.arrow_damage;
        if let (Some(i), true) = (idx[side], arrow > 0) {
            hit(state, side, escalate(arrow), DamageSource::Arrow);
            state.players[i].board.retain(|c| c.disabled || !c.has_effect(content, |e| matches!(e, Effect::Arrow { .. })));
        }
    }

    // One-battle cards leave the board after the battle they fought in,
    // disabled or not, and the hero sees the battle end.
    for i in idx.into_iter().flatten() {
        let player = state.players[i].id;
        let (gone, kept) = std::mem::take(&mut state.players[i].board)
            .into_iter()
            .partition(|c| c.lasts_one_battle(content));
        state.players[i].board = kept;
        for card in gone {
            state.return_instance(&card, content);
            events.push(Event::CardUsed { player, card: card.id });
        }
        if let Some(hero) = state.players[i].hero {
            let hero = content.hero(hero);
            heroes::ability(hero.kind).on_battle_end(hero, &mut state.players[i]);
        }
    }

    for side in 0..2 {
        let Some(i) = idx[side] else { continue };
        let fighter = &b.fighters[side];
        let won = winner == Some(side);
        let lost = winner == Some(1 - side);
        let p = &mut state.players[i];
        p.win_streak = if won { p.win_streak + 1 } else { 0 };
        p.bonus_gold += fighter.stats.gold_rules.iter().filter(|r| score[side] >= r.0).map(|r| r.1).sum::<u32>();
        p.last_battle = Some(LastBattle {
            opponent: idx[1 - side].map(|o| PlayerId(o as u8)),
            hp_lost: hp_before[side].saturating_sub(p.hp),
            questions: b.plan.iter().map(|s| s.question).collect(),
        });
        if lost {
            for &def in &fighter.stats.loss_shields {
                if state.players[i].hand.len() < t.hand_size as usize {
                    let card = state.new_instance(def, false, 1, content);
                    events.push(Event::CardGained { player: PlayerId(i as u8), card: card.id, def });
                    state.players[i].hand.push(card);
                }
            }
            shop::check_merges(state, i, env, events);
        }
    }
}
