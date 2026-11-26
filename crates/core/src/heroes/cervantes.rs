//! Miguel de Cervantes. Don Quixote, a few times a game, puts a Knight and a
//! Windmill in his hand: the Knight strikes whatever the result, the Windmill
//! keeps all damage off him for a battle, and a Knight topples a Windmill.

use super::{once_a_round, HeroAbility};
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::state::GameState;

/// The data ids of the two cards he hands himself.
const CARDS: [&str; 2] = ["knight", "windmill"];

pub struct Cervantes;

impl HeroAbility for Cervantes {
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if *action != AbilityAction::DonQuixote {
            return Err(RuleError::NoSuchAbility);
        }
        if state.players[idx].hero_state.uses >= hero.param("uses") {
            return Err(RuleError::NoCharges);
        }
        once_a_round(state, idx)?;
        let content = env.content;
        let hand = state.players[idx].hand.len() + CARDS.len();
        if hand > content.tunables.hand_size as usize {
            return Err(RuleError::HandFull);
        }
        let [Some(knight), Some(windmill)] = CARDS.map(|id| content.card_id(id)) else {
            return Err(RuleError::AbilityUnavailable);
        };
        let player = state.players[idx].id;
        for def in [knight, windmill] {
            let card = state.new_instance(def, false, 1, content);
            events.push(Event::CardGained { player, card: card.id, def });
            state.players[idx].hand.push(card);
        }
        let turn = state.turn;
        let p = &mut state.players[idx];
        p.hero_state.uses += 1;
        p.hero_state.used_turn = turn;
        p.acted_this_shop = true;
        Ok(())
    }
}
