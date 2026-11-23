//! Balance simulator: runs many lobbies of bots through the rules engine and
//! reports how each kind of player places.
//!
//! Usage: `cargo run --release -p owl-sim -- [--lobbies N] [--seed S] [--players N] [--data DIR]`

use std::collections::BTreeMap;
use std::process::ExitCode;

use owl_lobby::bot::{Bot, KNOWLEDGE, Knowledge, STRATEGIES, Strategy};
use owl_core::{
    Command, Content, Env, Event, GameState, LobbyRng, Phase, PlayerId, RuleError, SyntheticQuestions, apply,
};

/// A lobby that runs this long is treated as stuck and reported.
const TURN_LIMIT: u32 = 80;
/// Rough seconds per phase, to turn turn counts into match length.
const SHOP_SECONDS: u32 = 30;
const QUESTION_SECONDS: u32 = 15;

#[derive(Default, Clone, Copy)]
struct Tally {
    games: u32,
    placement_sum: u32,
    wins: u32,
    top_half: u32,
}

impl Tally {
    fn add(&mut self, placement: u32, players: u32) {
        self.games += 1;
        self.placement_sum += placement;
        self.wins += u32::from(placement == 1);
        self.top_half += u32::from(placement <= players / 2);
    }

    fn row(&self, label: &str) -> String {
        let games = self.games.max(1) as f32;
        format!(
            "  {label:<28} {:>6}   {:>5.2}   {:>5.1}%   {:>5.1}%",
            self.games,
            self.placement_sum as f32 / games,
            100.0 * self.wins as f32 / games,
            100.0 * self.top_half as f32 / games,
        )
    }
}

#[derive(Default)]
struct Report {
    lobbies: u32,
    stuck: u32,
    turns: Vec<u32>,
    seconds: Vec<u32>,
    endgame_questions: u32,
    battles: u32,
    ties: u32,
    by_knowledge: BTreeMap<Knowledge, Tally>,
    by_strategy: BTreeMap<Strategy, Tally>,
    by_both: BTreeMap<(Knowledge, Strategy), Tally>,
    by_hero: BTreeMap<String, Tally>,
}

