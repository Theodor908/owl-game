//! Isaac Newton. Flaming Laser Sword: his shop sometimes offers the sword;
//! bought, it removes wrong answers on a set number of questions of his
//! choice, in whichever battles he likes. While he still holds unused
//! revelations the shop does not offer it again.

use super::HeroAbility;
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::{HeroDef, HeroKind};
use crate::engine::Env;
use crate::state::{GameState, Player};
use crate::stats::BattleStats;

pub struct Newton;

impl HeroAbility for Newton {
    fn modify_stats(&self, hero: &HeroDef, player: &Player, stats: &mut BattleStats) {
        // The revelations he has bought and not yet used (`hero_state.uses`).
        stats.sword_uses = player.hero_state.uses;
        stats.sword_removes = hero.param("removed");
    }

    fn on_shop_roll(&self, hero: &HeroDef, state: &mut GameState, idx: usize, env: &mut Env<'_>) {
        // Only his own shop offers it, at most once a turn, and never while he holds one.
        let turn = state.turn;
        let p = &mut state.players[idx];
        let his_own = p.hero.is_some_and(|h| env.content.hero(h).kind == HeroKind::Newton);
        if his_own
            && p.hero_state.uses == 0
            && p.hero_state.sword_offer_turn != turn
            && env.rng.percent(hero.param("chance"))
        {
            p.hero_state.sword_offer_turn = turn;
        }
    }

    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        _env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if *action != AbilityAction::BuySword {
            return Err(RuleError::NoSuchAbility);
        }
        let turn = state.turn;
        let p = &mut state.players[idx];
        if p.hero_state.sword_offer_turn != turn || p.hero_state.uses > 0 {
            return Err(RuleError::AbilityUnavailable);
        }
        let cost = hero.param("cost");
        if p.gold < cost {
            return Err(RuleError::NotEnoughGold);
        }
        p.gold -= cost;
        p.hero_state.uses = hero.param("questions");
        p.acted_this_shop = true;
        events.push(Event::AbilityUsed { player: p.id });
        Ok(())
    }
}
