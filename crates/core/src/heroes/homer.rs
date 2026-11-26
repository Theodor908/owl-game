//! Homer. Bard of the Centuries: paid in gold over several rounds, it brings
//! Bards into every player's shop while he is alive, and every Bard another
//! player buys pays him a royalty. Lyrical Peak: in the endgame his bias is
//! Poetry and Music instead of his Poetry bonus alone.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::ids::{CardDefId, Category};
use crate::state::GameState;

/// The data id of the card his ability unlocks.
const BARD: &str = "bard";

pub struct Homer;

impl Homer {
    /// The index of Homer's own player, if he is still in the match.
    fn living(hero: &HeroDef, state: &GameState, env: &Env<'_>) -> Option<usize> {
        let id = env.content.hero_id(&hero.id)?;
        state.players.iter().position(|p| p.alive && p.hero == Some(id))
    }
}

impl HeroAbility for Homer {
    fn on_shop_roll(&self, hero: &HeroDef, state: &mut GameState, idx: usize, env: &mut Env<'_>) {
        let Some(bard) = env.content.card_id(BARD) else { return };
        let Some(homer) = Self::living(hero, state, env) else { return };
        let unlocked = state.players[homer].hero_state.uses >= hero.param("payments");
        let p = &mut state.players[idx];
        if unlocked && !p.extras.contains(&bard) && env.rng.percent(hero.param("bard_chance")) {
            p.extras.push(bard);
        }
    }

    fn on_any_buy(&self, hero: &HeroDef, state: &mut GameState, buyer: usize, def: CardDefId, env: &mut Env<'_>) {
        if env.content.card_id(BARD) != Some(def) {
            return;
        }
        match Self::living(hero, state, env) {
            Some(homer) if homer != buyer => state.players[homer].bonus_gold += hero.param("royalty"),
            _ => {}
        }
    }

    /// His own category and the second one his data names (Music), each at
    /// its own percent. Load checks that the second category is there.
    fn endgame_bias(&self, hero: &HeroDef) -> Vec<(Category, u32)> {
        let mut bias = vec![(hero.category, hero.param("endgame_poetry"))];
        bias.extend(hero.endgame_category.map(|c| (c, hero.param("endgame_music"))));
        bias
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
        if *action != AbilityAction::BardOfTheCenturies {
            return Err(RuleError::NoSuchAbility);
        }
        if state.players[idx].hero_state.uses >= hero.param("payments") {
            return Err(RuleError::NoCharges);
        }
        once_a_round(state, idx)?;
        let turn = state.turn;
        let cost = hero.param("cost");
        let p = &mut state.players[idx];
        if p.gold < cost {
            return Err(RuleError::NotEnoughGold);
        }
        p.gold -= cost;
        p.hero_state.uses += 1;
        p.hero_state.used_turn = turn;
        p.acted_this_shop = true;
        Ok(())
    }
}
