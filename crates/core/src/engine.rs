//! The single entry point: one command in, the events it caused out.

use crate::api::{Command, Event, RuleError};
use crate::content::Content;
use crate::question::QuestionSource;
use crate::rng::LobbyRng;
use crate::state::{GameState, Phase};
use crate::{battle, endgame, heroes, shop};

/// Everything the engine needs besides the state itself.
pub struct Env<'a> {
    pub content: &'a Content,
    pub rng: &'a mut LobbyRng,
    pub questions: &'a mut dyn QuestionSource,
}

/// Applies one command. On error the state is unchanged.
pub fn apply(state: &mut GameState, command: Command, env: &mut Env<'_>) -> Result<Vec<Event>, RuleError> {
    let mut events = Vec::new();
    let ev = &mut events;
    match command {
        Command::PickHero { player, hero } => heroes::pick_hero(state, player, hero, env, ev)?,
        Command::StartMatch => heroes::start_match(state, env, ev)?,

        Command::Buy { player, index } => shop::buy(state, player, index, false, env, ev)?,
        Command::BuyExtra { player, index } => shop::buy(state, player, index, true, env, ev)?,
        Command::Sell { player, card } => shop::sell(state, player, card, env, ev)?,
        Command::Place { player, card } => shop::place(state, player, card, env, ev)?,
        Command::UseCard { player, card } => shop::use_card(state, player, card, env, ev)?,
        Command::Reroll { player } => shop::reroll(state, player, env, ev)?,
        Command::ToggleFreeze { player } => shop::toggle_freeze(state, player)?,
        Command::UpgradeTavern { player } => shop::upgrade(state, player, env, ev)?,
        Command::PickReward { player, index } => shop::pick_reward(state, player, index, env, ev)?,
        Command::UseAbility { player, action } => heroes::use_ability(state, player, &action, env, ev)?,
        Command::ChooseEndgameEffect { player, card } => shop::choose_endgame(state, player, card, env)?,
        Command::EndShop => {
            if state.phase != Phase::Shop {
                return Err(RuleError::WrongPhase);
            }
            battle::start_battles(state, env, ev);
        }

        Command::UseHelp { player, help } => match state.phase {
            Phase::Battle => battle::use_help(state, player, help, env, ev)?,
            Phase::Endgame => endgame::use_help(state, player, help, env, ev)?,
            _ => return Err(RuleError::WrongPhase),
        },
        Command::SubmitAnswer { player, choice } => match state.phase {
            Phase::Battle => battle::submit(state, player, choice, env, ev)?,
            Phase::Endgame => endgame::submit(state, player, choice, env, ev)?,
            _ => return Err(RuleError::WrongPhase),
        },
        Command::QuestionTimeout { battle } => match state.phase {
            Phase::Battle => battle::timeout(state, battle, env, ev)?,
            Phase::Endgame => endgame::timeout(state, env, ev)?,
            _ => return Err(RuleError::WrongPhase),
        },
    }
    Ok(events)
}
