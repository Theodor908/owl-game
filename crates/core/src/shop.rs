//! The shop phase: income, offers, buying, selling, placing, merging.

use crate::api::{Event, RuleError};
use crate::content::{CardDef, Content, Effect};
use crate::engine::Env;
use crate::heroes;
use crate::ids::{CardDefId, CardInstanceId, PlayerId};
use crate::state::{CardInstance, GameState, Phase};

/// Checks that `player` may act in the shop and returns their index.
pub fn shopper(state: &GameState, player: PlayerId) -> Result<usize, RuleError> {
    if state.phase != Phase::Shop {
        return Err(RuleError::WrongPhase);
    }
    let idx = player.index();
    let p = state.players.get(idx).ok_or(RuleError::UnknownPlayer)?;
    if !p.alive {
        return Err(RuleError::PlayerDead);
    }
    if p.locked_out {
        return Err(RuleError::LockedOut);
    }
    Ok(idx)
}

/// Starts the next turn: income, healing, and a fresh shop for everyone alive.
pub fn start_turn(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let t = &content.tunables;
    state.turn += 1;
    state.phase = Phase::Shop;
    state.battles.clear();
    events.push(Event::TurnStarted { turn: state.turn });
    let income = (t.gold_start + t.gold_per_turn * (state.turn - 1)).min(t.gold_cap);

    for idx in 0..state.players.len() {
        let turn = state.turn;
        let p = &mut state.players[idx];
        if !p.alive {
            continue;
        }
        // Unspent gold is lost, unless the tunable says otherwise or the hero keeps some.
        let kept = p.hero.map_or(0, |h| {
            let hero = content.hero(h);
            heroes::ability(hero.kind).gold_kept(hero, p.gold, income, t)
        });
        let carried = if t.gold_carries_over { p.gold } else { kept };
        p.gold = carried + income + p.bonus_gold;
        p.bonus_gold = 0;
        if turn > 1 {
            p.upgrade_discount += 1;
        }
        p.acted_this_shop = false;
        p.locked_out = false;

        let healing: u32 = p
            .board
            .iter()
            .flat_map(|c| c.effects(content))
            .map(|e| if let Effect::HealPerTurn { amount } = e { *amount } else { 0 })
            .sum();
        let blocked = std::mem::take(&mut p.heal_blocked);
        if healing > 0 && !blocked {
            heal(state, idx, healing, events);
        }
        refresh_shop(state, idx, env, true);
    }
}

/// Refills a player's shop. On a new turn a frozen shop keeps its cards and
/// the extra offers are rolled again; on a reroll everything is replaced.
fn refresh_shop(state: &mut GameState, idx: usize, env: &mut Env<'_>, new_turn: bool) {
    let content = env.content;
    let t = &content.tunables;
    let keep = new_turn && state.players[idx].frozen;
    state.players[idx].frozen = false;
    if !keep {
        for def in std::mem::take(&mut state.players[idx].shop) {
            state.return_def(def, 1, content);
        }
    }
    let tavern = state.players[idx].tavern;
    let size = t.shop_size[(tavern - 1) as usize] as usize;
    while state.players[idx].shop.len() < size {
        let Some(def) = draw(state, env, |c| c.in_shop && c.tier <= tavern) else { break };
        state.players[idx].shop.push(def);
    }

    if new_turn {
        for def in std::mem::take(&mut state.players[idx].extras) {
            state.return_def(def, 1, content);
        }
        let chance = t.potion_chance_base + t.potion_chance_per_turn * (state.turn - 1);
        let potion = content.potion;
        if state.pool[usize::from(potion.0)] > 0 && env.rng.percent(chance) {
            state.pool[usize::from(potion.0)] -= 1;
            state.players[idx].extras.push(potion);
        }
    }
    // Every hero in the lobby gets a say in every shop (Shakespeare's skulls).
    let present: Vec<_> = state.players.iter().filter_map(|p| p.hero).collect();
    for hero in present {
        let hero = content.hero(hero);
        heroes::ability(hero.kind).on_shop_roll(hero, state, idx, env);
    }
}

