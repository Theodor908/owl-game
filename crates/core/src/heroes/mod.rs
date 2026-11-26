//! Hero abilities.
//!
//! Each hero lives in its own module and implements [`HeroAbility`]. The
//! engine calls the trait at fixed points and never names a hero; adding a
//! hero is one module, one line in [`ability`], and its tests.

mod beethoven;
mod benz;
mod cervantes;
mod cleopatra;
mod darwin;
mod goethe;
mod herodotus;
mod herophilus;
mod homer;
mod lumiere;
mod marco_polo;
mod marie_curie;
mod newton;
mod nietzsche;
mod no_ability;
mod pheidippides;
mod shakespeare;

use crate::api::{AbilityAction, Event, RuleError};
use crate::content::{Content, HeroDef, HeroKind, Tunables};
use crate::engine::Env;
use crate::ids::{CardDefId, Category, HeroId, PlayerId};
use crate::rng::LobbyRng;
use crate::shop;
use crate::state::{GameState, Phase, Player};
use crate::stats::BattleStats;

/// The points where a hero can change the game. Every method has a default
/// that does nothing, so a hero implements only what it uses.
pub trait HeroAbility: Sync {
    /// Adjusts the battle numbers of the player after cards are folded in.
    fn modify_stats(&self, _hero: &HeroDef, _player: &Player, _stats: &mut BattleStats) {}

    /// The hero's battle passive. Runs after [`HeroAbility::modify_stats`]
    /// and before the caps, and not at all when the opponent silences it.
    fn battle_passive(&self, _hero: &HeroDef, _player: &Player, _stats: &mut BattleStats) {}

    /// True if this hero's battle opponent loses their battle passive.
    fn silences_opponent(&self, _hero: &HeroDef) -> bool {
        false
    }

    /// Runs for the player at the end of each regular battle they fought.
    fn on_battle_end(&self, _hero: &HeroDef, _player: &mut Player) {}

