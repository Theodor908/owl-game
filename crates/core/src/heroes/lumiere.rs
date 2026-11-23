//! The Lumiere brothers. Record: for a gold cost, a film question from the
//! last battle appears again in the next one.

use super::HeroAbility;
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::state::GameState;

pub struct Lumiere;

impl HeroAbility for Lumiere {
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        _env: &mut Env<'_>,
        _events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        let AbilityAction::Record { question } = *action else {
            return Err(RuleError::NoSuchAbility);
        };
        let p = &mut state.players[idx];
        let cost = hero.param("cost");
        if p.gold < cost {
            return Err(RuleError::NotEnoughGold);
        }
        if p.hero_state.recorded.is_some() {
            return Err(RuleError::AlreadyUsed);
        }
        let recorded = p
            .last_battle
            .iter()
            .flat_map(|b| &b.questions)
            .find(|q| q.id == question && q.category == hero.category)
            .copied()
            .ok_or(RuleError::AbilityUnavailable)?;
        p.gold -= cost;
        p.hero_state.recorded = Some(recorded);
        Ok(())
    }
}