/// Takes one random card out of the shared pool, weighted by copies left.
pub fn draw(state: &mut GameState, env: &mut Env<'_>, allowed: impl Fn(&CardDef) -> bool) -> Option<CardDefId> {
    let weights: Vec<u32> = env
        .content
        .cards
        .iter()
        .zip(&state.pool)
        .map(|(card, &left)| if allowed(card) { left } else { 0 })
        .collect();
    let i = env.rng.pick_weighted(&weights)?;
    state.pool[i] -= 1;
    Some(CardDefId(i as u16))
}

pub fn heal(state: &mut GameState, idx: usize, amount: u32, events: &mut Vec<Event>) {
    let p = &mut state.players[idx];
    let healed = amount.min(p.max_hp - p.hp);
    p.hp += healed;
    if healed > 0 {
        events.push(Event::Healed { player: p.id, amount: healed });
    }
}

pub fn buy(
    state: &mut GameState,
    player: PlayerId,
    index: usize,
    extra: bool,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let content = env.content;
    let p = &state.players[idx];
    let offers = if extra { &p.extras } else { &p.shop };
    let def = *offers.get(index).ok_or(RuleError::BadIndex)?;
    let cost = content.card(def).cost;
    if p.gold < cost {
        return Err(RuleError::NotEnoughGold);
    }
    // A full hand takes one more card only if buying it makes a golden (or
    // combined) card at once, which takes the bought card away again.
    let hand_full = p.hand.len() >= content.tunables.hand_size as usize;
    if hand_full && !completes_merge(state, idx, def, content) {
        return Err(RuleError::HandFull);
    }

    let card = state.new_instance(def, false, 1, content);
    let p = &mut state.players[idx];
    p.gold -= cost;
    if extra {
        p.extras.remove(index);
    } else {
        p.shop.remove(index);
    }
    p.acted_this_shop = true;
    events.push(Event::CardBought { player, card: card.id, def });
    p.hand.push(card);
    check_merges(state, idx, env, events);
    Ok(())
}

pub fn sell(
    state: &mut GameState,
    player: PlayerId,
    card: CardInstanceId,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let content = env.content;
    let p = &mut state.players[idx];
    let sold = p.take_card(card).ok_or(RuleError::CardNotFound)?;
    // Tokens are worth nothing, so effects that hand them out are not an income.
    if sold.parts.iter().any(|part| !content.card(part.def).is_token()) {
        p.gold += content.tunables.sell_value;
    }
    if p.endgame_choice == Some(card) {
        p.endgame_choice = None;
    }
    p.acted_this_shop = true;
    state.return_instance(&sold, content);
    events.push(Event::CardSold { player, card });
    Ok(())
}

pub fn place(
    state: &mut GameState,
    player: PlayerId,
    card: CardInstanceId,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let content = env.content;
    let p = &mut state.players[idx];
    let pos = p.hand.iter().position(|c| c.id == card).ok_or(RuleError::CardNotFound)?;
    if p.hand[pos].has_effect(content, Effect::is_used_from_hand) {
        return Err(RuleError::NotUsable);
    }
    // A reward card never reaches the board: placing it opens the pick and
    // the card is gone, so it needs no free slot.
    if p.hand[pos].has_effect(content, |e| matches!(e, Effect::GoldenReward)) {
        if !reward_available(state, idx, content) {
            return Err(RuleError::NotUsable);
        }
        let p = &mut state.players[idx];
        p.hand.remove(pos);
        p.acted_this_shop = true;
        events.push(Event::CardUsed { player, card });
        offer_reward(state, idx, env, events);
        return Ok(());
    }
    if p.board.len() >= content.tunables.board_slots as usize {
        return Err(RuleError::BoardFull);
    }
    for part in &p.hand[pos].parts {
        if let Some(max) = content.card(part.def).max_on_board {
            let on_board = p.board.iter().filter(|c| c.has_def(part.def)).count();
            if on_board >= max as usize {
                return Err(RuleError::BoardLimit);
            }
        }
    }
    let placed = p.hand.remove(pos);
    p.board.push(placed);
    p.acted_this_shop = true;
    events.push(Event::CardPlaced { player, card });
    offer_due_rewards(state, idx, card, env, events);
    check_merges(state, idx, env, events);
    Ok(())
}

