//! Goethe. Prometheus: during his battle his opponent's
//! battle passives do not work. For gold, once a round, his category bonus
//! is raised for the next battle.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::state::{GameState, Player};
use crate::stats::BattleStats;

pub struct Goethe;

impl HeroAbility for Goethe {
    fn modify_stats(&self, hero: &HeroDef, player: &Player, stats: &mut BattleStats) {
        // The result of an active, so silence does not touch it.
        if player.hero_state.boosted {
            stats.weights[usize::from(hero.category.0)] += hero.param("boost");
        }
    }

    fn silences_opponent(&self, _hero: &HeroDef) -> bool {
        true
    }

    fn on_battle_end(&self, _hero: &HeroDef, player: &mut Player) {
        player.hero_state.boosted = false;
    }

    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        _env: &mut Env<'_>,
        _events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if *action != AbilityAction::PoetryBoost {
            return Err(RuleError::NoSuchAbility);
        }
        once_a_round(state, idx)?;
        let turn = state.turn;
        let cost = hero.param("cost");
        let p = &mut state.players[idx];
        if p.hero_state.boosted {
            return Err(RuleError::AlreadyUsed);
        }
        if p.gold < cost {
            return Err(RuleError::NotEnoughGold);
        }
        p.gold -= cost;
        p.hero_state.boosted = true;
        p.hero_state.used_turn = turn;
        p.acted_this_shop = true;
        Ok(())
    }
}
