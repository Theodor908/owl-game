//! The endgame between the last two players: one question at a time, with
//! damage that rises by one each question.

use crate::heroes;
use crate::api::{DamageSource, Event, Help, RuleError};
use crate::battle::pick_wrong;
use crate::content::{Content, Effect};
use crate::damage::{Damage, take_damage};
use crate::engine::Env;
use crate::ids::{Category, PlayerId};
use crate::question::OPTIONS;
use crate::state::{EndgameEffect, EndgameState, GameState, Phase, Player};

/// The one endgame effect a player brings: their chosen card if it is still
/// on the board, otherwise the first endgame card on the board.
fn chosen_effect(player: &Player, content: &Content) -> Option<EndgameEffect> {
    let to_effect = |e: &Effect| match *e {
        Effect::EndgameFewerOptions => Some(EndgameEffect::FewerOptions),
        Effect::EndgameTime { seconds } => Some(EndgameEffect::Time(seconds)),
        Effect::EndgameShield => Some(EndgameEffect::Shield),
        _ => None,
    };
    let of_card = |c: &crate::state::CardInstance| c.effects(content).find_map(to_effect);
    let chosen = player.board.iter().find(|c| Some(c.id) == player.endgame_choice).and_then(of_card);
    chosen.or_else(|| player.board.iter().find_map(of_card))
}

pub fn start(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let alive: Vec<usize> = state.alive().collect();
    let pair = [alive[0], alive[1]];
    let oracle = |p: &Player| -> u32 {
        p.board
            .iter()
            .flat_map(|c| c.effects(content))
            .map(|e| if let Effect::Oracle { endgame_total, .. } = e { *endgame_total } else { 0 })
            .sum()
    };
    let endgame = EndgameState {
        players: pair.map(|i| PlayerId(i as u8)),
        step: 1,
        question: None,
        locked: [None; 2],
        removed: [0; 2],
        effects: pair.map(|i| chosen_effect(&state.players[i], content)),
        shield_used: [false; 2],
        oracle_left: pair.map(|i| oracle(&state.players[i])),
    };
    events.push(Event::EndgameStarted { a: endgame.players[0], b: endgame.players[1] });
    state.phase = Phase::Endgame;
    state.battles.clear();
    state.endgame = Some(endgame);
    open_question(state, env, events);
}

fn open_question(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let t = &content.tunables;
    let Some(eg) = state.endgame.as_mut() else { return };
    let pair = eg.players.map(PlayerId::index);

    // Random category, biased by both heroes' endgame bias (their category
    // bonus, or Homer's Lyrical Peak) added into one draw.
    let mut weights = vec![0; t.categories.len()];
    for idx in pair {
        for (category, percent) in crate::heroes::endgame_bias(&state.players[idx], content) {
            weights[usize::from(category.0)] += percent;
        }
    }
    let category: Category = crate::battle::roll_category(&weights, env.rng);
    // Regular cards have stopped working, so nothing moves the base difficulty.
    let difficulty = crate::battle::roll_difficulty(&t.difficulty_odds, env.rng);
    let question = env.questions.draw(category, difficulty, &state.players[pair[0]].seen, &state.players[pair[1]].seen, env.rng);
    for idx in pair {
        state.players[idx].seen.insert(question.id);
    }

    eg.question = Some(question);
    eg.locked = [None; 2];
    eg.removed = [0; 2];
    let mut time = [t.endgame_answer_time; 2];
    for side in 0..2 {
        match eg.effects[side] {
            Some(EndgameEffect::FewerOptions) => eg.removed[side] = pick_wrong(question.correct, 0, 1, env.rng),
            Some(EndgameEffect::Time(seconds)) => time[side] += seconds,
            _ => {}
        }
    }
    events.push(Event::CategoryRevealed { battle: 0, category, difficulty: question.difficulty });
    events.push(Event::QuestionShown { battle: 0, question: question.id, time_a: time[0], time_b: time[1] });
    for side in 0..2 {
        if eg.removed[side] != 0 {
            events.push(Event::OptionsRemoved { player: eg.players[side], removed_mask: eg.removed[side] });
        }
    }
}

fn side_of(state: &GameState, player: PlayerId) -> Result<usize, RuleError> {
    let eg = state.endgame.as_ref().ok_or(RuleError::WrongPhase)?;
    let side = eg.players.iter().position(|&p| p == player).ok_or(RuleError::NotInBattle)?;
    if eg.locked[side].is_some() {
        return Err(RuleError::AlreadyAnswered);
    }
    Ok(side)
}

