//! What each player is allowed to see.
//!
//! Hidden information is enforced here, on the host, before anything leaves
//! the lobby: a player's hand, shop and gold are private; an opponent's board
//! is visible only during their battle; and nothing sent before the reveal
//! says which answer is correct.
//!
//! Two things soften that for everyone's benefit. Each player carries a
//! public summary (leading category on the board, golden cards, win streak,
//! last two battles), which hints at a build without showing it. And a
//! player who has been eliminated may watch the others' boards and shops,
//! a set delay behind the match.

use owl_core::state::{CardInstance, Player};
use owl_core::{CardDefId, Category, Content, Difficulty, Event, GameState, HeroId, Phase, PlayerId, QuestionId};
use serde::Serialize;

use crate::Millis;

/// One event on its way to one player, with the times the client needs to
/// show a countdown. The host decides; the client only displays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outgoing {
    pub event: Event,
    /// When the question opens, or when the shop phase began.
    pub opens_at: Option<Millis>,
    pub deadline: Option<Millis>,
}

/// What everyone may know about a player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublicPlayer {
    pub id: PlayerId,
    pub hero: Option<HeroId>,
    pub hp: u32,
    pub max_hp: u32,
    pub armor: u32,
    pub tavern: u32,
    pub alive: bool,
    pub placement: Option<u32>,
    /// Battles won in a row.
    pub win_streak: u32,
    /// Golden cards on the board.
    pub goldens: u32,
    /// The category with the most cards on the board, or `None` when two or
    /// more share the lead or there are none.
    pub top_category: Option<Category>,
    /// How many cards the leading category (or each of the leaders) has.
    pub top_category_cards: u32,
    /// The last battles, oldest first; at most `RECENT_BATTLES`.
    pub recent: Vec<RecentBattle>,
}

/// How many past battles everyone can see for each player.
pub const RECENT_BATTLES: usize = 2;

/// One finished battle, from one player's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RecentBattle {
    /// `None` when the opponent was a ghost.
    pub opponent: Option<PlayerId>,
    pub correct: u32,
    pub opponent_correct: u32,
}

/// What an eliminated player may watch of a living one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Spectated {
    pub id: PlayerId,
    pub board: Vec<CardInstance>,
    pub shop: Vec<CardDefId>,
    pub gold: u32,
    pub tavern: u32,
    pub hand_size: usize,
    /// The battle this player is in, as they see it themselves. Like every
    /// battle view it never says which answer is correct.
    pub battle: Option<BattleView>,
}

/// The living players, as an eliminated player may watch them.
pub fn spectated(state: &GameState) -> Vec<Spectated> {
    state
        .players
        .iter()
        .filter(|p| p.alive)
        .map(|p| Spectated {
            id: p.id,
            board: p.board.clone(),
            shop: p.shop.clone(),
            gold: p.gold,
            tavern: p.tavern,
            hand_size: p.hand.len(),
            battle: battle_view(state, p.id, None),
        })
        .collect()
}

/// The category with the most cards on a board and how many it has. A tie
/// for the lead gives no category, only the count.
fn top_category(player: &Player, content: &Content) -> (Option<Category>, u32) {
    let mut counts = vec![0u32; content.tunables.categories.len()];
    for part in player.board.iter().flat_map(|card| &card.parts) {
        if let Some(category) = content.card(part.def).category {
            counts[usize::from(category.0)] += 1;
        }
    }
    let best = counts.iter().copied().max().unwrap_or(0);
    if best == 0 {
        return (None, 0);
    }
    let mut leaders = counts.iter().enumerate().filter(|&(_, &count)| count == best);
    match (leaders.next(), leaders.next()) {
        (Some((index, _)), None) => (Some(Category(index as u8)), best),
        _ => (None, best),
    }
}

/// A player's view of the battle they are in. It never contains the correct answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleView {
    /// `None` when fighting a ghost.
    pub opponent: Option<PlayerId>,
    pub ghost_of: Option<PlayerId>,
    pub opponent_board: Vec<CardInstance>,
    pub question_number: usize,
    pub question_count: usize,
    pub category: Category,
    /// How hard the open question is, shown beside its category.
    pub difficulty: Difficulty,
    pub question: QuestionId,
    /// Options removed for this player, as a bit mask.
    pub removed: u8,
    pub locked: bool,
    pub opponent_locked: bool,
    pub my_correct: u32,
    pub opponent_correct: u32,
    /// Uses left this battle: Second Opinion, Flaming Laser Sword, The Oracle.
    pub second_opinion_left: u32,
    pub sword_left: u32,
    pub oracle_left: u32,
    pub opens_at: Option<Millis>,
    pub deadline: Option<Millis>,
}

/// Everything one player may see: the snapshot sent on join and reconnect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerView {
    pub you: PlayerId,
    pub phase: Phase,
    pub turn: u32,
    /// The player's own full state: hand, board, shop, gold.
    pub me: Player,
    pub players: Vec<PublicPlayer>,
    pub battle: Option<BattleView>,
    /// Deadline of hero select or of the shop phase.
    pub phase_deadline: Option<Millis>,
    /// Gold for the next tavern level, or `None` at the highest level. Sent
    /// so the client never has to work a price out for itself.
    pub upgrade_cost: Option<u32>,
    /// The living players as they were a set delay ago. Empty unless this
    /// player has been eliminated and the match is still running.
    pub spectate: Vec<Spectated>,
}

