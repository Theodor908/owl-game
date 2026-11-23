//! Who fights whom each turn.
//!
//! Pairings rotate so that everyone meets everyone. Among all ways to pair
//! the players, the engine picks one where the most recently repeated matchup
//! is as old as possible, and among those, one that leaves the most later
//! rounds of never-met pairs still possible. Without that look-ahead, six
//! players can be paired into a corner where the unmet pairs form two
//! triangles and no full round of new opponents exists.
//!
//! With an odd number of fighters, the one who met a ghost longest ago fights
//! a ghost.

use crate::rng::LobbyRng;
use crate::state::GameState;

type Matching = Vec<(usize, usize)>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Pairing {
    pub pairs: Matching,
    /// (player, the player the ghost is copied from).
    pub ghost: Option<(usize, usize)>,
}

pub fn make_pairs(state: &GameState, rng: &mut LobbyRng) -> Pairing {
    let mut fighters: Vec<usize> = state.alive().filter(|&i| !state.players[i].skip_battle).collect();
    rng.shuffle(&mut fighters);
    let mut pairing = Pairing::default();

    if fighters.len() % 2 == 1 {
        let pos = (0..fighters.len())
            .min_by_key(|&i| state.players[fighters[i]].last_ghost_turn)
            .expect("an odd count is at least one");
        let player = fighters.remove(pos);
        let sources: Vec<usize> = state.alive().filter(|&i| i != player).collect();
        if !sources.is_empty() {
            let source = sources[rng.below(sources.len() as u32) as usize];
            pairing.ghost = Some((player, source));
        }
    }

    let mut met: Vec<Vec<bool>> = state.last_met.iter().map(|row| row.iter().map(|&t| t > 0).collect()).collect();
    let all = matchings(&fighters);
    let best = all.iter().max_by_key(|matching| {
        let gaps = matching.iter().map(|&(a, b)| state.turn - state.last_met[a][b]);
        let freshest_repeat = gaps.clone().min().unwrap_or(0);
        mark(&mut met, matching, true);
        let rounds_left = fresh_rounds(&mut met, &all);
        // Undo only what this candidate added.
        for &(a, b) in matching.iter() {
            let was_met = state.last_met[a][b] > 0;
            met[a][b] = was_met;
            met[b][a] = was_met;
        }
        (freshest_repeat, rounds_left, gaps.sum::<u32>())
    });
    if let Some(pairs) = best {
        // Lower seat first, so side A of a battle is predictable for clients.
        pairing.pairs = pairs.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
        pairing.pairs.sort_unstable();
    }
    pairing
}

/// Every way to split `players` into pairs: at most 15 for six players.
fn matchings(players: &[usize]) -> Vec<Matching> {
    let Some((&first, rest)) = players.split_first() else { return vec![Vec::new()] };
    let mut all = Vec::new();
    for i in 0..rest.len() {
        let mut remaining = rest.to_vec();
        let partner = remaining.remove(i);
        for mut tail in matchings(&remaining) {
            tail.push((first, partner));
            all.push(tail);
        }
    }
    all
}

fn mark(met: &mut [Vec<bool>], matching: &[(usize, usize)], value: bool) {
    for &(a, b) in matching {
        met[a][b] = value;
        met[b][a] = value;
    }
}

/// How many more rounds can be played in which nobody meets an opponent they
/// have already met.
fn fresh_rounds(met: &mut [Vec<bool>], all: &[Matching]) -> u32 {
    let mut best = 0;
    for matching in all {
        if matching.is_empty() || matching.iter().any(|&(a, b)| met[a][b]) {
            continue;
        }
        mark(met, matching, true);
        best = best.max(1 + fresh_rounds(met, all));
        mark(met, matching, false);
    }
    best
}
