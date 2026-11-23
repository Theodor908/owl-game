//! Herodotus, Father of History. Unlocks at the highest tavern level and
//! works once per game, in one of two ways: rewrite the last battle into a
//! win, or come back from death.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, DamageSource, Event, RuleError};
use crate::content::{CardKind, HeroDef};
use crate::damage::{self, base_damage, win_damage};
use crate::engine::Env;
use crate::ids::PlayerId;
use crate::shop;
use crate::state::{CardInstance, GameState};
use crate::stats::fold_stats;

pub struct Herodotus;

/// The top tavern level reached and the charge (`uses`, one per game) not yet spent.
fn unlocked(hero: &HeroDef, state: &GameState, idx: usize, env: &Env<'_>) -> bool {
    let p = &state.players[idx];
    p.tavern >= env.content.tunables.max_tavern_level && p.hero_state.uses < hero.param("uses")
}

impl HeroAbility for Herodotus {
    /// The rewrite: he gets back what he lost in his last battle and deals the
    /// most he could have dealt, as if he had won every question. Reflect
    /// applies as usual. The rewrite never kills: anyone it would kill is left
    /// at 1 HP with a shield for the next battle.
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if *action != AbilityAction::Rewrite {
            return Err(RuleError::NoSuchAbility);
        }
        let content = env.content;
        let t = &content.tunables;
        if !unlocked(hero, state, idx, env) {
            return Err(RuleError::AbilityUnavailable);
        }
        once_a_round(state, idx)?;
        let last = state.players[idx].last_battle.clone().ok_or(RuleError::AbilityUnavailable)?;
        let opponent = last
            .opponent
            .map(PlayerId::index)
            .filter(|&o| state.players[o].alive)
            .ok_or(RuleError::AbilityUnavailable)?;

        let turn = state.turn;
        state.players[idx].hero_state.uses += 1;
        state.players[idx].hero_state.used_turn = turn;
        shop::heal(state, idx, last.hp_lost, events);
        let mine = fold_stats(&state.players[idx], content);
        let theirs = fold_stats(&state.players[opponent], content);
        let questions = t.house_questions + t.weighted_questions;
        let base = base_damage(&mine, questions, 0, state.players[idx].win_streak, t);
        let amount = win_damage(&mine, &theirs, base, true, t);
        for i in [idx, opponent] {
            state.players[i].taken_this_battle = 0;
        }
        damage::deal(state, Some(idx), &mine, Some(opponent), &theirs, amount, DamageSource::Ability, content, events);
        for i in [idx, opponent] {
            let p = &mut state.players[i];
            if p.hp == 0 {
                p.hp = 1;
                p.temp_armor += t.herodotus_shield;
            }
        }
        Ok(())
    }

    /// The revival: if the ability is unused when he dies before the endgame,
    /// he comes back with half HP and no armor.
    fn on_would_die(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> bool {
        if !unlocked(hero, state, idx, env) {
            return false;
        }
        let content = env.content;
        let turn = state.turn;
        let p = &mut state.players[idx];
        p.hero_state.uses += 1;
        p.hero_state.used_turn = turn;
        p.hp = (p.max_hp / 2).max(1);
        p.temp_armor = 0;
        let is_shield = |c: &CardInstance| c.parts.iter().all(|part| content.card(part.def).kind == CardKind::Armor);
        let (shields, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut p.board).into_iter().partition(is_shield);
        p.board = kept;
        p.board.iter_mut().for_each(|c| c.armor_left = 0);
        let player = p.id;
        for shield in &shields {
            state.return_instance(shield, content);
        }
        events.push(Event::PlayerRevived { player });
        true
    }
}
