//! William Shakespeare. To Be or Not To Be costs all of the gold of the round:
//! with many players left he skips the next battle, with few he receives
//! skulls. While he is in the lobby, skulls turn up in every player's shop.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::state::GameState;

pub struct Shakespeare;

impl HeroAbility for Shakespeare {
    fn on_shop_roll(&self, hero: &HeroDef, state: &mut GameState, idx: usize, env: &mut Env<'_>) {
        let skull = env.content.skull;
        let p = &mut state.players[idx];
        if !p.extras.contains(&skull) && env.rng.percent(hero.param("skull_chance")) {
            p.extras.push(skull);
        }
    }

    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if *action != AbilityAction::ToBeOrNotToBe {
            return Err(RuleError::NoSuchAbility);
        }
        once_a_round(state, idx)?;
        // It costs the whole gold of the round, so it must be the first action.
        if state.players[idx].acted_this_shop {
            return Err(RuleError::AbilityUnavailable);
        }
        let content = env.content;
        let many_left = state.alive_count() > hero.param("skip_above") as usize;
        // Skipping is limited per game, so it cannot be used to coast to a placement.
        if many_left && state.players[idx].hero_state.uses >= hero.param("skips") {
            return Err(RuleError::NoCharges);
        }
        state.players[idx].gold = 0;
        state.players[idx].acted_this_shop = true;
        state.players[idx].hero_state.used_turn = state.turn;
        if many_left {
            // Not to be: skip the next battle and do nothing else this round.
            state.players[idx].hero_state.uses += 1;
            state.players[idx].skip_battle = true;
            state.players[idx].locked_out = true;
        } else {
            // To be: no skip, skulls instead.
            for _ in 0..hero.param("skulls") {
                if state.players[idx].hand.len() >= content.tunables.hand_size as usize {
                    break;
                }
                let card = state.new_instance(content.skull, false, 1, content);
                let player = state.players[idx].id;
                events.push(Event::CardGained { player, card: card.id, def: content.skull });
                state.players[idx].hand.push(card);
            }
        }
        Ok(())
    }
}
