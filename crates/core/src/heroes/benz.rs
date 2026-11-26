//! Karl Benz. Combines two cards into one that takes a single slot and keeps
//! the full effects of both. Limited uses per game, at most one a round.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::{Effect, HeroDef};
use crate::engine::Env;
use crate::ids::CardInstanceId;
use crate::shop;
use crate::state::GameState;

pub struct Benz;

impl HeroAbility for Benz {
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        let AbilityAction::Combine { a, b } = *action else {
            return Err(RuleError::NoSuchAbility);
        };
        let content = env.content;
        let p = &state.players[idx];
        if p.hero_state.uses >= hero.param("uses") {
            return Err(RuleError::NoCharges);
        }
        once_a_round(state, idx)?;
        if a == b {
            return Err(RuleError::NotUsable);
        }
        for id in [a, b] {
            let card = p.find_card(id).ok_or(RuleError::CardNotFound)?;
            // Skulls and one-battle cards vanish after a battle and a reward card
            // when it is placed, so they cannot be fused into a lasting card.
            if card.lasts_one_battle(content)
                || card.has_effect(content, |e| e.is_used_from_hand() || matches!(e, Effect::Skull | Effect::GoldenReward))
            {
                return Err(RuleError::NotUsable);
            }
        }

        // The combined card takes the place of the first one.
        let slot = p.board.iter().position(|c| c.id == a);
        let id = CardInstanceId(state.next_instance);
        let turn = state.turn;
        state.next_instance += 1;
        let p = &mut state.players[idx];
        let mut combined = p.take_card(a).ok_or(RuleError::CardNotFound)?;
        let other = p.take_card(b).ok_or(RuleError::CardNotFound)?;
        combined.id = id;
        combined.parts.extend(other.parts);
        combined.armor_left += other.armor_left;
        match slot {
            Some(i) => p.board.insert(i.min(p.board.len()), combined),
            None => p.hand.push(combined),
        }
        p.hero_state.uses += 1;
        p.hero_state.used_turn = turn;
        if p.endgame_choice == Some(a) || p.endgame_choice == Some(b) {
            p.endgame_choice = Some(id);
        }
        events.push(Event::CardsCombined { player: p.id, card: id });
        // A golden card combined onto the board has been placed: its reward is due now.
        shop::offer_due_rewards(state, idx, id, env, events);
        Ok(())
    }
}
