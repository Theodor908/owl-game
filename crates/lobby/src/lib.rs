//! Drives one lobby: phase deadlines, bots, per-player views and the match log.
//!
//! This crate knows nothing about networks or runtimes. The host (the online
//! server, or the offline extension inside the app) tells it the current time
//! on every call and forwards player commands; the lobby tells the engine
//! when a deadline has passed. That keeps it testable with a fake clock.

pub mod bank;
pub mod bot;
pub mod placeholder;
pub mod view;

use std::collections::VecDeque;
use std::sync::Arc;

use owl_core::{
    Command, Content, Env, Event, GameState, LobbyRng, Phase, PlayerId, QuestionSource, RuleError, apply,
};
use thiserror::Error;

use bot::Bot;
pub use view::{Outgoing, PlayerView};
use view::{RECENT_BATTLES, RecentBattle, Spectated};

/// Milliseconds on the host's clock. Only differences matter.
pub type Millis = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LobbyError {
    #[error(transparent)]
    Rule(#[from] RuleError),
    #[error("that command belongs to another player")]
    NotYours,
    #[error("only the host can send that command")]
    HostOnly,
    #[error("the question is not open yet")]
    TooEarly,
    #[error("the deadline has passed")]
    TooLate,
}

/// Who sits in a seat. Bots send commands exactly like a human would.
pub enum Seat {
    Human,
    Bot(Box<Bot>),
}

/// Timing of the question a battle is currently on.
#[derive(Debug, Clone, Default)]
struct QuestionClock {
    players: [Option<PlayerId>; 2],
    asked: u32,
    open: bool,
    opens_at: Millis,
    deadline: [Millis; 2],
}

pub struct Lobby {
    content: Arc<Content>,
    state: GameState,
    rng: LobbyRng,
    questions: Box<dyn QuestionSource + Send>,
    seats: Vec<Seat>,
    outbox: Vec<Vec<Outgoing>>,
    log: Vec<Command>,
    /// Deadline of hero select or of the shop phase.
    phase_deadline: Millis,
    clocks: Vec<QuestionClock>,
    in_endgame: bool,
    /// When each bot will answer the question it is on.
    think_at: Vec<Millis>,
    /// The last turn each bot did its shopping in.
    shopped: Vec<u32>,
    /// Bot thinking times only. Never touches the rules, so replays ignore it.
    pacing: LobbyRng,
    /// Each player's last battles, oldest first.
    recent: Vec<Vec<RecentBattle>>,
    /// The time of the last call, so a view knows how old a snapshot is.
    now: Millis,
    /// The living players at past moments, oldest first, for the eliminated to watch.
    watched: VecDeque<(Millis, Vec<Spectated>)>,
    /// Length of the log when the last snapshot was taken.
    watched_at_log: usize,
}

/// The player a command acts for, or `None` for commands only the host may send.
fn owner(command: &Command) -> Option<PlayerId> {
    match *command {
        Command::StartMatch | Command::EndShop | Command::QuestionTimeout { .. } => None,
        Command::PickHero { player, .. }
        | Command::Buy { player, .. }
        | Command::BuyExtra { player, .. }
        | Command::Sell { player, .. }
        | Command::Place { player, .. }
        | Command::UseCard { player, .. }
        | Command::Reroll { player }
        | Command::ToggleFreeze { player }
        | Command::UpgradeTavern { player }
        | Command::PickReward { player, .. }
        | Command::UseAbility { player, .. }
        | Command::ChooseEndgameEffect { player, .. }
        | Command::UseHelp { player, .. }
        | Command::SubmitAnswer { player, .. } => Some(player),
    }
}

impl Lobby {
    /// Opens a lobby in hero select, one seat per player, and deals every
    /// player their hero offer. Fails if there are too few heroes for the seats.
    pub fn new(
        content: Arc<Content>,
        seats: Vec<Seat>,
        seed: u64,
        questions: Box<dyn QuestionSource + Send>,
        now: Millis,
    ) -> Result<Self, LobbyError> {
        let players = seats.len();
        let phase_deadline = now + Millis::from(content.tunables.hero_select_seconds) * 1000;
        let mut rng = LobbyRng::from_seed(seed);
        let state = GameState::new(players, &content, &mut rng)?;
        Ok(Self {
            state,
            rng,
            pacing: LobbyRng::from_seed(seed ^ 0x0B07),
            content,
            questions,
            seats,
            outbox: vec![Vec::new(); players],
            log: Vec::new(),
            phase_deadline,
            clocks: Vec::new(),
            in_endgame: false,
            think_at: vec![0; players],
            shopped: vec![0; players],
            recent: vec![Vec::new(); players],
            now,
            watched: VecDeque::new(),
            watched_at_log: usize::MAX,
        })
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn is_finished(&self) -> bool {
        self.state.phase == Phase::Finished
    }

    /// Every accepted command so far, in order. With the seed, the content
    /// and the same question source, this replays the match exactly.
    pub fn log(&self) -> &[Command] {
        &self.log
    }

    /// Takes the events waiting for one player, already filtered to what
    /// that player is allowed to see.
    pub fn drain(&mut self, player: PlayerId) -> Vec<Outgoing> {
        self.outbox.get_mut(player.index()).map(std::mem::take).unwrap_or_default()
    }

    /// A full snapshot of what one player may see, for joining and reconnecting.
    pub fn view(&self, player: PlayerId) -> PlayerView {
        let clock = self.clocks.iter().find(|c| c.open && c.players.contains(&Some(player)));
        let timing = clock.map(|c| {
            let side = usize::from(c.players[1] == Some(player));
            (c.opens_at, c.deadline[side])
        });
        let phase_deadline = matches!(self.state.phase, Phase::HeroSelect | Phase::Shop).then_some(self.phase_deadline);
        view::player_view(&self.state, &self.content, player, timing, phase_deadline, &self.recent, self.spectate(player))
    }

    /// What an eliminated player watches: the newest snapshot that is at
    /// least the spectating delay old. Nothing for the living, and nothing
    /// once the match is over.
    fn spectate(&self, player: PlayerId) -> Vec<Spectated> {
        let viewer = &self.state.players[player.index()];
        if viewer.alive || self.is_finished() {
            return Vec::new();
        }
        let delay = Millis::from(self.content.tunables.spectate_delay_ms);
        self.watched.iter().rev().find(|(at, _)| at + delay <= self.now).map(|(_, seen)| seen.clone()).unwrap_or_default()
    }

    /// Keeps the snapshots the eliminated watch: a new one whenever the match
    /// has moved on, and no more old ones than the delay needs.
    fn record_for_spectators(&mut self, now: Millis) {
        self.now = now;
        let someone_out = self.state.players.iter().any(|p| !p.alive);
        if !someone_out || self.is_finished() {
            self.watched.clear();
            self.watched_at_log = usize::MAX;
            return;
        }
        if self.watched_at_log != self.log.len() {
            self.watched_at_log = self.log.len();
            self.watched.push_back((now, view::spectated(&self.state)));
        }
        // Only the newest snapshot that is already old enough is still needed.
        let delay = Millis::from(self.content.tunables.spectate_delay_ms);
        while self.watched.len() > 1 && self.watched[1].0 + delay <= now {
            self.watched.pop_front();
        }
    }

    /// A command from a player. Rejected unless it is theirs to send and, for
    /// answers, arrives while their question is open.
    pub fn handle(&mut self, now: Millis, from: PlayerId, command: Command) -> Result<(), LobbyError> {
        match owner(&command) {
            None => return Err(LobbyError::HostOnly),
            Some(player) if player != from => return Err(LobbyError::NotYours),
            Some(_) => {}
        }
        if matches!(command, Command::SubmitAnswer { .. } | Command::UseHelp { .. })
            && let Some(clock) = self.clocks.iter().find(|c| c.open && c.players.contains(&Some(from)))
        {
            let side = usize::from(clock.players[1] == Some(from));
            let grace = Millis::from(self.content.tunables.answer_grace_ms);
            if now < clock.opens_at {
                return Err(LobbyError::TooEarly);
            }
            if now > clock.deadline[side] + grace {
                return Err(LobbyError::TooLate);
            }
        }
        self.apply(now, command)?;
        self.record_for_spectators(now);
        Ok(())
    }

    fn apply(&mut self, now: Millis, command: Command) -> Result<(), RuleError> {
        let mut env = Env { content: &self.content, rng: &mut self.rng, questions: self.questions.as_mut() };
        let events = apply(&mut self.state, command.clone(), &mut env)?;
        self.log.push(command);
        self.absorb(now, events);
        Ok(())
    }

    /// Advances the lobby to `now`: bots act and passed deadlines fire. The
    /// host calls this regularly; calling it more often changes nothing.
    pub fn tick(&mut self, now: Millis) {
        // Each pass handles what is due; a pass that changes nothing ends the tick.
        for _ in 0..1000 {
            let before = self.log.len();
            self.run_bots(now);
            self.fire_deadlines(now);
            if self.log.len() == before || self.is_finished() {
                break;
            }
        }
        self.record_for_spectators(now);
    }

    fn run_bots(&mut self, now: Millis) {
        let phase = self.state.phase;
        let turn = self.state.turn;
        for seat in 0..self.seats.len() {
            let Seat::Bot(bot) = &mut self.seats[seat] else { continue };
            let mut env = Env { content: &self.content, rng: &mut self.rng, questions: self.questions.as_mut() };
            match phase {
                Phase::HeroSelect if self.state.players[seat].hero.is_none() => bot.pick_hero(&mut self.state, &mut env),
                Phase::Shop if self.shopped[seat] != turn => {
                    self.shopped[seat] = turn;
                    bot.shop(&mut self.state, &mut env);
                }
                Phase::Battle | Phase::Endgame => {
                    let player = PlayerId(seat as u8);
                    let due = self
                        .clocks
                        .iter()
                        .any(|c| c.open && c.players.contains(&Some(player)) && now >= c.opens_at.max(self.think_at[seat]));
                    if due {
                        bot.answer(&mut self.state, &mut env);
                    }
                }
                _ => {}
            }
            let events = std::mem::take(&mut bot.events);
            self.log.append(&mut bot.accepted);
            self.absorb(now, events);
            if self.state.phase != phase {
                // The phase moved on; the next pass of the tick picks it up.
                return;
            }
        }
    }

    fn fire_deadlines(&mut self, now: Millis) {
        let grace = Millis::from(self.content.tunables.answer_grace_ms);
        let command = match self.state.phase {
            Phase::HeroSelect => {
                let all_picked = self.state.players.iter().all(|p| p.hero.is_some());
                (all_picked || now >= self.phase_deadline).then_some(Command::StartMatch)
            }
            Phase::Shop => (now >= self.phase_deadline).then_some(Command::EndShop),
            Phase::Battle | Phase::Endgame => self
                .clocks
                .iter()
                .position(|c| c.open && now > c.deadline[0].max(c.deadline[1]) + grace)
                .map(|battle| Command::QuestionTimeout { battle }),
            Phase::Finished => None,
        };
        if let Some(command) = command {
            // A host command the engine refuses would be a bug here, not a
            // player error; leaving the state alone is the safe response.
            let _ = self.apply(now, command);
        }
    }

    /// Updates the clocks from new events and files each event in the
    /// outbox of every player allowed to see it.
    fn absorb(&mut self, now: Millis, events: Vec<Event>) {
        let t = &self.content.tunables;
        for event in events {
            let mut timing = None;
            match event {
                Event::TurnStarted { .. } => {
                    self.phase_deadline = now + Millis::from(t.shop_seconds) * 1000;
                    timing = Some((now, self.phase_deadline));
                }
                Event::BattleStarted { battle, a, b, .. } => {
                    if battle == 0 {
                        self.clocks.clear();
                    }
                    self.clocks.push(QuestionClock { players: [Some(a), b], ..Default::default() });
                }
                Event::EndgameStarted { a, b } => {
                    self.in_endgame = true;
                    self.clocks = vec![QuestionClock { players: [Some(a), Some(b)], ..Default::default() }];
                }
                Event::QuestionShown { battle, time_a, time_b, .. } => {
                    if let Some(clock) = self.clocks.get_mut(battle) {
                        let pause = if clock.asked > 0 { t.answer_reveal_ms } else { 0 } + t.category_reveal_ms;
                        clock.asked += 1;
                        clock.open = true;
                        clock.opens_at = now + Millis::from(pause);
                        clock.deadline =
                            [time_a, time_b].map(|seconds| clock.opens_at + Millis::from(seconds) * 1000);
                        for player in clock.players.into_iter().flatten() {
                            // Bots take between 1.5 and 6 seconds to answer.
                            self.think_at[player.index()] = clock.opens_at + 1500 + Millis::from(self.pacing.below(4500));
                        }
                        timing = Some((clock.opens_at, clock.deadline[0].max(clock.deadline[1])));
                    }
                }
                Event::BattleScored { battle, a_correct, b_correct } => {
                    if let Some(clock) = self.clocks.get_mut(battle) {
                        clock.open = false;
                        // The endgame is one long battle; only regular ones are listed.
                        if !self.in_endgame {
                            let correct = [a_correct, b_correct];
                            for side in 0..2 {
                                let Some(player) = clock.players[side] else { continue };
                                let list = &mut self.recent[player.index()];
                                list.push(RecentBattle {
                                    opponent: clock.players[1 - side],
                                    correct: correct[side],
                                    opponent_correct: correct[1 - side],
                                });
                                if list.len() > RECENT_BATTLES {
                                    list.remove(0);
                                }
                            }
                        }
                    }
                }
                Event::MatchFinished { .. } => self.clocks.iter_mut().for_each(|c| c.open = false),
                _ => {}
            }

            let battle_players = |battle: usize| self.clocks.get(battle).map(|c| c.players).unwrap_or_default();
            let recipients = view::audience(&event, self.in_endgame, battle_players, |player| {
                self.clocks.iter().find(|c| c.players.contains(&Some(player))).map(|c| c.players).unwrap_or_default()
            });
            for seat in 0..self.outbox.len() {
                let player = PlayerId(seat as u8);
                if recipients.includes(player) {
                    let (opens_at, deadline) = match (&event, timing) {
                        // Each player gets their own answer deadline.
                        (Event::QuestionShown { battle, .. }, Some((opens_at, latest))) => {
                            let clock = &self.clocks[*battle];
                            let own = clock.players.iter().position(|&p| p == Some(player)).map(|side| clock.deadline[side]);
                            (Some(opens_at), Some(own.unwrap_or(latest)))
                        }
                        (_, Some((opens_at, deadline))) => (Some(opens_at), Some(deadline)),
                        (_, None) => (None, None),
                    };
                    self.outbox[seat].push(Outgoing { event: event.clone(), opens_at, deadline });
                }
            }
        }
    }
}

/// Replays a match log from the start and returns the final state. With the
/// same content, seed and question source it reproduces the match exactly.
pub fn replay(
    content: &Content,
    players: usize,
    seed: u64,
    questions: &mut dyn QuestionSource,
    log: &[Command],
) -> Result<GameState, RuleError> {
    let mut rng = LobbyRng::from_seed(seed);
    let mut state = GameState::new(players, content, &mut rng)?;
    let mut env = Env { content, rng: &mut rng, questions };
    for command in log {
        apply(&mut state, command.clone(), &mut env)?;
    }
    Ok(state)
}
