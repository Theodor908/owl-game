//! Properties that must hold in every lobby, checked by playing whole matches
//! with random commands, legal and illegal alike.

use owl_core::state::Fighter;
use owl_core::{
    AbilityAction, CardDefId, CardInstanceId, Category, Command, Content, Env, Event, GameState, Help, HeroId,
    LobbyRng, Phase, PlayerId, QuestionId, SyntheticQuestions, apply,
};

fn content() -> Content {
    Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads")
}

/// Every pooled card is always somewhere: in the pool, on offer, or held.
/// Only a used potion ever leaves the game.
fn assert_pool_is_conserved(state: &GameState, content: &Content) {
    let mut held = vec![0u32; content.cards.len()];
    for p in &state.players {
        for &def in p.shop.iter().chain(&p.extras).chain(p.rewards.iter().flatten()) {
            held[usize::from(def.0)] += 1;
        }
        for part in p.cards().flat_map(|c| &c.parts) {
            held[usize::from(part.def.0)] += part.copies;
        }
    }
    for (i, card) in content.cards.iter().enumerate() {
        if card.is_token() {
            continue;
        }
        let total = state.pool[i] + held[i];
        if CardDefId(i as u16) == content.potion {
            assert!(total <= card.copies, "potions appeared from nowhere");
        } else {
            assert_eq!(total, card.copies, "{} copies were lost or duplicated", card.id);
        }
    }
}

fn assert_invariants(state: &GameState, content: &Content) {
    let t = &content.tunables;
    for p in &state.players {
        assert!(p.hp <= p.max_hp, "HP above starting HP");
        assert!(p.board.len() <= t.board_slots as usize, "board over its slot limit");
        assert!(p.tavern >= 1 && p.tavern <= t.max_tavern_level);
        assert!(p.alive || p.placement.is_some(), "an eliminated player has a placement");
    }
    assert_pool_is_conserved(state, content);
}

fn random_card(state: &GameState, player: usize, rng: &mut LobbyRng) -> CardInstanceId {
    let cards: Vec<CardInstanceId> = state.players[player].cards().map(|c| c.id).collect();
    if cards.is_empty() { CardInstanceId(u32::MAX) } else { cards[rng.below(cards.len() as u32) as usize] }
}

fn random_shop_command(state: &GameState, idx: usize, rng: &mut LobbyRng) -> Command {
    let player = PlayerId(idx as u8);
    let p = &state.players[idx];
    let card = random_card(state, idx, rng);
    let hand_card = p.hand.first().map_or(CardInstanceId(u32::MAX), |c| c.id);
    match rng.below(20) {
        0..=4 => Command::Buy { player, index: rng.below(7) as usize },
        5..=8 => Command::Place { player, card: hand_card },
        9 => Command::Sell { player, card },
        10 => Command::Reroll { player },
        11 => Command::UpgradeTavern { player },
        12 => Command::ToggleFreeze { player },
        13 => Command::UseCard { player, card },
        14 => Command::BuyExtra { player, index: 0 },
        15 => Command::PickReward { player, index: rng.below(4) as usize },
        16 => Command::ChooseEndgameEffect { player, card },
        _ => {
            let def = p.cards().next().map_or(CardDefId(0), |c| c.parts[0].def);
            let question = p.last_battle.iter().flat_map(|b| &b.questions).next().map_or(QuestionId(0), |q| q.id);
            let options = &p.hero_state.house_options;
            let action = match rng.below(8) {
                0 => AbilityAction::Rewrite,
                1 => AbilityAction::DarwinMerge { def },
                2 => AbilityAction::Combine { a: card, b: random_card(state, idx, rng) },
                3 => AbilityAction::Record { question },
                4 => AbilityAction::Ubermensch,
                5 if options.len() >= 2 => AbilityAction::ChooseHouse { first: options[0], second: options[1] },
                5 => AbilityAction::ChooseHouse { first: Category(0), second: Category(0) },
                // Rarely, so Shakespeare does not skip every battle.
                6 if rng.percent(10) => AbilityAction::ToBeOrNotToBe,
                _ => AbilityAction::Ubermensch,
            };
            Command::UseAbility { player, action }
        }
    }
}

struct Outcome {
    state: GameState,
    events: Vec<Event>,
}

