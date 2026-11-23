//! Charles Darwin. Evolutionary Shortcut: a limited number of times per game,
//! make a golden card from two copies instead of three. At most once a round.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::{Effect, HeroDef};
use crate::engine::Env;
use crate::shop;
use crate::state::GameState;

pub struct Darwin;

impl HeroAbility for Darwin {
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        let AbilityAction::DarwinMerge { def } = *action else {
            return Err(RuleError::NoSuchAbility);
        };
        let p = &state.players[idx];
        if p.hero_state.uses >= hero.param("uses") {
            return Err(RuleError::NoCharges);
        }
        once_a_round(state, idx)?;
        let card = env.content.cards.get(usize::from(def.0)).ok_or(RuleError::CardNotFound)?;
        let second_golden_healer =
            state.golden_healer_made && card.has(|e| matches!(e, Effect::HealPerTurn { .. }));
        if !card.can_golden || second_golden_healer {
            return Err(RuleError::NotUsable);
        }
        if p.cards().filter(|c| c.is_plain(def)).count() < 2 {
            return Err(RuleError::CardNotFound);
        }
        let turn = state.turn;
        let hero_state = &mut state.players[idx].hero_state;
        hero_state.uses += 1;
        hero_state.used_turn = turn;
        shop::make_golden(state, idx, def, 2, env, events);
        Ok(())
    }
}
