//! Folding a player's hero and board into the numbers one battle uses.
//!
//! The snapshot is taken once, at battle start. Cards in hand and cards
//! disabled by a skull contribute nothing.

use serde::{Deserialize, Serialize};

use crate::content::{Content, Effect};
use crate::heroes;
use crate::ids::{CardDefId, Category};
use crate::state::Player;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleStats {
    pub tavern: u32,
    /// Percent chance per category for the player's weighted questions. Sums to at most 100.
    pub weights: Vec<u32>,
    pub weight_total: u32,
    pub split_bonus: u32,
    pub time_bonus: u32,
    pub damage_on_win: u32,
    pub multiplier_percent: u32,
    pub extra_on_loss: u32,
    pub streak_per_win: u32,
    pub streak_max: u32,
    pub perfect_double: bool,
    pub reflect: u32,
    pub reflect_pierce: u32,
    pub armor_pierce: u32,
    /// Share of the opponent's armor that does not count, in percent (Marie Curie).
    pub armor_pierce_percent: u32,
    /// Damage added on a win for each of the player's own correct answers, and its cap (Pheidippides).
    pub damage_per_correct: u32,
    pub damage_per_correct_max: u32,
    pub damage_cap: Option<u32>,
    pub tie_breaker: bool,
    pub zero_zero: u32,
    /// Categories with an active category bonus, and the damage per correct answer.
    pub bonus_categories: Vec<(Category, u32)>,
    /// (minimum correct answers, gold) rules.
    pub gold_rules: Vec<(u32, u32)>,
    pub loss_shields: Vec<CardDefId>,
    pub second_opinion_uses: u32,
    pub oracle_uses: u32,
    pub sword_uses: u32,
    pub sword_removes: u32,
    pub hunts: bool,
    pub arrow_damage: u32,
    /// Difficulty steps this player's cards add to the questions of each
    /// category, positive is harder. Indexed by category.
    pub difficulty: Vec<i32>,
}

pub fn fold_stats(player: &Player, content: &Content) -> BattleStats {
    let t = &content.tunables;
    let categories = t.categories.len();
    let mut stats = BattleStats { tavern: player.tavern, weights: vec![0; categories], ..Default::default() };
    let mut tribe = vec![0u32; categories];
    let mut tribe_golden = vec![false; categories];
    // Difficulty steps from cards that touch every category, and from cards
    // that touch only the top category.
    let (mut shift_all, mut shift_top) = (0i32, 0i32);

    let hero = player.hero.map(|h| content.hero(h));
    if let Some(hero) = hero {
        stats.weights[usize::from(hero.category.0)] += hero.category_bonus;
    }

    for card in player.board.iter().filter(|c| !c.disabled) {
        for part in &card.parts {
            let def = content.card(part.def);
            if let Some(category) = def.category {
                let i = usize::from(category.0);
                tribe[i] += if part.golden { t.category_bonus_cards } else { 1 };
                tribe_golden[i] |= part.golden;
            }
            for effect in def.effects_for(part.golden) {
                match *effect {
                    Effect::CategoryWeight { percent } => {
                        if let Some(category) = def.category {
                            stats.weights[usize::from(category.0)] += percent;
                        }
                    }
                    Effect::AnswerTime { seconds } => stats.time_bonus += seconds,
                    Effect::DamageOnWin { amount } => stats.damage_on_win += amount,
                    Effect::Reflect { percent } => stats.reflect += percent,
                    Effect::TieBreaker => stats.tie_breaker = true,
                    Effect::ZeroZero { damage } => stats.zero_zero += damage,
                    Effect::GoldOnCorrect { min_correct, gold } => stats.gold_rules.push((min_correct, gold)),
                    Effect::ShieldOnLoss { ref card } => stats.loss_shields.extend(content.card_id(card)),
                    Effect::RemoveWrong { uses } => stats.second_opinion_uses += uses,
                    Effect::DamageMultiplier { percent } => stats.multiplier_percent += percent,
                    Effect::ExtraDamageOnLoss { amount } => stats.extra_on_loss += amount,
                    Effect::ReflectPierce { percent } => stats.reflect_pierce += percent,
                    Effect::ArmorPierce { amount } => stats.armor_pierce += amount,
                    Effect::StreakDamage { per_win, max } => {
                        stats.streak_per_win += per_win;
                        stats.streak_max += max;
                    }
                    Effect::SplitBonus { points } => stats.split_bonus += points,
                    Effect::PerfectDouble => stats.perfect_double = true,
                    Effect::Oracle { per_battle, .. } => stats.oracle_uses += per_battle,
                    Effect::AllCategories => tribe.iter_mut().for_each(|n| *n += 1),
                    Effect::DamageCap { amount } => {
                        stats.damage_cap = Some(stats.damage_cap.map_or(amount, |cap| cap.min(amount)));
                    }
                    Effect::HuntHealer => stats.hunts = true,
                    Effect::Arrow { damage } => stats.arrow_damage += damage,
                    Effect::DifficultyShift { steps, all } => {
                        *(if all { &mut shift_all } else { &mut shift_top }) += steps;
                    }
                    // Armor lives on the card instance. The rest act outside battle stats.
                    Effect::Armor { .. }
                    | Effect::EndgameFewerOptions
                    | Effect::EndgameTime { .. }
                    | Effect::EndgameShield
                    | Effect::HealPerTurn { .. }
                    | Effect::HealOnce { .. }
                    | Effect::Skull
                    | Effect::GrantOnGolden { .. }
                    | Effect::GoldenHeal { .. }
                    | Effect::GoldenReward => {}
                }
            }
        }
    }

    if let Some(hero) = hero {
        heroes::ability(hero.kind).modify_stats(hero, player, &mut stats);
    }

    // The top category is the one with the highest weight, hero and cards
    // together. Several that tie for the lead are all top; with no weights
    // there is none.
    let highest = stats.weights.iter().copied().max().unwrap_or(0);
    stats.difficulty = stats
        .weights
        .iter()
        .map(|&weight| shift_all + if highest > 0 && weight == highest { shift_top } else { 0 })
        .collect();

    // A player's percentages are capped at 100 in total; scale down if over.
    let sum: u32 = stats.weights.iter().sum();
    if sum > 100 {
        stats.weights.iter_mut().for_each(|w| *w = *w * 100 / sum);
    }
    stats.weight_total = sum.min(100);
    stats.time_bonus = stats.time_bonus.min(t.answer_time_bonus_cap);
    stats.reflect = stats.reflect.min(hero.map_or(t.reflect_cap, |h| h.reflect_cap(t)));
    stats.reflect_pierce = stats.reflect_pierce.min(100);
    stats.armor_pierce_percent = stats.armor_pierce_percent.min(100);
    stats.zero_zero = stats.zero_zero.min(t.zero_zero_cap);

    for (i, &count) in tribe.iter().enumerate() {
        if count >= t.category_bonus_cards {
            let damage = if tribe_golden[i] { t.category_bonus_damage_golden } else { t.category_bonus_damage };
            stats.bonus_categories.push((Category(i as u8), damage));
        }
    }
    stats
}