struct Args {
    lobbies: u32,
    seed: u64,
    players: usize,
    data: String,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { lobbies: 2000, seed: 1, players: 6, data: "data".into() };
    let mut input = std::env::args().skip(1);
    while let Some(flag) = input.next() {
        let value = input.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let bad = |_| format!("bad value for {flag}: {value}");
        match flag.as_str() {
            "--lobbies" => args.lobbies = value.parse().map_err(bad)?,
            "--seed" => args.seed = value.parse().map_err(bad)?,
            "--players" => args.players = value.parse().map_err(bad)?,
            "--data" => args.data = value,
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    if !(2..=6).contains(&args.players) {
        return Err("--players must be between 2 and 6".into());
    }
    Ok(args)
}

fn run_lobby(content: &Content, players: usize, seed: u64, report: &mut Report) -> Result<(), RuleError> {
    let mut rng = LobbyRng::from_seed(seed);
    let mut setup = LobbyRng::from_seed(seed ^ 0x5EED);
    let mut questions = SyntheticQuestions::default();
    let mut state = GameState::new(players, content, &mut rng)?;
    let mut env = Env { content, rng: &mut rng, questions: &mut questions };

    let mut bots: Vec<Bot> = (0..players)
        .map(|i| {
            let knowledge = KNOWLEDGE[setup.below(KNOWLEDGE.len() as u32) as usize];
            let strategy = STRATEGIES[setup.below(STRATEGIES.len() as u32) as usize];
            Bot::new(PlayerId(i as u8), knowledge, strategy, content, setup.next_u64())
        })
        .collect();
    for bot in &mut bots {
        bot.pick_hero(&mut state, &mut env);
    }
    let mut events = apply(&mut state, Command::StartMatch, &mut env)?;

    let mut endgame_questions = 0;
    while state.phase != Phase::Finished && state.turn <= TURN_LIMIT {
        match state.phase {
            Phase::Shop => {
                for bot in &mut bots {
                    bot.shop(&mut state, &mut env);
                }
                events.extend(apply(&mut state, Command::EndShop, &mut env).expect("shop phase"));
            }
            Phase::Battle | Phase::Endgame => {
                endgame_questions += u32::from(state.phase == Phase::Endgame);
                for bot in &mut bots {
                    bot.answer(&mut state, &mut env);
                }
            }
            Phase::HeroSelect | Phase::Finished => break,
        }
        for bot in &mut bots {
            events.append(&mut bot.events);
        }
        for event in events.drain(..) {
            if let Event::BattleScored { a_correct, b_correct, .. } = event {
                report.battles += 1;
                report.ties += u32::from(a_correct == b_correct);
            }
        }
    }

    report.lobbies += 1;
    if state.phase != Phase::Finished {
        report.stuck += 1;
        return Ok(());
    }
    let questions_per_battle = content.tunables.house_questions + content.tunables.weighted_questions;
    report.turns.push(state.turn);
    report.endgame_questions += endgame_questions;
    report.seconds.push(
        state.turn * (SHOP_SECONDS + questions_per_battle * QUESTION_SECONDS) + endgame_questions * QUESTION_SECONDS,
    );
    for (bot, player) in bots.iter().zip(&state.players) {
        let placement = player.placement.unwrap_or(players as u32);
        let n = players as u32;
        report.by_knowledge.entry(bot.knowledge).or_default().add(placement, n);
        report.by_strategy.entry(bot.strategy).or_default().add(placement, n);
        report.by_both.entry((bot.knowledge, bot.strategy)).or_default().add(placement, n);
        if let Some(hero) = player.hero {
            report.by_hero.entry(content.hero(hero).name.clone()).or_default().add(placement, n);
        }
    }
    Ok(())
}

fn print_table<K: std::fmt::Debug>(title: &str, rows: &BTreeMap<K, Tally>) {
    println!("\n{title}");
    println!("  {:<28} {:>6}   {:>5}   {:>6}   {:>6}", "", "games", "place", "win", "top half");
    for (key, tally) in rows {
        println!("{}", tally.row(&format!("{key:?}").replace('"', "")));
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let content = match Content::load(&args.data) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let mut report = Report::default();
    for lobby in 0..args.lobbies {
        if let Err(error) = run_lobby(&content, args.players, args.seed.wrapping_add(u64::from(lobby)), &mut report) {
            eprintln!("cannot run a lobby: {error}");
            return ExitCode::FAILURE;
        }
    }

    let finished = report.turns.len().max(1) as f32;
    let mut turns = report.turns.clone();
    turns.sort_unstable();
    let pick = |v: &[u32], q: f32| v.get(((v.len() as f32 - 1.0) * q) as usize).copied().unwrap_or(0);
    let mut seconds = report.seconds.clone();
    seconds.sort_unstable();
    println!("Lobbies: {} ({} players each), stuck: {}", report.lobbies, args.players, report.stuck);
    println!(
        "Turns before the endgame: mean {:.1}, median {}, 10th-90th percentile {}-{}",
        turns.iter().sum::<u32>() as f32 / finished,
        pick(&turns, 0.5),
        pick(&turns, 0.1),
        pick(&turns, 0.9)
    );
    println!("Endgame questions per match: mean {:.1}", report.endgame_questions as f32 / finished);
    println!(
        "Estimated match length: median {:.0} min, 90th percentile {:.0} min ({}s shop, {}s per question)",
        pick(&seconds, 0.5) as f32 / 60.0,
        pick(&seconds, 0.9) as f32 / 60.0,
        SHOP_SECONDS,
        QUESTION_SECONDS
    );
    println!(
        "Battles: {}, ties: {:.1}%",
        report.battles,
        100.0 * report.ties as f32 / report.battles.max(1) as f32
    );

    print_table("By knowledge", &report.by_knowledge);
    print_table("By strategy", &report.by_strategy);
    print_table("By knowledge and strategy", &report.by_both);
    print_table("By hero", &report.by_hero);

    println!("\nArchetypes named in the project rules");
    let all_high = report.by_knowledge.get(&Knowledge::High).copied().unwrap_or_default();
    let low_tank = report.by_both.get(&(Knowledge::Low, Strategy::Tank)).copied().unwrap_or_default();
    let reflect = report.by_strategy.get(&Strategy::Reflect).copied().unwrap_or_default();
    println!("{}", all_high.row("High knowledge"));
    println!("{}", low_tank.row("Low knowledge, defensive"));
    println!("{}", reflect.row("Reflect"));
    ExitCode::SUCCESS
}