/// The tier golden reward options come from: the next one up from the
/// player's tavern level at the moment the pick is opened.
fn reward_tier(state: &GameState, idx: usize, content: &Content) -> u32 {
    (state.players[idx].tavern + 1).min(content.tunables.max_tavern_level)
}

/// True if the pool can give the player at least one reward option now.
fn reward_available(state: &GameState, idx: usize, content: &Content) -> bool {
    let tier = reward_tier(state, idx, content);
    content.cards.iter().zip(&state.pool).any(|(card, &left)| left > 0 && card.in_shop && card.tier == tier)
}

/// Offers one golden reward: a pick of different cards from the next tier up.
fn offer_reward(state: &mut GameState, idx: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let tier = reward_tier(state, idx, content);
    let mut choices = Vec::new();
    let mut names: Vec<&str> = Vec::new();
    for _ in 0..content.tunables.golden_reward_choices {
        if let Some(def) = draw(state, env, |c| c.in_shop && c.tier == tier && !names.contains(&c.id.as_str())) {
            names.push(&content.card(def).id);
            choices.push(def);
        }
    }
    if !choices.is_empty() {
        events.push(Event::RewardOffered { player: state.players[idx].id, choices: choices.clone() });
        state.players[idx].rewards.push(choices);
    }
}

/// Offers the reward of every golden part of a card on the board that has
/// not given one yet. Each golden card gives its reward once: taking it
/// back and placing it again gives nothing.
pub fn offer_due_rewards(
    state: &mut GameState,
    idx: usize,
    card: CardInstanceId,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) {
    let Some(placed) = state.players[idx].board.iter_mut().find(|c| c.id == card) else { return };
    let mut due = 0;
    for part in placed.parts.iter_mut().filter(|part| part.reward_due) {
        part.reward_due = false;
        due += 1;
    }
    for _ in 0..due {
        offer_reward(state, idx, env, events);
    }
}

pub fn use_card(
    state: &mut GameState,
    player: PlayerId,
    card: CardInstanceId,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let content = env.content;
    let p = &mut state.players[idx];
    let pos = p.hand.iter().position(|c| c.id == card).ok_or(RuleError::CardNotFound)?;
    let effect = p.hand[pos]
        .effects(content)
        .find(|e| e.is_used_from_hand())
        .cloned()
        .ok_or(RuleError::NotUsable)?;
    match effect {
        Effect::HealOnce { amount } => {
            // A used potion leaves the game: it is not returned to the pool.
            p.hand.remove(pos);
            heal(state, idx, amount, events);
        }
        _ => return Err(RuleError::NotUsable),
    }
    state.players[idx].acted_this_shop = true;
    events.push(Event::CardUsed { player, card });
    Ok(())
}

pub fn reroll(
    state: &mut GameState,
    player: PlayerId,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let cost = env.content.tunables.reroll_cost;
    let p = &mut state.players[idx];
    if p.gold < cost {
        return Err(RuleError::NotEnoughGold);
    }
    p.gold -= cost;
    p.acted_this_shop = true;
    p.frozen = false;
    refresh_shop(state, idx, env, false);
    events.push(Event::ShopRolled { player });
    Ok(())
}

pub fn toggle_freeze(state: &mut GameState, player: PlayerId) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let p = &mut state.players[idx];
    p.frozen = !p.frozen;
    Ok(())
}

/// Gold needed for the player's next tavern level, after the waiting discount.
pub fn upgrade_cost(state: &GameState, idx: usize, content: &Content) -> Option<u32> {
    let p = &state.players[idx];
    let base = content.tunables.upgrade_costs.get((p.tavern - 1) as usize)?;
    Some(base.saturating_sub(p.upgrade_discount))
}

