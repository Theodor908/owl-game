//! Marco Polo. Silk Road: once a turn, for gold, a random card in his shop
//! is swapped for a random one a tier above his tavern level, which he may
//! then buy as usual. At the top level a card below the top tier is swapped
//! for a top-tier one; if every card is already top tier, one of them is
//! swapped for a different top-tier card.

use super::HeroAbility;
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::shop;
use crate::state::GameState;

pub struct MarcoPolo;

impl HeroAbility for MarcoPolo {
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if *action != AbilityAction::SilkRoad {
            return Err(RuleError::NoSuchAbility);
        }
        let content = env.content;
        let turn = state.turn;
        let cost = hero.param("cost");
        let p = &state.players[idx];
        if p.hero_state.used_turn == turn || p.shop.is_empty() {
            return Err(RuleError::AbilityUnavailable);
        }
        if p.gold < cost {
            return Err(RuleError::NotEnoughGold);
        }
        let top = content.tunables.max_tavern_level;
        let target = (p.tavern + 1).min(top);
        // Below the top level any card may go. At the top, a card that is
        // not yet top tier goes first; only if there is none, any card.
        let mut candidates: Vec<usize> =
            (0..p.shop.len()).filter(|&i| p.tavern < top || content.card(p.shop[i]).tier < top).collect();
        if candidates.is_empty() {
            candidates = (0..p.shop.len()).collect();
        }
        let slot = candidates[env.rng.below(candidates.len() as u32) as usize];
        let old = p.shop[slot];
        // The new card is drawn before the old one goes back, so it is a different card.
        let Some(new) = shop::draw(state, env, |card| card.in_shop && card.tier == target) else {
            return Err(RuleError::AbilityUnavailable);
        };
        state.return_def(old, 1, content);
        let p = &mut state.players[idx];
        p.shop[slot] = new;
        p.gold -= cost;
        p.hero_state.used_turn = turn;
        p.acted_this_shop = true;
        events.push(Event::AbilityUsed { player: p.id });
        Ok(())
    }
}
