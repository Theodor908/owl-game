//! Ludwig van Beethoven. Frenzied Notes: paid in gold over several rounds,
//! it unlocks Notes in his own shop. Each Note placed moves him to his next
//! phase, in turn Syncope, Passionate Composer and Can't Hear the Critics.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::state::{GameState, Player};
use crate::stats::BattleStats;

/// The data id of the card that moves him on.
const NOTE: &str = "note";

pub struct Beethoven;

impl HeroAbility for Beethoven {
    fn battle_passive(&self, hero: &HeroDef, player: &Player, stats: &mut BattleStats) {
        let Some(phase) = player.hero_state.stage.checked_sub(1) else { return };
        match phase % 3 {
            // Syncope: damage to the opponent whatever the result.
            0 => stats.strike += hero.param("syncope"),
            // Passionate Composer: more of his own category.
            1 => stats.weights[usize::from(hero.category.0)] += hero.param("passion"),
            // Can't Hear the Critics: reflect, within the usual cap.
            _ => stats.reflect += hero.param("critics_reflect"),
        }
    }

    fn on_shop_roll(&self, hero: &HeroDef, state: &mut GameState, idx: usize, env: &mut Env<'_>) {
        let content = env.content;
        let Some(note) = content.card_id(NOTE) else { return };
        let p = &mut state.players[idx];
        let his_own = p.hero.is_some() && p.hero == content.hero_id(&hero.id);
        if his_own
            && p.hero_state.uses >= hero.param("payments")
            && !p.extras.contains(&note)
            && env.rng.percent(hero.param("note_chance"))
        {
            p.extras.push(note);
        }
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
        if *action != AbilityAction::FrenziedNotes {
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