pub fn upgrade(
    state: &mut GameState,
    player: PlayerId,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let cost = upgrade_cost(state, idx, env.content).ok_or(RuleError::MaxTavern)?;
    let p = &mut state.players[idx];
    if p.gold < cost {
        return Err(RuleError::NotEnoughGold);
    }
    p.gold -= cost;
    p.tavern += 1;
    p.upgrade_discount = 0;
    p.acted_this_shop = true;
    events.push(Event::TavernUpgraded { player, level: p.tavern });
    Ok(())
}

pub fn pick_reward(
    state: &mut GameState,
    player: PlayerId,
    index: usize,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let content = env.content;
    let p = &state.players[idx];
    let choices = p.rewards.first().ok_or(RuleError::BadIndex)?;
    let def = *choices.get(index).ok_or(RuleError::BadIndex)?;
    if p.hand.len() >= content.tunables.hand_size as usize {
        return Err(RuleError::HandFull);
    }
    let choices = state.players[idx].rewards.remove(0);
    for (i, other) in choices.into_iter().enumerate() {
        if i != index {
            state.return_def(other, 1, content);
        }
    }
    let card = state.new_instance(def, false, 1, content);
    events.push(Event::CardGained { player, card: card.id, def });
    state.players[idx].hand.push(card);
    state.players[idx].acted_this_shop = true;
    check_merges(state, idx, env, events);
    Ok(())
}

/// Resolves rewards a player left unpicked when the shop closed. The options
/// go back to the pool and the player gets a reward card in hand, to place
/// on a later turn; that opens a fresh pick. A full hand has no room for a
/// card of any kind, so then the reward is lost, as it always was.
pub fn settle_rewards(state: &mut GameState, idx: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let player = state.players[idx].id;
    for choices in std::mem::take(&mut state.players[idx].rewards) {
        for def in choices {
            state.return_def(def, 1, content);
        }
        if state.players[idx].hand.len() < content.tunables.hand_size as usize {
            let def = content.golden_reward;
            let card = state.new_instance(def, false, 1, content);
            events.push(Event::CardGained { player, card: card.id, def });
            state.players[idx].hand.push(card);
        }
    }
}

pub fn choose_endgame(
    state: &mut GameState,
    player: PlayerId,
    card: CardInstanceId,
    env: &mut Env<'_>,
) -> Result<(), RuleError> {
    let idx = shopper(state, player)?;
    let p = &mut state.players[idx];
    let found = p.find_card(card).ok_or(RuleError::CardNotFound)?;
    if !found.has_effect(env.content, Effect::is_endgame) {
        return Err(RuleError::NotUsable);
    }
    p.endgame_choice = Some(card);
    Ok(())
}

/// True when the player may turn copies of `def` into a golden card.
/// Healers have their own conditions; every other card only needs the copies.
fn golden_allowed(state: &GameState, idx: usize, def: CardDefId, content: &Content) -> bool {
    let card = content.card(def);
    if !card.can_golden {
        return false;
    }
    if !card.has(|e| matches!(e, Effect::HealPerTurn { .. })) {
        return true;
    }
    let p = &state.players[idx];
    let on_board = p.board.iter().any(|c| c.is_plain(def));
    let in_hand = p.hand.iter().filter(|c| c.is_plain(def)).count();
    let disabled_last_round = state.turn > 0 && p.healer_disabled_turn == Some(state.turn - 1);
    !state.golden_healer_made && on_board && in_hand >= 2 && !disabled_last_round
}

