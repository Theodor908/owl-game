//! Working out damage, and the one place where a player takes it.
//!
//! Every point of damage in the game, from any source, goes through
//! [`take_damage`]. Nothing else lowers HP or armor.

use crate::api::{DamageSource, Event};
use crate::content::{CardKind, Content, Tunables};
use crate::state::GameState;
use crate::stats::BattleStats;

#[derive(Debug, Clone, Copy)]
pub struct Damage {
    pub amount: u32,
    pub source: DamageSource,
    /// How much of the receiver's armor this damage ignores (Siege Ram).
    pub armor_ignored: u32,
    /// The share of the receiver's armor that does not count against this
    /// damage, in percent (Marie Curie). Taken off first, rounding the armor
    /// that still counts down; `armor_ignored` comes off what is left.
    pub armor_percent_ignored: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageOutcome {
    /// The amount after the receiver's cap and before armor. Reflect is based on this.
    pub incoming: u32,
    pub armor_absorbed: u32,
    pub hp_lost: u32,
}

/// Damage before multipliers: answer margin + base + tavern level + flat bonuses.
pub fn base_damage(stats: &BattleStats, margin: u32, category_bonus: u32, streak: u32, t: &Tunables) -> u32 {
    let streak_bonus = (streak * stats.streak_per_win).min(stats.streak_max);
    margin + t.damage_base + stats.tavern + stats.damage_on_win + streak_bonus + category_bonus
}

/// Full damage a winner deals: base, then multipliers, then the loser's own
/// penalty, then the per-battle cap.
pub fn win_damage(
    winner: &BattleStats,
    loser: &BattleStats,
    base: u32,
    perfect: bool,
    t: &Tunables,
) -> u32 {
    let mut damage = base * (100 + winner.multiplier_percent) / 100;
    if winner.perfect_double && perfect {
        damage *= 2;
    }
    (damage + loser.extra_on_loss).min(t.max_battle_damage)
}

/// The reflect a holder has left after the opponent's reflect is netted off.
/// Equal values cancel; only the higher side keeps the difference.
pub fn net_reflect(holder: u32, opponent: u32) -> u32 {
    holder.saturating_sub(opponent)
}

/// Applies damage to a player: receiver cap, then armor, then HP.
/// Reflect and death are decided by the caller from the outcome.
pub fn take_damage(
    state: &mut GameState,
    idx: usize,
    damage: Damage,
    cap: Option<u32>,
    content: &Content,
    events: &mut Vec<Event>,
) -> DamageOutcome {
    let player = &mut state.players[idx];
    let mut amount = damage.amount;
    if let Some(cap) = cap {
        amount = amount.min(cap.saturating_sub(player.taken_this_battle));
    }
    player.taken_this_battle += amount;

    let counted = player.armor() * (100 - damage.armor_percent_ignored.min(100)) / 100;
    let usable_armor = counted.saturating_sub(damage.armor_ignored);
    let absorbed = amount.min(usable_armor);
    let mut left = absorbed;
    let from_temp = left.min(player.temp_armor);
    player.temp_armor -= from_temp;
    left -= from_temp;

    let mut broken = Vec::new();
    for card in player.board.iter_mut().filter(|c| !c.disabled) {
        let take = left.min(card.armor_left);
        card.armor_left -= take;
        left -= take;
        let is_shield = card.parts.iter().all(|p| content.card(p.def).kind == CardKind::Armor);
        if take > 0 && card.armor_left == 0 && is_shield {
            broken.push(card.id);
        }
    }

    let hp_lost = (amount - absorbed).min(player.hp);
    player.hp -= hp_lost;
    let id = player.id;

    for card_id in broken {
        if let Some(card) = state.players[idx].take_card(card_id) {
            state.return_instance(&card, content);
            events.push(Event::ShieldBroken { player: id, card: card_id });
        }
    }
    events.push(Event::DamageDealt {
        to: id,
        source: damage.source,
        amount,
        armor_absorbed: absorbed,
        hp_lost,
    });
    DamageOutcome { incoming: amount, armor_absorbed: absorbed, hp_lost }
}

/// Deals damage from one side of a battle to the other and handles reflect.
///
/// `to` is `None` for a ghost, whose damage is ignored. `from` is `None` when
/// the attacker is a ghost, which cannot be hit by reflect. Reflected damage
/// is a new, non-reflectable hit on the attacker, through their own armor.
#[allow(clippy::too_many_arguments)]
pub fn deal(
    state: &mut GameState,
    from: Option<usize>,
    from_stats: &BattleStats,
    to: Option<usize>,
    to_stats: &BattleStats,
    amount: u32,
    source: DamageSource,
    content: &Content,
    events: &mut Vec<Event>,
) {
    let Some(to) = to else { return };
    if amount == 0 {
        return;
    }
    let damage = Damage {
        amount,
        source,
        armor_ignored: from_stats.armor_pierce,
        armor_percent_ignored: from_stats.armor_pierce_percent,
    };
    let outcome = take_damage(state, to, damage, to_stats.damage_cap, content, events);

    let Some(from) = from else { return };
    let percent = net_reflect(to_stats.reflect, from_stats.reflect) * (100 - from_stats.reflect_pierce) / 100;
    let reflected = outcome.incoming * percent / 100;
    if reflected > 0 {
        let back = Damage { amount: reflected, source: DamageSource::Reflect, armor_ignored: 0, armor_percent_ignored: 0 };
        take_damage(state, from, back, from_stats.damage_cap, content, events);
    }
}
