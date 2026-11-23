//! Friedrich Nietzsche. Ubermensch: for a gold cost, choose the categories of
//! the house questions in his next battle, a different one for each, from
//! random options.

use super::HeroAbility;
use crate::api::{AbilityAction, Event, RuleError};
use crate::content::HeroDef;
use crate::engine::Env;
use crate::ids::Category;
use crate::state::GameState;

pub struct Nietzsche;

impl HeroAbility for Nietzsche {
    fn use_ability(
        &self,
        hero: &HeroDef,
        state: &mut GameState,
        idx: usize,
        action: &AbilityAction,
        env: &mut Env<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        let p = &mut state.players[idx];
        match *action {
            AbilityAction::Ubermensch => {
                let cost = hero.param("cost");
                if p.gold < cost {
                    return Err(RuleError::NotEnoughGold);
                }
                if !p.hero_state.house_options.is_empty() || p.hero_state.house_choice.is_some() {
                    return Err(RuleError::AlreadyUsed);
                }
                p.gold -= cost;
                let mut options: Vec<Category> = (0..env.content.category_count() as u8).map(Category).collect();
                env.rng.shuffle(&mut options);
                options.truncate(hero.param("options") as usize);
                events.push(Event::HouseOptionsShown { player: p.id, options: options.clone() });
                p.hero_state.house_options = options;
                Ok(())
            }
            AbilityAction::ChooseHouse { first, second } => {
                let options = &p.hero_state.house_options;
                if first == second || !options.contains(&first) || !options.contains(&second) {
                    return Err(RuleError::BadIndex);
                }
                p.hero_state.house_options.clear();
                p.hero_state.house_choice = Some((first, second));
                Ok(())
            }
            _ => Err(RuleError::NoSuchAbility),
        }
    }
}