pub fn use_help(
    state: &mut GameState,
    player: PlayerId,
    help: Help,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let side = side_of(state, player)?;
    // Regular cards stop working in the endgame; only The Oracle carries over.
    if help != Help::Oracle {
        return Err(RuleError::NotUsable);
    }
    let eg = state.endgame.as_mut().ok_or(RuleError::WrongPhase)?;
    if eg.oracle_left[side] == 0 {
        return Err(RuleError::NoCharges);
    }
    eg.oracle_left[side] -= 1;
    eg.locked[side] = eg.question.map(|q| Some(q.correct));
    reveal_if_ready(state, env, events);
    Ok(())
}

pub fn submit(
    state: &mut GameState,
    player: PlayerId,
    choice: Option<u8>,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let side = side_of(state, player)?;
    let eg = state.endgame.as_mut().ok_or(RuleError::WrongPhase)?;
    if let Some(option) = choice
        && (option >= OPTIONS || eg.removed[side] & (1 << option) != 0)
    {
        return Err(RuleError::InvalidAnswer);
    }
    eg.locked[side] = Some(choice);
    reveal_if_ready(state, env, events);
    Ok(())
}

pub fn timeout(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) -> Result<(), RuleError> {
    let eg = state.endgame.as_mut().ok_or(RuleError::WrongPhase)?;
    for locked in &mut eg.locked {
        locked.get_or_insert(None);
    }
    reveal_if_ready(state, env, events);
    Ok(())
}

fn reveal_if_ready(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) {
    let content = env.content;
    let Some(eg) = state.endgame.as_mut() else { return };
    let Some(question) = eg.question else { return };
    if eg.locked.iter().any(Option::is_none) {
        return;
    }
    let right = eg.locked.map(|l| l == Some(Some(question.correct)));
    let pair = eg.players.map(PlayerId::index);
    let step = eg.step;
    eg.step += 1;
    // The first hit on a Second Wind holder is ignored.
    let mut hit = [!right[0], !right[1]];
    for side in 0..2 {
        if hit[side] && eg.effects[side] == Some(EndgameEffect::Shield) && !eg.shield_used[side] {
            eg.shield_used[side] = true;
            hit[side] = false;
        }
    }
    events.push(Event::AnswerRevealed {
        battle: 0,
        correct_option: question.correct,
        a_correct: right[0],
        b_correct: right[1],
    });

    // Both correct: nothing. Both wrong: both take the current step.
    let before = pair.map(|i| state.players[i].hp + state.players[i].armor());
    for side in 0..2 {
        if hit[side] {
            // Hero passives marked as carrying into the endgame: the other
            // side's hero may make part of this player's armor not count.
            let armor_percent_ignored = heroes::endgame_armor_pierce(state, content, pair[1 - side]);
            let damage =
                Damage { amount: step, source: DamageSource::EndgameStep, armor_ignored: 0, armor_percent_ignored };
            take_damage(state, pair[side], damage, None, content, events);
        }
    }

    let dead = pair.map(|i| state.players[i].hp == 0);
    let winner = match dead {
        [false, false] => {
            open_question(state, env, events);
            return;
        }
        [true, false] => 1,
        [false, true] => 0,
        // Both died on the same question: whoever had more left wins. They
        // took the same hit, so that is whoever had more before it. With
        // equal HP both won, and the podium order is random.
        [true, true] => match before[0].cmp(&before[1]) {
            std::cmp::Ordering::Greater => 0,
            std::cmp::Ordering::Less => 1,
            std::cmp::Ordering::Equal => {
                state.shared_win = true;
                events.push(Event::SharedVictory { a: PlayerId(pair[0] as u8), b: PlayerId(pair[1] as u8) });
                env.rng.below(2) as usize
            }
        },
    };
    let loser = pair[1 - winner];
    state.players[loser].alive = false;
    state.players[loser].placement = Some(2);
    state.players[pair[winner]].placement = Some(1);
    state.phase = Phase::Finished;
    let winner = PlayerId(pair[winner] as u8);
    events.push(Event::PlayerEliminated { player: PlayerId(loser as u8), placement: 2 });
    events.push(Event::MatchFinished { winner });
}