/// Plays one whole match with random commands and checks the invariants after every one.
fn play(content: &Content, players: usize, seed: u64) -> Outcome {
    let mut rng = LobbyRng::from_seed(seed);
    let mut chooser = LobbyRng::from_seed(seed ^ 0xABCD);
    let mut questions = SyntheticQuestions::default();
    let mut state = GameState::new(players, content, &mut rng).expect("enough heroes");
    let mut env = Env { content, rng: &mut rng, questions: &mut questions };
    let mut events = Vec::new();

    // Applies a command; a rejected one must leave the state untouched.
    let mut send = |state: &mut GameState, command: Command, events: &mut Vec<Event>| {
        let before = state.clone();
        match apply(state, command.clone(), &mut env) {
            Ok(new) => events.extend(new),
            Err(error) => assert_eq!(*state, before, "{command:?} was rejected ({error}) but changed the state"),
        }
        assert_invariants(state, content);
    };

    for i in 0..players {
        // Any hero at all: one outside the player's offer must be refused.
        let hero = HeroId(chooser.below(content.heroes.len() as u32) as u8);
        send(&mut state, Command::PickHero { player: PlayerId(i as u8), hero }, &mut events);
        let picked = state.players[i].hero;
        assert!(picked.is_none() || state.players[i].hero_offer.contains(&hero));
        // Some players pick from their offer; the rest are given a hero at match start.
        if chooser.chance(1, 2) {
            let hero = state.players[i].hero_offer[chooser.below(2) as usize];
            send(&mut state, Command::PickHero { player: PlayerId(i as u8), hero }, &mut events);
        }
    }
    send(&mut state, Command::StartMatch, &mut events);

    let mut steps = 0;
    while state.phase != Phase::Finished {
        steps += 1;
        assert!(steps < 20_000 && state.turn < 200, "the lobby did not terminate");
        match state.phase {
            Phase::Shop => {
                for idx in 0..players {
                    for _ in 0..12 {
                        let command = random_shop_command(&state, idx, &mut chooser);
                        send(&mut state, command, &mut events);
                    }
                }
                send(&mut state, Command::EndShop, &mut events);
            }
            Phase::Battle => {
                let waiting = |f: &Fighter| f.player.filter(|_| f.locked.is_none());
                let pending: Vec<(usize, PlayerId, u8)> = state
                    .battles
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| !b.done)
                    .flat_map(|(i, b)| {
                        let right = b.plan[b.current].question.correct;
                        b.fighters.iter().filter_map(waiting).map(move |p| (i, p, right))
                    })
                    .collect();
                for (battle, player, right) in pending {
                    if state.phase != Phase::Battle {
                        break;
                    }
                    match chooser.below(10) {
                        0 => send(&mut state, Command::QuestionTimeout { battle }, &mut events),
                        1 => {
                            let help = [Help::SecondOpinion, Help::FlamingSword, Help::Oracle][chooser.below(3) as usize];
                            send(&mut state, Command::UseHelp { player, help }, &mut events);
                        }
                        n => {
                            let choice = if n < 6 { Some(right) } else { Some(chooser.below(4) as u8) };
                            send(&mut state, Command::SubmitAnswer { player, choice }, &mut events);
                        }
                    }
                }
            }
            Phase::Endgame => {
                let endgame = state.endgame.clone().expect("endgame state exists");
                let right = endgame.question.expect("a question is open").correct;
                for player in endgame.players {
                    if state.phase != Phase::Endgame {
                        break;
                    }
                    let choice = if chooser.percent(50) { Some(right) } else { None };
                    send(&mut state, Command::SubmitAnswer { player, choice }, &mut events);
                }
                if state.phase == Phase::Endgame && chooser.percent(20) {
                    send(&mut state, Command::QuestionTimeout { battle: 0 }, &mut events);
                }
            }
            Phase::HeroSelect | Phase::Finished => unreachable!("handled above"),
        }
    }
    Outcome { state, events }
}

#[test]
fn every_lobby_terminates_with_one_winner_and_distinct_placements() {
    let content = content();
    for seed in 0..40 {
        let players = 2 + (seed % 5) as usize;
        let outcome = play(&content, players, seed);
        let mut placements: Vec<u32> =
            outcome.state.players.iter().map(|p| p.placement.expect("everyone is placed")).collect();
        placements.sort_unstable();
        assert_eq!(placements, (1..=players as u32).collect::<Vec<_>>(), "seed {seed}");
        // Nobody but the winner is left standing. The winner is not standing
        // either when the last players died together in a regular battle; the
        // engine then names the best placed of them the winner.
        let standing: Vec<_> = outcome.state.players.iter().filter(|p| p.alive).collect();
        assert!(standing.len() <= 1 && standing.iter().all(|p| p.placement == Some(1)), "seed {seed}");
        assert!(matches!(outcome.events.last(), Some(Event::MatchFinished { .. })));
    }
}

#[test]
fn the_same_seed_and_commands_replay_to_the_same_match() {
    let content = content();
    for seed in [3, 17, 99] {
        let first = play(&content, 6, seed);
        let second = play(&content, 6, seed);
        assert_eq!(first.state, second.state);
        assert_eq!(first.events, second.events);
    }
}

#[test]
fn different_seeds_give_different_matches() {
    let content = content();
    assert_ne!(play(&content, 6, 1).events, play(&content, 6, 2).events);
}

#[test]
fn everyone_meets_everyone_before_any_pair_repeats() {
    let content = content();
    // Many seeds: a greedy pairing passes on some and corners itself on others.
    for seed in 0..200 {
        let mut rng = LobbyRng::from_seed(seed);
        let mut questions = SyntheticQuestions::default();
        let mut state = GameState::new(6, &content, &mut rng).expect("enough heroes");
        let mut env = Env { content: &content, rng: &mut rng, questions: &mut questions };
        apply(&mut state, Command::StartMatch, &mut env).unwrap();

        let mut met = std::collections::BTreeSet::new();
        for _ in 0..5 {
            let events = apply(&mut state, Command::EndShop, &mut env).unwrap();
            for event in events {
                if let Event::BattleStarted { a, b: Some(b), .. } = event {
                    assert!(met.insert((a, b)), "seed {seed}: a pair repeated before everyone had met");
                }
            }
            // Nobody answers, so every battle is a 0-0 tie and nobody dies.
            while state.phase == Phase::Battle {
                let open = state.battles.iter().position(|b| !b.done).expect("a battle is open");
                apply(&mut state, Command::QuestionTimeout { battle: open }, &mut env).unwrap();
            }
        }
        assert_eq!(met.len(), 15, "six players make fifteen pairs in five rounds");
    }
}