    /// Runs each time any player's shop is filled while this hero is in the
    /// lobby. `idx` is the player whose shop it is, not necessarily the hero.
    fn on_shop_roll(&self, _hero: &HeroDef, _state: &mut GameState, _idx: usize, _env: &mut Env<'_>) {}

    /// Runs for every hero in the lobby after any player's successful buy,
    /// of a shop card or an extra. `buyer` is the player who bought `def`.
    fn on_any_buy(&self, _hero: &HeroDef, _state: &mut GameState, _buyer: usize, _def: CardDefId, _env: &mut Env<'_>) {}

    /// Runs when the player is about to be eliminated. Returns true if the hero survives.
    fn on_would_die(
        &self,
        _hero: &HeroDef,
        _state: &mut GameState,
        _idx: usize,
        _env: &mut Env<'_>,
        _events: &mut Vec<Event>,
    ) -> bool {
        false
    }

    /// How much of the gold left at the end of a turn the player keeps for
    /// the next one. `income` is what the new turn pays everyone.
    fn gold_kept(&self, _hero: &HeroDef, _leftover: u32, _income: u32, _t: &Tunables) -> u32 {
        0
    }

    /// A passive that carries into the endgame: the share of the opponent's
    /// armor that does not count against this hero's player, in percent.
    fn endgame_armor_pierce(&self, _hero: &HeroDef) -> u32 {
        0
    }

    /// What this hero adds to the endgame category draw: pairs of category
    /// and percent. By default the hero's own category bonus.
    fn endgame_bias(&self, hero: &HeroDef) -> Vec<(Category, u32)> {
        vec![(hero.category, hero.category_bonus)]
    }

    /// Runs an activated ability during the shop phase.
    fn use_ability(
        &self,
        _hero: &HeroDef,
        _state: &mut GameState,
        _idx: usize,
        _action: &AbilityAction,
        _env: &mut Env<'_>,
        _events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        Err(RuleError::NoSuchAbility)
    }
}

/// An ability with a limited number of uses per match works at most once in
/// each round. Refuses a second use in the turn of the last one; each hero
/// records that turn in `hero_state.used_turn` when its charge is spent.
pub(crate) fn once_a_round(state: &GameState, idx: usize) -> Result<(), RuleError> {
    if state.players[idx].hero_state.used_turn == state.turn {
        return Err(RuleError::UsedThisRound);
    }
    Ok(())
}

/// What the hero of player `idx` takes off an opponent's armor in the endgame, in percent.
pub fn endgame_armor_pierce(state: &GameState, content: &Content, idx: usize) -> u32 {
    state.players[idx].hero.map_or(0, |h| {
        let hero = content.hero(h);
        ability(hero.kind).endgame_armor_pierce(hero)
    })
}

/// What the player's hero adds to the endgame category draw.
pub fn endgame_bias(player: &Player, content: &Content) -> Vec<(Category, u32)> {
    player.hero.map_or_else(Vec::new, |h| {
        let hero = content.hero(h);
        ability(hero.kind).endgame_bias(hero)
    })
}

/// Lets every hero in the lobby react to a buy (Homer's royalties).
pub fn on_any_buy(state: &mut GameState, buyer: usize, def: CardDefId, env: &mut Env<'_>) {
    let present: Vec<_> = state.players.iter().filter_map(|p| p.hero).collect();
    for hero in present {
        let hero = env.content.hero(hero);
        ability(hero.kind).on_any_buy(hero, state, buyer, def, env);
    }
}

/// The registry: which module implements which hero.
pub fn ability(kind: HeroKind) -> &'static dyn HeroAbility {
    match kind {
        HeroKind::Herodotus => &herodotus::Herodotus,
        HeroKind::Darwin => &darwin::Darwin,
        HeroKind::Benz => &benz::Benz,
        HeroKind::Lumiere => &lumiere::Lumiere,
        HeroKind::Newton => &newton::Newton,
        HeroKind::Nietzsche => &nietzsche::Nietzsche,
        HeroKind::Shakespeare => &shakespeare::Shakespeare,
        HeroKind::Herophilus => &herophilus::Herophilus,
        HeroKind::MarcoPolo => &marco_polo::MarcoPolo,
        HeroKind::Pheidippides => &pheidippides::Pheidippides,
        HeroKind::Cleopatra => &cleopatra::Cleopatra,
        HeroKind::MarieCurie => &marie_curie::MarieCurie,
        HeroKind::Beethoven => &beethoven::Beethoven,
        HeroKind::Goethe => &goethe::Goethe,
        HeroKind::Cervantes => &cervantes::Cervantes,
        HeroKind::Homer => &homer::Homer,
        HeroKind::NoAbility => &no_ability::NoAbility,
    }
}

/// Deals every player a private offer of distinct heroes, drawn from the
/// whole list with the lobby's RNG. No hero is offered to two players, so
/// each hero still appears at most once per lobby.
pub fn deal_offers(players: usize, content: &Content, rng: &mut LobbyRng) -> Result<Vec<Vec<HeroId>>, RuleError> {
    let size = content.tunables.hero_offer_size as usize;
    let mut deck: Vec<HeroId> = (0..content.heroes.len() as u8).map(HeroId).collect();
    if size == 0 || deck.len() < players * size {
        return Err(RuleError::NotEnoughHeroes);
    }
    rng.shuffle(&mut deck);
    Ok(deck.chunks(size).take(players).map(<[HeroId]>::to_vec).collect())
}

/// A player picks a hero from their own offer, once.
pub fn pick_hero(
    state: &mut GameState,
    player: PlayerId,
    hero: HeroId,
    _env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    if state.phase != Phase::HeroSelect {
        return Err(RuleError::WrongPhase);
    }
    let p = state.players.get_mut(player.index()).ok_or(RuleError::UnknownPlayer)?;
    if p.hero.is_some() {
        return Err(RuleError::HeroAlreadyPicked);
    }
    if !p.hero_offer.contains(&hero) {
        return Err(RuleError::HeroNotOffered);
    }
    p.hero = Some(hero);
    events.push(Event::HeroPicked { player, hero });
    Ok(())
}

/// Ends hero select. Anyone who has not picked gets a random hero from their own offer.
pub fn start_match(state: &mut GameState, env: &mut Env<'_>, events: &mut Vec<Event>) -> Result<(), RuleError> {
    if state.phase != Phase::HeroSelect {
        return Err(RuleError::WrongPhase);
    }
    let content = env.content;
    if state.players.iter().any(|p| p.hero.is_none() && p.hero_offer.is_empty()) {
        return Err(RuleError::NotEnoughHeroes);
    }
    for p in &mut state.players {
        if p.hero.is_none() {
            let hero = p.hero_offer[env.rng.below(p.hero_offer.len() as u32) as usize];
            p.hero = Some(hero);
            events.push(Event::HeroPicked { player: p.id, hero });
        }
        let hp = p.hero.map_or(0, |h| content.hero(h).hp);
        p.hp = hp;
        p.max_hp = hp;
    }
    shop::start_turn(state, env, events);
    Ok(())
}

pub fn use_ability(
    state: &mut GameState,
    player: PlayerId,
    action: &AbilityAction,
    env: &mut Env<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let idx = shop::shopper(state, player)?;
    let hero = state.players[idx].hero.ok_or(RuleError::NoSuchAbility)?;
    let hero = env.content.hero(hero);
    ability(hero.kind).use_ability(hero, state, idx, action, env, events)?;
    events.push(Event::AbilityUsed { player });
    Ok(())
}

pub fn on_would_die(state: &mut GameState, idx: usize, env: &mut Env<'_>, events: &mut Vec<Event>) -> bool {
    let Some(hero) = state.players[idx].hero else { return false };
    let hero = env.content.hero(hero);
    ability(hero.kind).on_would_die(hero, state, idx, env, events)
}