/// Merges every set of three identical plain cards the player holds, in hand
/// or on the board, until none is left.
pub fn check_merges(state: &mut GameState, idx: usize, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    loop {
        let p = &state.players[idx];
        let candidate = p.cards().find_map(|card| {
            let [part] = &card.parts[..] else { return None };
            if part.golden || p.cards().filter(|c| c.is_plain(part.def)).count() < 3 {
                return None;
            }
            let mergeable = content.card(part.def).merges_into.is_some()
                || golden_allowed(state, idx, part.def, content);
            mergeable.then_some(part.def)
        });
        let Some(def) = candidate else { break };
        match content.card(def).merges_into {
            Some(target) => {
                let (slot, _) = remove_plain(state, idx, def, 3);
                let card = state.new_instance(target, false, 1, content);
                let player = state.players[idx].id;
                events.push(Event::CardGained { player, card: card.id, def: target });
                insert(state, idx, slot, card);
            }
            None => make_golden(state, idx, def, 3, env, events),
        }
    }
}

/// True if one more plain copy of `def` would merge at once: the player
/// already holds two, in hand or on the board, and the card may merge.
fn completes_merge(state: &GameState, idx: usize, def: CardDefId, content: &Content) -> bool {
    let held = state.players[idx].cards().filter(|c| c.is_plain(def)).count();
    held >= 2 && (content.card(def).merges_into.is_some() || golden_allowed(state, idx, def, content))
}

/// Removes `count` plain copies of `def`, board first. Returns the board slot
/// of the first one removed from the board, and the removed cards.
fn remove_plain(state: &mut GameState, idx: usize, def: CardDefId, count: usize) -> (Option<usize>, Vec<CardInstance>) {
    let p = &mut state.players[idx];
    let mut slot = None;
    let mut removed = Vec::new();
    while removed.len() < count {
        if let Some(i) = p.board.iter().position(|c| c.is_plain(def)) {
            slot.get_or_insert(i);
            removed.push(p.board.remove(i));
        } else if let Some(i) = p.hand.iter().position(|c| c.is_plain(def)) {
            removed.push(p.hand.remove(i));
        } else {
            break;
        }
    }
    (slot, removed)
}

fn insert(state: &mut GameState, idx: usize, slot: Option<usize>, card: CardInstance) {
    let p = &mut state.players[idx];
    match slot {
        Some(i) => p.board.insert(i.min(p.board.len()), card),
        None => p.hand.push(card),
    }
}

/// Turns `count` plain copies of `def` into one golden card and fires its
/// on-golden effects. The golden reward is offered when the card is placed
/// on the board, not here; a card that goes straight to the board because
/// the hand is full counts as placed. The caller has checked that the
/// copies exist.
pub fn make_golden(
    state: &mut GameState,
    idx: usize,
    def: CardDefId,
    count: usize,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) {
    let content = env.content;
    let t = &content.tunables;
    let player = state.players[idx].id;
    let (slot, removed) = remove_plain(state, idx, def, count);
    let mut golden = state.new_instance(def, true, removed.len() as u32, content);
    let golden_id = golden.id;
    events.push(Event::GoldenMade { player, card: golden.id, def });
    // A golden card goes to the hand, for the player to place when they
    // choose. Only when the hand has no room does it take the board slot one
    // of its copies left, so the card is never lost.
    let room = state.players[idx].hand.len() < t.hand_size as usize;
    golden.parts.iter_mut().for_each(|part| part.reward_due = true);
    insert(state, idx, if room { None } else { slot }, golden);

    for effect in &content.card(def).golden_effects {
        match effect {
            Effect::GrantOnGolden { card } => {
                if let Some(granted) = content.card_id(card) {
                    let card = state.new_instance(granted, false, 1, content);
                    events.push(Event::CardGained { player, card: card.id, def: granted });
                    state.players[idx].hand.push(card);
                }
            }
            Effect::GoldenHeal { owner_div, others_div } => {
                state.golden_healer_made = true;
                for other in 0..state.players.len() {
                    if !state.players[other].alive {
                        continue;
                    }
                    let div = if other == idx { *owner_div } else { *others_div };
                    let amount = state.players[other].max_hp / div.max(1);
                    heal(state, other, amount, events);
                }
            }
            _ => {}
        }
    }

    // Does nothing while the card is in hand.
    offer_due_rewards(state, idx, golden_id, env, events);
}