pub fn player_view(
    state: &GameState,
    content: &Content,
    you: PlayerId,
    question_timing: Option<(Millis, Millis)>,
    phase_deadline: Option<Millis>,
    recent: &[Vec<RecentBattle>],
    spectate: Vec<Spectated>,
) -> PlayerView {
    let players = state
        .players
        .iter()
        .map(|p| {
            let (top_category, top_category_cards) = top_category(p, content);
            PublicPlayer {
                id: p.id,
                hero: p.hero,
                hp: p.hp,
                max_hp: p.max_hp,
                armor: p.armor(),
                tavern: p.tavern,
                alive: p.alive,
                placement: p.placement,
                win_streak: p.win_streak,
                goldens: p.board.iter().flat_map(|card| &card.parts).filter(|part| part.golden).count() as u32,
                top_category,
                top_category_cards,
                recent: recent.get(p.id.index()).cloned().unwrap_or_default(),
            }
        })
        .collect();
    PlayerView {
        you,
        phase: state.phase,
        turn: state.turn,
        me: state.players[you.index()].clone(),
        players,
        battle: battle_view(state, you, question_timing),
        phase_deadline,
        upgrade_cost: owl_core::shop::upgrade_cost(state, you.index(), content),
        spectate,
    }
}

fn battle_view(state: &GameState, you: PlayerId, timing: Option<(Millis, Millis)>) -> Option<BattleView> {
    let (opens_at, deadline) = timing.unzip();
    if let Some(endgame) = &state.endgame
        && state.phase == Phase::Endgame
    {
        let side = endgame.players.iter().position(|&p| p == you)?;
        let question = endgame.question?;
        let opponent = endgame.players[1 - side];
        return Some(BattleView {
            opponent: Some(opponent),
            ghost_of: None,
            opponent_board: state.players[opponent.index()].board.clone(),
            question_number: endgame.step as usize,
            question_count: 0,
            category: question.category,
            difficulty: question.difficulty,
            question: question.id,
            removed: endgame.removed[side],
            locked: endgame.locked[side].is_some(),
            opponent_locked: endgame.locked[1 - side].is_some(),
            my_correct: 0,
            opponent_correct: 0,
            second_opinion_left: 0,
            sword_left: 0,
            oracle_left: endgame.oracle_left[side],
            opens_at,
            deadline,
        });
    }
    if state.phase != Phase::Battle {
        return None;
    }
    let battle = state.battles.iter().find(|b| !b.done && b.fighters.iter().any(|f| f.player == Some(you)))?;
    let side = usize::from(battle.fighters[1].player == Some(you));
    let (mine, theirs) = (&battle.fighters[side], &battle.fighters[1 - side]);
    let question = battle.plan[battle.current].question;
    // A ghost shows the board it was copied from.
    let board_owner = theirs.player.or(theirs.ghost_of)?;
    Some(BattleView {
        opponent: theirs.player,
        ghost_of: theirs.ghost_of,
        opponent_board: state.players[board_owner.index()].board.clone(),
        question_number: battle.current + 1,
        question_count: battle.plan.len(),
        category: question.category,
        difficulty: question.difficulty,
        question: question.id,
        removed: mine.removed,
        locked: mine.locked.is_some(),
        opponent_locked: theirs.locked.is_some(),
        my_correct: mine.correct_count(),
        opponent_correct: theirs.correct_count(),
        second_opinion_left: mine.second_opinion_left,
        sword_left: mine.sword_left,
        oracle_left: mine.oracle_left,
        opens_at,
        deadline,
    })
}

/// Who may see an event.
pub enum Audience {
    Everyone,
    Players([Option<PlayerId>; 2]),
}

impl Audience {
    fn one(player: PlayerId) -> Self {
        Audience::Players([Some(player), None])
    }

    pub fn includes(&self, player: PlayerId) -> bool {
        match self {
            Audience::Everyone => true,
            Audience::Players(list) => list.contains(&Some(player)),
        }
    }
}

/// Decides who sees an event. `battle_players` gives the two sides of a
/// battle by index, `battle_of` the two sides of the battle a player is in.
/// The endgame is watched by the whole lobby.
pub fn audience(
    event: &Event,
    in_endgame: bool,
    battle_players: impl Fn(usize) -> [Option<PlayerId>; 2],
    battle_of: impl Fn(PlayerId) -> [Option<PlayerId>; 2],
) -> Audience {
    match *event {
        // What a player does in their own shop is theirs alone.
        Event::CardBought { player, .. }
        | Event::CardSold { player, .. }
        | Event::CardPlaced { player, .. }
        | Event::CardUsed { player, .. }
        | Event::CardGained { player, .. }
        | Event::ShopRolled { player }
        | Event::GoldenMade { player, .. }
        | Event::CardsCombined { player, .. }
        | Event::RewardOffered { player, .. }
        | Event::AbilityUsed { player }
        | Event::HouseOptionsShown { player, .. }
        | Event::OptionsRemoved { player, .. } => Audience::one(player),

        // A battle is seen by its two sides; the endgame by everyone.
        Event::BattleStarted { battle, .. }
        | Event::CategoryRevealed { battle, .. }
        | Event::QuestionShown { battle, .. }
        | Event::AnswerRevealed { battle, .. }
        | Event::BattleScored { battle, .. } => {
            if in_endgame { Audience::Everyone } else { Audience::Players(battle_players(battle)) }
        }
        // Boards are visible only to the two players fighting.
        Event::CardDisabled { player, .. } | Event::ShieldBroken { player, .. } => {
            if in_endgame { Audience::Everyone } else { Audience::Players(battle_of(player)) }
        }

        Event::HeroPicked { .. }
        | Event::TurnStarted { .. }
        | Event::TavernUpgraded { .. }
        | Event::Healed { .. }
        | Event::DamageDealt { .. }
        | Event::PlayerRevived { .. }
        | Event::PlayerEliminated { .. }
        | Event::EndgameStarted { .. }
        | Event::SharedVictory { .. }
        | Event::MatchFinished { .. } => Audience::Everyone,
    }
}
