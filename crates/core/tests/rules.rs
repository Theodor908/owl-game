//! One test per game rule, driven through commands the way a host would.
//!
//! Tests read numbers from the real data files, so a balance change that
//! breaks a rule shows up here.

use owl_core::battle::{split, wanted_difficulty};
use owl_core::content::Effect;
use owl_core::damage::net_reflect;
use owl_core::state::Fighter;
use owl_core::stats::{BattleStats, fold_stats};
use owl_core::{
    AbilityAction, CardInstanceId, Category, Command, Content, DamageSource, Difficulty, Env, Event, GameState, Help,
    HeroId, LobbyRng, Phase, PlayerId, RuleError, SyntheticQuestions, apply,
};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

struct Lobby {
    content: Content,
    state: GameState,
    rng: LobbyRng,
    questions: SyntheticQuestions,
}

impl Lobby {
    /// A lobby in its first shop phase, one player per hero name. The test
    /// chooses the heroes, so the dealt offers are replaced by the hero asked for.
    fn new(heroes: &[&str]) -> Self {
        let content = load_content();
        let state = GameState::new(heroes.len(), &content, &mut LobbyRng::from_seed(0)).expect("enough heroes");
        let mut lobby = Self { content, state, rng: LobbyRng::from_seed(42), questions: SyntheticQuestions::default() };
        for (i, name) in heroes.iter().enumerate() {
            let hero = lobby.content.hero_id(name).expect("hero exists");
            lobby.state.players[i].hero_offer = vec![hero];
            lobby.apply(Command::PickHero { player: PlayerId(i as u8), hero }).expect("hero is offered");
        }
        lobby.apply(Command::StartMatch).expect("match starts");
        lobby
    }

    fn apply(&mut self, command: Command) -> Result<Vec<Event>, RuleError> {
        let mut env = Env { content: &self.content, rng: &mut self.rng, questions: &mut self.questions };
        apply(&mut self.state, command, &mut env)
    }

    fn card(&mut self, name: &str, golden: bool) -> owl_core::state::CardInstance {
        let def = self.content.card_id(name).expect("card exists");
        self.state.new_instance(def, golden, if golden { 3 } else { 1 }, &self.content)
    }

    /// Puts a card straight on a player's board.
    fn give(&mut self, player: usize, name: &str) -> CardInstanceId {
        let card = self.card(name, false);
        self.state.players[player].board.push(card);
        self.state.players[player].board.last().expect("just pushed").id
    }

    fn give_golden(&mut self, player: usize, name: &str) {
        let card = self.card(name, true);
        self.state.players[player].board.push(card);
    }

    fn give_hand(&mut self, player: usize, name: &str) -> CardInstanceId {
        let card = self.card(name, false);
        self.state.players[player].hand.push(card);
        self.state.players[player].hand.last().expect("just pushed").id
    }

    fn hp(&self, player: usize) -> u32 {
        self.state.players[player].hp
    }

    fn stats(&self, player: usize) -> BattleStats {
        fold_stats(&self.state.players[player], &self.content)
    }

    /// Ends the shop and plays the battles; `correct[i]` is how many of the
    /// first questions player `i` answers correctly.
    fn battle(&mut self, correct: &[u32]) -> Vec<Event> {
        let mut events = self.apply(Command::EndShop).expect("shop ends");
        let turn = self.state.turn;
        while self.state.phase == Phase::Battle && self.state.turn == turn {
            let waiting = |f: &Fighter| f.player.filter(|_| f.locked.is_none()).map(|p| (p, f.answers.len()));
            let pending: Vec<(PlayerId, usize, u8)> = self
                .state
                .battles
                .iter()
                .filter(|b| !b.done)
                .flat_map(|b| {
                    let right = b.plan[b.current].question.correct;
                    b.fighters.iter().filter_map(waiting).map(move |(p, answered)| (p, answered, right))
                })
                .collect();
            for (player, answered, right) in pending {
                let knows = correct[player.index()] as usize > answered;
                let choice = Some(if knows { right } else { (right + 1) % 4 });
                events.extend(self.apply(Command::SubmitAnswer { player, choice }).expect("answer accepted"));
            }
        }
        events
    }

    /// Answers the current endgame question for both players.
    fn endgame_answer(&mut self, right: [bool; 2]) -> Vec<Event> {
        let endgame = self.state.endgame.clone().expect("in the endgame");
        let correct = endgame.question.expect("a question is open").correct;
        let mut events = Vec::new();
        for side in 0..2 {
            let wrong = (0..4).find(|&o| o != correct && endgame.removed[side] & (1 << o) == 0).expect("a wrong option");
            let choice = Some(if right[side] { correct } else { wrong });
            events.extend(self.apply(Command::SubmitAnswer { player: endgame.players[side], choice }).expect("answer"));
        }
        events
    }
}

fn load_content() -> Content {
    Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads")
}

/// A lobby still in hero select, with its offers dealt from `seed`.
fn hero_select(players: usize, seed: u64) -> (Content, GameState, LobbyRng) {
    let content = load_content();
    let mut rng = LobbyRng::from_seed(seed);
    let state = GameState::new(players, &content, &mut rng).expect("enough heroes");
    (content, state, rng)
}

fn offers(state: &GameState) -> Vec<Vec<HeroId>> {
    state.players.iter().map(|p| p.hero_offer.clone()).collect()
}

fn damage_to(events: &[Event], player: PlayerId, wanted: DamageSource) -> Vec<(u32, u32, u32)> {
    events
        .iter()
        .filter_map(|e| match *e {
            Event::DamageDealt { to, source, amount, armor_absorbed, hp_lost } if to == player && source == wanted => {
                Some((amount, armor_absorbed, hp_lost))
            }
            _ => None,
        })
        .collect()
}

// ---- Content -------------------------------------------------------------

#[test]
fn content_loads_with_the_documented_card_pool() {
    let lobby = Lobby::new(&["benz", "darwin"]);
    let in_shop = lobby.content.cards.iter().filter(|c| c.in_shop).count();
    // 28 unique shop cards, 8 difficulty cards, and 3 category lines across every category.
    assert_eq!(lobby.content.tunables.categories.len(), 13);
    assert_eq!(in_shop, 28 + 8 + 3 * lobby.content.tunables.categories.len());
    // The 12 starting heroes, Beethoven, Goethe, Cervantes and Homer.
    assert_eq!(lobby.content.heroes.len(), 16);
}

#[test]
fn a_card_says_what_its_golden_version_does_exactly_when_it_can_be_golden() {
    let content = Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads");
    assert!(content.cards.iter().any(|card| card.can_golden));
    for card in &content.cards {
        assert_eq!(card.golden_text.is_some(), card.can_golden, "{}", card.id);
    }
}

#[test]
fn unique_cards_have_one_copy_and_cannot_be_golden() {
    let lobby = Lobby::new(&["benz", "darwin"]);
    for name in ["oracle", "second_opinion"].into_iter().chain(DIFFICULTY_CARDS.map(|card| card.0)) {
        let card = lobby.content.card(lobby.content.card_id(name).unwrap());
        assert_eq!(card.copies, 1, "{name}");
        assert!(!card.can_golden && card.golden_text.is_none(), "{name}");
        assert_eq!(lobby.state.pool[usize::from(lobby.content.card_id(name).unwrap().0)], 1, "{name}");
    }
}

/// The eight difficulty cards: id, tier, steps (positive is harder), and
/// whether the card touches every category or only the holder's top one.
const DIFFICULTY_CARDS: [(&str, u32, i32, bool); 8] = [
    ("primer", 3, -1, false),
    ("treatise", 3, 1, false),
    ("abridged_edition", 3, -2, false),
    ("magnum_opus", 3, 2, false),
    ("glossary", 2, -1, true),
    ("footnotes", 2, 1, true),
    ("pocket_guide", 2, -2, true),
    ("unabridged_edition", 2, 2, true),
];

#[test]
fn the_difficulty_cards_are_special_cards_of_tiers_two_and_three_at_the_normal_price() {
    let content = load_content();
    for (name, tier, steps, all) in DIFFICULTY_CARDS {
        let card = content.card(content.card_id(name).unwrap());
        assert_eq!((card.tier, card.cost, card.in_shop), (tier, content.tunables.card_cost, true), "{name}");
        assert_eq!(card.effects, vec![owl_core::Effect::DifficultyShift { steps, all }], "{name}");
        // Three of them never merge, since only one exists.
        assert_eq!((card.copies, card.can_golden, card.merges_into), (1, false, None), "{name}");
    }
}

// ---- Difficulty ------------------------------------------------------------

/// The difficulty a question of `category` gets from each base difficulty,
/// with the boards as they are: [from easy, from medium, from hard].
fn difficulties(lobby: &Lobby, category: u8) -> [Difficulty; 3] {
    let (a, b) = (lobby.stats(0), lobby.stats(1));
    Difficulty::ALL.map(|base| wanted_difficulty(base, Category(category), &a, &b))
}

#[test]
fn a_question_gets_a_base_difficulty_drawn_from_the_tunable_odds() {
    use Difficulty::{Easy, Hard, Medium};
    let content = load_content();
    assert_eq!(content.tunables.difficulty_odds.iter().sum::<u32>(), 100);
    let mut rng = LobbyRng::from_seed(5);
    let mut seen = [0u32; 3];
    for _ in 0..3000 {
        seen[owl_core::battle::roll_difficulty(&content.tunables.difficulty_odds, &mut rng) as usize] += 1;
    }
    for (count, odds) in seen.iter().zip(content.tunables.difficulty_odds) {
        assert!(count.abs_diff(odds * 30) < 150, "{seen:?} is not close to the odds");
    }
    // With no cards a question keeps its base difficulty.
    let lobby = Lobby::new(&["benz", "darwin"]);
    assert_eq!(difficulties(&lobby, 0), [Easy, Medium, Hard]);
}

#[test]
fn lower_and_raise_move_a_question_one_step_and_greatly_makes_it_always_easy_or_hard() {
    use Difficulty::{Easy, Hard, Medium};
    for (card, expected) in [
        ("glossary", [Easy, Easy, Medium]),
        ("footnotes", [Medium, Hard, Hard]),
        ("pocket_guide", [Easy, Easy, Easy]),
        ("unabridged_edition", [Hard, Hard, Hard]),
    ] {
        let mut lobby = Lobby::new(&["benz", "darwin"]);
        lobby.give(0, card);
        // Every category, the hero's own or not.
        for category in 0..lobby.content.category_count() as u8 {
            assert_eq!(difficulties(&lobby, category), expected, "{card}");
        }
        // In hand it does nothing.
        let mut lobby = Lobby::new(&["benz", "darwin"]);
        lobby.give_hand(0, card);
        assert_eq!(difficulties(&lobby, 0), [Easy, Medium, Hard], "{card} in hand");
    }
}

#[test]
fn difficulty_steps_from_both_boards_add_up_cancel_and_only_the_result_is_capped() {
    use Difficulty::{Easy, Hard, Medium};
    // A raise and a lower give no change.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "footnotes");
    lobby.give(1, "glossary");
    assert_eq!(difficulties(&lobby, 0), [Easy, Medium, Hard]);

    // Two raises give two steps, whichever board they are on.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "footnotes");
    lobby.give(1, "treatise");
    let darwin = lobby.content.hero(lobby.state.players[1].hero.unwrap()).category;
    assert_eq!(difficulties(&lobby, darwin.0), [Hard, Hard, Hard]);

    // "Greatly" counts as two steps: a lower against it leaves one step.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "unabridged_edition");
    lobby.give(1, "glossary");
    assert_eq!(difficulties(&lobby, 0), [Medium, Hard, Hard]);

    // Greatly raise against greatly lower: no change.
    lobby.give(1, "pocket_guide");
    lobby.state.players[1].board.remove(0);
    assert_eq!(difficulties(&lobby, 0), [Easy, Medium, Hard]);

    // Steps cancel before the cap: +2 +1 -2 is one step up, not "hard, then two down".
    lobby.give(0, "footnotes");
    assert_eq!(difficulties(&lobby, 0), [Medium, Hard, Hard]);

    // Three steps down stop at easy.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "pocket_guide");
    lobby.give(1, "glossary");
    assert_eq!(difficulties(&lobby, 0), [Easy, Easy, Easy]);
}

#[test]
fn a_top_category_card_touches_the_holders_highest_weight_and_every_category_tied_for_it() {
    use Difficulty::{Easy, Hard, Medium};
    let plain = [Easy, Medium, Hard];
    let raised = [Medium, Hard, Hard];
    // Herodotus: +25% History (category 0). Physics is category 4.
    let mut lobby = Lobby::new(&["herodotus", "darwin"]);
    let (history, physics, sport) = (0, 4, 9);
    assert_eq!(lobby.content.tunables.categories[usize::from(physics)], "physics");
    lobby.give(0, "treatise");
    assert_eq!(difficulties(&lobby, history), raised);
    assert_eq!(difficulties(&lobby, physics), plain);
    // The opponent's top category is not the holder's.
    let darwin = lobby.content.hero(lobby.state.players[1].hero.unwrap()).category;
    assert_eq!(difficulties(&lobby, darwin.0), plain);

    // 25% Physics from a card ties with the hero's 25% History: both are top.
    lobby.give(0, "master_physics");
    assert_eq!((lobby.stats(0).weights[0], lobby.stats(0).weights[4]), (25, 25));
    assert_eq!(difficulties(&lobby, history), raised);
    assert_eq!(difficulties(&lobby, physics), raised);
    assert_eq!(difficulties(&lobby, sport), plain);

    // 35% Physics leads alone.
    lobby.give(0, "apprentice_physics");
    assert_eq!(difficulties(&lobby, history), plain);
    assert_eq!(difficulties(&lobby, physics), raised);

    // Hero and cards count together: History back in the lead with 25 + 15.
    lobby.give(0, "scholar_history");
    assert_eq!(difficulties(&lobby, history), raised);
    assert_eq!(difficulties(&lobby, physics), plain);

    // With no weights at all there is no top category, and the card does nothing.
    lobby.state.players[0].hero = None;
    lobby.state.players[0].board.retain(|c| c.parts[0].def == lobby.content.card_id("treatise").unwrap());
    assert_eq!(lobby.stats(0).weight_total, 0);
    for category in 0..lobby.content.category_count() as u8 {
        assert_eq!(difficulties(&lobby, category), plain);
    }
}

#[test]
fn a_difficulty_card_sets_the_difficulty_of_the_battles_questions_for_both_players() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "unabridged_edition");
    let events = lobby.apply(Command::EndShop).unwrap();
    // House questions included.
    let plan = &lobby.state.battles[0].plan;
    assert_eq!(plan.len(), 6);
    assert!(plan.iter().all(|slot| slot.question.difficulty == Difficulty::Hard));
    assert!(events.iter().any(|e| matches!(e, Event::CategoryRevealed { difficulty: Difficulty::Hard, .. })));

    // Without the card the six questions are not all hard (the odds make that one in 15625).
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::EndShop).unwrap();
    assert!(lobby.state.battles[0].plan.iter().any(|slot| slot.question.difficulty != Difficulty::Hard));
}

#[test]
fn a_ghosts_difficulty_cards_work_from_the_board_it_was_copied_from() {
    // Find who fights a ghost and whose board the ghost copies. The cards
    // given below do not touch the dice, so the pairing is the same again.
    let ghost_battle = |lobby: &Lobby| {
        let battle = lobby.state.battles.iter().find(|b| b.fighters[1].player.is_none()).expect("a ghost battle");
        (battle.fighters[1].ghost_of.expect("a ghost has a source").index(), battle.plan.clone())
    };
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    lobby.apply(Command::EndShop).unwrap();
    let (source, _) = ghost_battle(&lobby);

    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    lobby.give(source, "pocket_guide");
    lobby.apply(Command::EndShop).unwrap();
    let (again, plan) = ghost_battle(&lobby);
    assert_eq!(again, source);
    assert!(plan.iter().all(|slot| slot.question.difficulty == Difficulty::Easy));
}

#[test]
fn difficulty_cards_do_nothing_in_the_endgame() {
    let mut lobby = endgame_lobby(&[(0, "unabridged_edition"), (1, "footnotes")]);
    let mut seen = Vec::new();
    for _ in 0..12 {
        seen.push(lobby.state.endgame.as_ref().unwrap().question.unwrap().difficulty);
        lobby.endgame_answer([true, true]);
    }
    assert!(seen.iter().any(|&d| d != Difficulty::Hard), "all hard: {seen:?}");
}

// ---- Hero select ---------------------------------------------------------

#[test]
fn every_player_is_offered_two_distinct_heroes_and_no_hero_is_offered_twice() {
    for players in 2..=6 {
        for seed in 0..50 {
            let (content, state, _) = hero_select(players, seed);
            assert_eq!(state.players.len(), players);
            let mut seen = std::collections::BTreeSet::new();
            for offer in offers(&state) {
                assert_eq!(offer.len(), 2, "seed {seed}: an offer is exactly two heroes");
                for hero in offer {
                    assert!(usize::from(hero.0) < content.heroes.len());
                    assert!(seen.insert(hero), "seed {seed}: {hero:?} was offered twice");
                }
            }
        }
    }
}

#[test]
fn hero_offers_are_decided_by_the_lobby_seed() {
    let deal = |seed| offers(&hero_select(6, seed).1);
    assert_eq!(deal(7), deal(7));
    assert!((0..20).any(|seed| deal(seed) != deal(seed + 1)), "different seeds deal different offers");
}

#[test]
fn a_lobby_needs_two_heroes_per_player() {
    let mut content = load_content();
    assert!(GameState::new(6, &content, &mut LobbyRng::from_seed(1)).is_ok());
    content.heroes.truncate(11);
    assert_eq!(GameState::new(6, &content, &mut LobbyRng::from_seed(1)).err(), Some(RuleError::NotEnoughHeroes));
    assert!(GameState::new(5, &content, &mut LobbyRng::from_seed(1)).is_ok());
}

#[test]
fn a_player_can_pick_only_a_hero_from_their_own_offer_and_only_once() {
    let (content, mut state, mut rng) = hero_select(6, 3);
    let mut questions = SyntheticQuestions::default();
    let mut env = Env { content: &content, rng: &mut rng, questions: &mut questions };
    let mine = state.players[0].hero_offer.clone();
    let before = state.clone();

    // Every hero outside the offer is refused, another player's offer included.
    for hero in (0..content.heroes.len() as u8).map(HeroId).filter(|h| !mine.contains(h)) {
        let refused = apply(&mut state, Command::PickHero { player: P0, hero }, &mut env);
        assert_eq!(refused, Err(RuleError::HeroNotOffered));
    }
    let unknown = HeroId(content.heroes.len() as u8);
    assert_eq!(apply(&mut state, Command::PickHero { player: P0, hero: unknown }, &mut env), Err(RuleError::HeroNotOffered));
    assert_eq!(state, before, "a refused pick changes nothing");

    let events = apply(&mut state, Command::PickHero { player: P0, hero: mine[1] }, &mut env).unwrap();
    assert_eq!(events, vec![Event::HeroPicked { player: P0, hero: mine[1] }]);
    assert_eq!(state.players[0].hero, Some(mine[1]));
    let again = apply(&mut state, Command::PickHero { player: P0, hero: mine[0] }, &mut env);
    assert_eq!(again, Err(RuleError::HeroAlreadyPicked));
    assert_eq!(state.players[0].hero, Some(mine[1]));
}

#[test]
fn a_player_who_has_not_picked_gets_a_random_hero_from_their_offer_when_hero_select_ends() {
    let mut given = [false; 2];
    for seed in 0..40 {
        let (content, mut state, mut rng) = hero_select(6, seed);
        let mut questions = SyntheticQuestions::default();
        let mut env = Env { content: &content, rng: &mut rng, questions: &mut questions };
        let picked = state.players[0].hero_offer[0];
        apply(&mut state, Command::PickHero { player: P0, hero: picked }, &mut env).unwrap();
        let events = apply(&mut state, Command::StartMatch, &mut env).unwrap();

        assert_eq!(state.players[0].hero, Some(picked), "a pick already made is kept");
        for p in &state.players {
            let hero = p.hero.expect("everyone has a hero once the match starts");
            let slot = p.hero_offer.iter().position(|&h| h == hero).expect("the hero comes from the player's offer");
            assert_eq!((p.hp, p.max_hp), (content.hero(hero).hp, content.hero(hero).hp));
            if p.id != P0 {
                given[slot] = true;
                assert!(events.contains(&Event::HeroPicked { player: p.id, hero }));
            }
        }
    }
    assert_eq!(given, [true, true], "either offered hero can be the one given");
}

#[test]
fn the_hero_given_at_the_deadline_is_decided_by_the_seed() {
    let run = |seed| {
        let (content, mut state, mut rng) = hero_select(6, seed);
        let mut questions = SyntheticQuestions::default();
        let mut env = Env { content: &content, rng: &mut rng, questions: &mut questions };
        apply(&mut state, Command::StartMatch, &mut env).unwrap();
        state.players.iter().map(|p| p.hero).collect::<Vec<_>>()
    };
    assert_eq!(run(9), run(9));
}

const SILK_ROAD: Command = Command::UseAbility { player: P0, action: AbilityAction::SilkRoad };

#[test]
fn marco_polo_pays_to_swap_one_shop_card_for_one_a_tier_higher_once_a_turn() {
    let mut lobby = Lobby::new(&["marco_polo", "darwin"]);
    lobby.state.players[0].gold = 1;
    assert_eq!(lobby.apply(SILK_ROAD), Err(RuleError::NotEnoughGold));

    lobby.state.players[0].gold = 5;
    let before = lobby.state.players[0].shop.clone();
    lobby.apply(SILK_ROAD).unwrap();
    let after = lobby.state.players[0].shop.clone();
    assert_eq!(lobby.state.players[0].gold, 3, "the swap costs 2 and buys nothing");
    assert_eq!(after.len(), before.len());
    let changed: Vec<usize> = (0..after.len()).filter(|&i| after[i] != before[i]).collect();
    assert_eq!(changed.len(), 1, "exactly one card is swapped");
    assert_eq!(lobby.content.card(after[changed[0]]).tier, 2, "one tier above his level 1");
    assert_eq!(lobby.apply(SILK_ROAD), Err(RuleError::AbilityUnavailable), "once a turn");

    lobby.battle(&[0, 0]);
    lobby.state.players[0].gold = 5;
    lobby.apply(SILK_ROAD).expect("a new turn, a new swap");
}

#[test]
fn at_the_top_level_the_silk_road_swaps_a_lower_card_or_else_any_card_for_a_top_tier_one() {
    let mut lobby = Lobby::new(&["marco_polo", "darwin"]);
    let top = lobby.content.tunables.max_tavern_level;
    let tier = |lobby: &Lobby, i: usize| lobby.content.card(lobby.state.players[0].shop[i]).tier;
    lobby.state.players[0].tavern = top;
    lobby.state.players[0].gold = 5;
    let size = lobby.state.players[0].shop.len();
    assert!((0..size).all(|i| tier(&lobby, i) < top), "the shop was filled at level 1");
    lobby.apply(SILK_ROAD).unwrap();
    assert_eq!((0..size).filter(|&i| tier(&lobby, i) == top).count(), 1, "one lower card became a top-tier one");

    // Every card already top tier: one of them becomes a different top-tier card.
    let aegis = lobby.content.card_id("aegis").unwrap();
    lobby.state.players[0].shop = vec![aegis; size];
    lobby.state.players[0].hero_state.used_turn = 0;
    lobby.state.players[0].gold = 5;
    lobby.apply(SILK_ROAD).unwrap();
    let shop = lobby.state.players[0].shop.clone();
    assert!((0..size).all(|i| tier(&lobby, i) == top));
    assert_eq!(shop.iter().filter(|&&def| def != aegis).count(), 1);
}

#[test]
fn pheidippides_adds_a_point_of_damage_for_each_of_his_correct_answers_when_he_wins() {
    let mut lobby = Lobby::new(&["pheidippides", "darwin"]);
    let events = lobby.battle(&[4, 2]);
    // Margin 2 + base 2 + level 1 = 5, and 4 more for his four correct answers.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(9, 0, 9)]);

    // Losing, his correct answers add nothing to anyone.
    let events = lobby.battle(&[2, 4]);
    assert_eq!(damage_to(&events, P0, DamageSource::Battle), vec![(5, 0, 5)]);
    assert!(damage_to(&events, P1, DamageSource::Battle).is_empty());
}

#[test]
fn cleopatra_keeps_unspent_gold_up_to_one_and_a_half_times_the_gold_cap_and_nobody_else_does() {
    let mut lobby = Lobby::new(&["cleopatra", "darwin"]);
    let t = lobby.content.tunables.clone();
    let income = |turn: u32| (t.gold_start + t.gold_per_turn * (turn - 1)).min(t.gold_cap);
    assert!(!t.gold_carries_over);
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.players[0].gold, income(1) + income(2), "her first turn's gold came along");
    assert_eq!(lobby.state.players[1].gold, income(2), "Darwin's did not");

    lobby.state.players[0].gold = 3 * t.gold_cap;
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.players[0].gold, t.gold_cap * 3 / 2, "the cap is the gold cap and half again");
}

#[test]
fn marie_curie_makes_half_of_her_opponents_armor_count_rounding_down() {
    let mut lobby = Lobby::new(&["marie_curie", "darwin"]);
    lobby.give(1, "wooden_shield");
    let events = lobby.battle(&[4, 2]);
    // 3 armor counts as 1: it absorbs 1 of the 5 damage.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(5, 1, 4)]);

    // Her own armor is not affected.
    let mut lobby = Lobby::new(&["marie_curie", "darwin"]);
    lobby.give(0, "wooden_shield");
    let events = lobby.battle(&[2, 4]);
    assert_eq!(damage_to(&events, P0, DamageSource::Battle), vec![(5, 3, 2)]);
}

#[test]
fn marie_curies_half_life_carries_into_the_endgame() {
    let mut lobby = Lobby::new(&["marie_curie", "darwin"]);
    lobby.give(1, "wooden_shield");
    lobby.state.turn = lobby.content.tunables.endgame_min_turn;
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.phase, Phase::Endgame);
    let darwin_side = lobby.state.endgame.as_ref().unwrap().players.iter().position(|&p| p == P1).unwrap();
    let mut right = [true, true];
    right[darwin_side] = false;
    // Step 1: 3 armor counts as 1 and absorbs the hit. Step 2: 2 armor
    // counts as 1, so 1 of the 2 damage reaches his HP.
    let first = lobby.endgame_answer(right);
    assert_eq!(damage_to(&first, P1, DamageSource::EndgameStep), vec![(1, 1, 0)]);
    let second = lobby.endgame_answer(right);
    assert_eq!(damage_to(&second, P1, DamageSource::EndgameStep), vec![(2, 1, 1)]);
}

#[test]
fn win_damage_is_margin_plus_base_plus_tavern_level() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let before = lobby.hp(1);
    lobby.battle(&[4, 2]);
    let base = lobby.content.tunables.damage_base;
    assert_eq!(before - lobby.hp(1), 2 + base + 1);
    assert_eq!(lobby.hp(0), lobby.state.players[0].max_hp, "the winner takes nothing");
}

#[test]
fn a_tie_deals_no_damage_whatever_the_score() {
    for score in [0, 3, 6] {
        let mut lobby = Lobby::new(&["benz", "darwin"]);
        lobby.battle(&[score, score]);
        assert_eq!((lobby.hp(0), lobby.hp(1)), (35, 32));
    }
}

#[test]
fn a_battle_deals_at_most_the_per_battle_cap() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.players[0].tavern = 6;
    lobby.give_golden(0, "whetstone");
    let events = lobby.battle(&[6, 0]);
    let cap = lobby.content.tunables.max_battle_damage;
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(cap, 0, cap)]);
}

#[test]
fn armor_absorbs_damage_and_a_depleted_shield_frees_its_slot() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "wooden_shield");
    let events = lobby.battle(&[4, 2]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(5, 3, 2)]);
    assert!(lobby.state.players[1].board.is_empty());
    assert!(events.iter().any(|e| matches!(e, Event::ShieldBroken { player, .. } if *player == P1)));
}

#[test]
fn a_shield_only_counts_once_it_is_placed() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give_hand(1, "wooden_shield");
    let events = lobby.battle(&[4, 2]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(5, 0, 5)]);
}

#[test]
fn siege_ram_ignores_part_of_the_armor() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "siege_ram");
    lobby.give(1, "iron_shield");
    let events = lobby.battle(&[4, 2]);
    // 6 armor, 3 ignored: only 3 can absorb.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(5, 3, 2)]);
    assert_eq!(lobby.state.players[1].board[0].armor_left, 3);
}

#[test]
fn aegis_caps_the_damage_taken_in_one_battle() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.players[0].tavern = 6;
    lobby.give(1, "aegis");
    let events = lobby.battle(&[6, 0]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(8, 0, 8)]);
}

#[test]
fn double_or_nothing_multiplies_wins_and_punishes_losses() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "double_or_nothing");
    let events = lobby.battle(&[4, 2]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle)[0].0, 5 * 150 / 100);

    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "double_or_nothing");
    let events = lobby.battle(&[4, 2]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle)[0].0, 5 + 2);
}

#[test]
fn perfect_mind_doubles_only_a_perfect_win() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "perfect_mind");
    let events = lobby.battle(&[6, 5]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle)[0].0, (1 + 2 + 1) * 2);

    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "perfect_mind");
    let events = lobby.battle(&[5, 4]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle)[0].0, 1 + 2 + 1);
}

#[test]
fn streak_banner_adds_damage_for_earlier_wins_in_a_row() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "streak_banner");
    let first = lobby.battle(&[3, 2]);
    let second = lobby.battle(&[3, 2]);
    assert_eq!(damage_to(&first, P1, DamageSource::Battle)[0].0, 4);
    assert_eq!(damage_to(&second, P1, DamageSource::Battle)[0].0, 5);
}

// ---- Reflect -------------------------------------------------------------

#[test]
fn reflect_sends_part_of_the_incoming_damage_back_before_armor() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "hall_of_mirrors");
    lobby.give(1, "iron_shield");
    let events = lobby.battle(&[4, 2]);
    // 5 incoming, fully absorbed by armor, and 30% of the 5, rounded down, still goes back.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(5, 5, 0)]);
    assert_eq!(damage_to(&events, P0, DamageSource::Reflect), vec![(1, 0, 1)]);
}

#[test]
fn reflected_damage_is_never_reflected_again() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "mirror_shard");
    lobby.give(1, "hall_of_mirrors");
    let events = lobby.battle(&[4, 2]);
    let reflections = events.iter().filter(|e| matches!(e, Event::DamageDealt { source: DamageSource::Reflect, .. }));
    assert_eq!(reflections.count(), 1);
}

#[test]
fn when_two_reflect_players_meet_the_lower_percentage_is_subtracted_from_both() {
    assert_eq!(net_reflect(50, 25), 25);
    assert_eq!(net_reflect(25, 50), 0);
    assert_eq!(net_reflect(40, 40), 0);
    assert_eq!(net_reflect(40, 0), 40);

    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "mirror_shard");
    lobby.give(1, "hall_of_mirrors");
    let events = lobby.battle(&[4, 2]);
    // 30% against 10% leaves 20% of 5.
    assert_eq!(damage_to(&events, P0, DamageSource::Reflect), vec![(1, 0, 1)]);
}

#[test]
fn the_mirror_cards_reflect_ten_twenty_and_thirty_percent_and_double_when_golden() {
    let content = load_content();
    let reflect = |name: &str, golden: bool| -> u32 {
        let card = content.card(content.card_id(name).expect("card exists"));
        card.effects_for(golden)
            .iter()
            .map(|e| match *e {
                Effect::Reflect { percent } => percent,
                _ => 0,
            })
            .sum()
    };
    assert_eq!((reflect("mirror_shard", false), reflect("mirror_shard", true)), (10, 20));
    assert_eq!((reflect("thorned_mirror", false), reflect("thorned_mirror", true)), (20, 40));
    assert_eq!((reflect("hall_of_mirrors", false), reflect("hall_of_mirrors", true)), (30, 60));
}

#[test]
fn a_players_total_reflect_is_capped_at_sixty_percent() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    assert_eq!(lobby.content.tunables.reflect_cap, 60);
    lobby.give_golden(0, "hall_of_mirrors");
    lobby.give(0, "thorned_mirror");
    // 60 + 20 from the cards.
    assert_eq!(lobby.stats(0).reflect, 60);
}

#[test]
fn herophilus_has_the_highest_reflect_cap_at_eighty_percent() {
    let mut lobby = Lobby::new(&["herophilus", "darwin"]);
    lobby.give_golden(0, "hall_of_mirrors");
    lobby.give(0, "thorned_mirror");
    // 25 from Open Heart, 60 + 20 from the cards.
    assert_eq!(lobby.stats(0).reflect, 80);
    let cap = |name: &str| lobby.content.hero(lobby.content.hero_id(name).unwrap()).reflect_cap(&lobby.content.tunables);
    assert!(lobby.content.heroes.iter().all(|h| h.reflect_cap(&lobby.content.tunables) <= cap("herophilus")));
}

#[test]
fn two_reflect_players_net_their_capped_percentages() {
    let mut lobby = Lobby::new(&["benz", "herophilus"]);
    for player in 0..2 {
        lobby.give_golden(player, "hall_of_mirrors");
        lobby.give(player, "thorned_mirror");
    }
    // Benz has 80 capped to 60, Herophilus 105 capped to 80: Herophilus keeps
    // 20%, where the uncapped numbers would have left him 25%.
    assert_eq!((lobby.stats(0).reflect, lobby.stats(1).reflect), (60, 80));
    let events = lobby.battle(&[6, 0]);
    // 6 + 2 + 1 = 9 incoming, 20% of it back, rounded down.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(9, 0, 9)]);
    assert_eq!(damage_to(&events, P0, DamageSource::Reflect), vec![(1, 0, 1)]);
}

#[test]
fn piercing_quill_halves_the_opponents_reflect() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "piercing_quill");
    lobby.give(1, "hall_of_mirrors");
    let events = lobby.battle(&[6, 0]);
    // 9 incoming, and half of the 30% leaves 15% of it, rounded down.
    assert_eq!(damage_to(&events, P0, DamageSource::Reflect), vec![(1, 0, 1)]);
}

#[test]
fn reflect_does_nothing_on_a_plain_tie() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "hall_of_mirrors");
    let events = lobby.battle(&[3, 3]);
    assert!(!events.iter().any(|e| matches!(e, Event::DamageDealt { .. })));
}

// ---- Ties ----------------------------------------------------------------

#[test]
fn the_tie_breaker_deals_half_the_tavern_level_and_cancels_when_both_hold_it() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.players[0].tavern = 5;
    lobby.give(0, "scales_of_judgment");
    let events = lobby.battle(&[3, 3]);
    assert_eq!(damage_to(&events, P1, DamageSource::TieBreaker), vec![(2, 0, 2)]);

    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "scales_of_judgment");
    lobby.give(1, "scales_of_judgment");
    let events = lobby.battle(&[3, 3]);
    assert!(damage_to(&events, P1, DamageSource::TieBreaker).is_empty());
    assert!(damage_to(&events, P0, DamageSource::TieBreaker).is_empty());
}

#[test]
fn the_tie_breaker_needs_at_least_one_correct_answer() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "scales_of_judgment");
    let events = lobby.battle(&[0, 0]);
    assert!(damage_to(&events, P1, DamageSource::TieBreaker).is_empty());
}

#[test]
fn last_laugh_deals_one_damage_on_a_zero_zero_tie_only() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "last_laugh");
    let events = lobby.battle(&[0, 0]);
    assert_eq!(damage_to(&events, P1, DamageSource::ZeroZero), vec![(1, 0, 1)]);
    let events = lobby.battle(&[2, 2]);
    assert!(damage_to(&events, P1, DamageSource::ZeroZero).is_empty());
}

#[test]
fn in_overtime_a_tie_splits_the_higher_damage_between_both_players() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.turn = lobby.content.tunables.overtime_turn;
    lobby.state.players[0].tavern = 6;
    let events = lobby.battle(&[3, 3]);
    // The higher of the two would-be damages is 0 + 2 + 6 = 8; both take half.
    assert_eq!(damage_to(&events, P0, DamageSource::Overtime), vec![(4, 0, 4)]);
    assert_eq!(damage_to(&events, P1, DamageSource::Overtime), vec![(4, 0, 4)]);
}

// ---- Questions, weights, time ---------------------------------------------

#[test]
fn a_battle_has_two_house_questions_and_four_weighted_ones() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::EndShop).unwrap();
    let plan = &lobby.state.battles[0].plan;
    assert_eq!(plan.len(), 6);
    assert_eq!(plan.iter().filter(|s| s.owner.is_none()).count(), 2);
    assert_eq!(plan.iter().filter(|s| s.owner == Some(0)).count(), 2);
}

#[test]
fn the_weighted_questions_split_three_one_past_the_threshold_and_never_four_zero() {
    let stats = |total| BattleStats { weight_total: total, ..Default::default() };
    assert_eq!(split(&stats(65), &stats(25), 4, 40), 3);
    assert_eq!(split(&stats(64), &stats(25), 4, 40), 2);
    assert_eq!(split(&stats(25), &stats(100), 4, 40), 1);
    let with_notes = BattleStats { weight_total: 45, split_bonus: 20, ..Default::default() };
    assert_eq!(split(&with_notes, &stats(25), 4, 40), 3);
}

#[test]
fn category_weights_add_up_across_hero_and_cards_and_cap_at_one_hundred() {
    let mut lobby = Lobby::new(&["herodotus", "darwin"]);
    lobby.give(0, "apprentice_history");
    let stats = lobby.stats(0);
    assert_eq!((stats.weights[0], stats.weight_total), (35, 35));

    lobby.give_golden(0, "master_history");
    lobby.give_golden(0, "scholar_physics");
    let stats = lobby.stats(0);
    assert_eq!(stats.weight_total, 100);
    assert!(stats.weights.iter().sum::<u32>() <= 100);
}

#[test]
fn answer_time_bonus_is_capped() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "long_hourglass");
    lobby.give(0, "long_hourglass");
    let t = &lobby.content.tunables;
    let (base, cap) = (t.answer_time, t.answer_time_bonus_cap);
    assert_eq!(lobby.stats(0).time_bonus, cap);
    let events = lobby.apply(Command::EndShop).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::QuestionShown { time_a, time_b, .. } if *time_a == base + cap && *time_b == base)));
}

#[test]
fn a_question_ends_only_when_both_players_have_locked_in() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::EndShop).unwrap();
    let events = lobby.apply(Command::SubmitAnswer { player: P0, choice: Some(0) }).unwrap();
    assert!(events.is_empty());
    assert_eq!(lobby.apply(Command::SubmitAnswer { player: P0, choice: Some(1) }), Err(RuleError::AlreadyAnswered));
    let events = lobby.apply(Command::SubmitAnswer { player: P1, choice: None }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::AnswerRevealed { .. })));
}

#[test]
fn a_timeout_counts_missing_answers_as_wrong() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::EndShop).unwrap();
    let events = lobby.apply(Command::QuestionTimeout { battle: 0 }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::AnswerRevealed { a_correct: false, b_correct: false, .. })));
}

#[test]
fn flaming_sword_and_second_opinion_together_leave_only_the_correct_answer() {
    let mut lobby = Lobby::new(&["newton", "darwin"]);
    lobby.give(0, "second_opinion");
    lobby.state.players[0].hero_state.uses = 2;
    lobby.apply(Command::EndShop).unwrap();
    lobby.apply(Command::UseHelp { player: P0, help: Help::FlamingSword }).unwrap();
    lobby.apply(Command::UseHelp { player: P0, help: Help::SecondOpinion }).unwrap();
    let battle = &lobby.state.battles[0];
    let correct = battle.plan[0].question.correct;
    assert_eq!(battle.fighters[0].removed, 0b1111 & !(1 << correct));
    // Second Opinion works once per battle.
    lobby.apply(Command::SubmitAnswer { player: P0, choice: Some(correct) }).unwrap();
    lobby.apply(Command::SubmitAnswer { player: P1, choice: None }).unwrap();
    assert_eq!(lobby.apply(Command::UseHelp { player: P0, help: Help::SecondOpinion }), Err(RuleError::NoCharges));
}

const BUY_SWORD: Command = Command::UseAbility { player: P0, action: AbilityAction::BuySword };

#[test]
fn newton_has_no_sword_until_he_buys_one_and_can_buy_only_what_the_shop_offers() {
    let mut lobby = Lobby::new(&["newton", "darwin"]);
    lobby.state.players[0].hero_state.sword_offer_turn = 0;
    assert_eq!(lobby.apply(BUY_SWORD), Err(RuleError::AbilityUnavailable), "the shop is not offering it");
    lobby.apply(Command::EndShop).unwrap();
    assert_eq!(lobby.state.battles[0].fighters[0].sword_left, 0);
    assert_eq!(lobby.apply(Command::UseHelp { player: P0, help: Help::FlamingSword }), Err(RuleError::NoCharges));
}

#[test]
fn a_bought_sword_costs_gold_and_gives_two_revelations_that_last_until_used() {
    let mut lobby = Lobby::new(&["newton", "darwin"]);
    let turn = lobby.state.turn;
    lobby.state.players[0].hero_state.sword_offer_turn = turn;
    lobby.state.players[0].gold = 1;
    assert_eq!(lobby.apply(BUY_SWORD), Err(RuleError::NotEnoughGold));
    lobby.state.players[0].gold = 3;
    lobby.apply(BUY_SWORD).unwrap();
    assert_eq!(lobby.state.players[0].gold, 1, "the sword costs 2");
    assert_eq!(lobby.state.players[0].hero_state.uses, 2);
    assert_eq!(lobby.apply(BUY_SWORD), Err(RuleError::AbilityUnavailable), "he already holds one");

    // One revelation used in this battle; the other is still there afterwards.
    lobby.apply(Command::EndShop).unwrap();
    assert_eq!(lobby.state.battles[0].fighters[0].sword_left, 2);
    lobby.apply(Command::UseHelp { player: P0, help: Help::FlamingSword }).unwrap();
    assert_eq!(lobby.state.players[0].hero_state.uses, 1);
    assert_eq!(lobby.state.battles[0].fighters[0].sword_left, 1);
}

#[test]
fn the_shop_offers_the_sword_only_to_newton_and_never_while_he_holds_one() {
    let mut lobby = Lobby::new(&["newton", "darwin"]);
    // Rerolling gives the shop many chances to offer it.
    lobby.state.players[0].hero_state.sword_offer_turn = 0;
    lobby.state.players[0].hero_state.uses = 1;
    for player in [P0, P1] {
        for _ in 0..40 {
            lobby.state.players[player.index()].gold = 5;
            lobby.apply(Command::Reroll { player }).unwrap();
        }
    }
    assert_eq!(lobby.state.players[0].hero_state.sword_offer_turn, 0, "he still holds a revelation");
    assert_eq!(lobby.state.players[1].hero_state.sword_offer_turn, 0, "Darwin is never offered it");

    lobby.state.players[0].hero_state.uses = 0;
    for _ in 0..40 {
        lobby.state.players[0].gold = 5;
        lobby.apply(Command::Reroll { player: P0 }).unwrap();
    }
    assert_eq!(lobby.state.players[0].hero_state.sword_offer_turn, lobby.state.turn, "40 tries at 50% offer it");
}

#[test]
fn a_removed_option_cannot_be_chosen() {
    let mut lobby = Lobby::new(&["newton", "darwin"]);
    lobby.state.players[0].hero_state.uses = 2;
    lobby.apply(Command::EndShop).unwrap();
    lobby.apply(Command::UseHelp { player: P0, help: Help::FlamingSword }).unwrap();
    let removed = lobby.state.battles[0].fighters[0].removed;
    let gone = (0..4).find(|o| removed & (1 << o) != 0).unwrap();
    assert_eq!(lobby.apply(Command::SubmitAnswer { player: P0, choice: Some(gone) }), Err(RuleError::InvalidAnswer));
}

#[test]
fn the_oracle_answers_one_question_per_battle() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "oracle");
    lobby.apply(Command::EndShop).unwrap();
    lobby.apply(Command::UseHelp { player: P0, help: Help::Oracle }).unwrap();
    let events = lobby.apply(Command::SubmitAnswer { player: P1, choice: None }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::AnswerRevealed { a_correct: true, .. })));
    assert_eq!(lobby.apply(Command::UseHelp { player: P0, help: Help::Oracle }), Err(RuleError::NoCharges));
}

// ---- Category bonus --------------------------------------------------------

#[test]
fn three_cards_of_a_category_give_the_bonus_and_more_cards_do_not_stack() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let t = lobby.content.tunables.clone();
    lobby.give(0, "apprentice_history");
    lobby.give(0, "scholar_history");
    assert!(lobby.stats(0).bonus_categories.is_empty());
    lobby.give(0, "master_history");
    lobby.give(0, "apprentice_history");
    assert_eq!(lobby.stats(0).bonus_categories.len(), 1);
    assert_eq!(lobby.stats(0).bonus_categories[0].1, t.category_bonus_damage);
}

#[test]
fn a_golden_category_card_counts_as_three_and_raises_the_bonus() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give_golden(0, "apprentice_history");
    let bonus = lobby.stats(0).bonus_categories;
    assert_eq!(bonus.len(), 1);
    assert_eq!(bonus[0].1, lobby.content.tunables.category_bonus_damage_golden);
}

#[test]
fn encyclopedia_counts_as_a_card_of_every_category() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "apprentice_film");
    lobby.give(0, "scholar_film");
    lobby.give(0, "encyclopedia");
    assert_eq!(lobby.stats(0).bonus_categories.len(), 1);
}

// ---- Shop ----------------------------------------------------------------

#[test]
fn gold_starts_at_three_and_rises_by_one_per_turn_to_the_cap() {
    // Three players, so the lobby stays in regular turns.
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    assert_eq!(lobby.state.players[0].gold, 3);
    lobby.battle(&[0, 0, 0]);
    assert_eq!(lobby.state.players[0].gold, 4);
    for _ in 0..9 {
        lobby.battle(&[0, 0, 0]);
    }
    assert_eq!(lobby.state.players[0].gold, lobby.content.tunables.gold_cap);
}

#[test]
fn bought_cards_go_to_the_hand_and_do_nothing_until_placed() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    let p = &lobby.state.players[0];
    assert_eq!((p.hand.len(), p.board.len(), p.gold), (1, 0, 0));
    let card = p.hand[0].id;
    lobby.apply(Command::Place { player: P0, card }).unwrap();
    assert_eq!(lobby.state.players[0].board.len(), 1);
}

#[test]
fn a_shop_only_offers_cards_at_or_below_the_tavern_level() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    for level in 1..=3 {
        lobby.state.players[0].tavern = level;
        for _ in 0..30 {
            lobby.state.players[0].gold = 10;
            lobby.apply(Command::Reroll { player: P0 }).unwrap();
            let shop = &lobby.state.players[0].shop;
            assert_eq!(shop.len(), lobby.content.tunables.shop_size[(level - 1) as usize] as usize);
            assert!(shop.iter().all(|&d| (1..=level).contains(&lobby.content.card(d).tier)));
        }
    }
}

#[test]
fn the_tavern_upgrade_gets_cheaper_each_turn_it_is_not_bought() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    assert_eq!(lobby.apply(Command::UpgradeTavern { player: P0 }), Err(RuleError::NotEnoughGold));
    lobby.battle(&[0, 0]);
    // Turn 2: 4 gold, and the cost of 5 is one lower.
    lobby.apply(Command::UpgradeTavern { player: P0 }).unwrap();
    assert_eq!((lobby.state.players[0].tavern, lobby.state.players[0].gold), (2, 0));
}

#[test]
fn a_frozen_shop_keeps_its_cards_for_the_next_turn() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let before = lobby.state.players[0].shop.clone();
    lobby.apply(Command::ToggleFreeze { player: P0 }).unwrap();
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.players[0].shop, before);
    assert!(!lobby.state.players[0].frozen);
}

#[test]
fn selling_returns_gold_and_puts_the_card_back_in_the_pool() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let def = lobby.state.players[0].shop[0];
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    let pool = lobby.state.pool[usize::from(def.0)];
    let card = lobby.state.players[0].hand[0].id;
    lobby.apply(Command::Sell { player: P0, card }).unwrap();
    assert_eq!(lobby.state.players[0].gold, lobby.content.tunables.sell_value);
    assert_eq!(lobby.state.pool[usize::from(def.0)], pool + 1);
}

#[test]
fn the_board_holds_seven_cards_and_the_hand_ten() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    for _ in 0..7 {
        lobby.give(0, "hourglass");
    }
    // Different cards, so nothing merges.
    for name in ["whetstone", "mirror_shard", "coin_purse", "siege_ram", "last_laugh"] {
        lobby.give_hand(0, name);
        lobby.give_hand(0, name);
    }
    let card = lobby.state.players[0].hand[0].id;
    assert_eq!(lobby.apply(Command::Place { player: P0, card }), Err(RuleError::BoardFull));
    assert_eq!(lobby.apply(Command::Buy { player: P0, index: 0 }), Err(RuleError::HandFull));
}

#[test]
fn a_rejected_command_leaves_the_state_unchanged() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.players[0].gold = 1;
    let before = lobby.state.clone();
    assert_eq!(lobby.apply(Command::Buy { player: P0, index: 0 }), Err(RuleError::NotEnoughGold));
    assert_eq!(lobby.apply(Command::Buy { player: P0, index: 99 }), Err(RuleError::BadIndex));
    assert_eq!(lobby.apply(Command::SubmitAnswer { player: P0, choice: Some(0) }), Err(RuleError::WrongPhase));
    assert_eq!(lobby.apply(Command::PickReward { player: P0, index: 0 }), Err(RuleError::BadIndex));
    assert_eq!(lobby.state, before);
}

// ---- Golden cards ----------------------------------------------------------

#[test]
fn three_identical_cards_merge_into_a_golden_one_counting_hand_and_board() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "whetstone");
    lobby.give_hand(0, "whetstone");
    let third = lobby.give_hand(0, "whetstone");
    lobby.apply(Command::Place { player: P0, card: third }).unwrap();
    // The golden card goes to the hand; it works once the player places it.
    let p = &lobby.state.players[0];
    assert_eq!((p.board.len(), p.hand.len()), (0, 1));
    assert!(p.hand[0].parts[0].golden);
    assert_eq!(lobby.stats(0).damage_on_win, 0, "a card in hand does nothing");
    let golden = p.hand[0].id;
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    assert_eq!(lobby.stats(0).damage_on_win, 2, "golden doubles the number");
}

#[test]
fn a_full_hand_still_takes_the_card_that_completes_a_golden_one() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let hand_size = lobby.content.tunables.hand_size as usize;
    let offered = lobby.state.players[0].shop[0];
    let name = lobby.content.card(offered).id.clone();
    lobby.give_hand(0, &name);
    lobby.give_hand(0, &name);
    // Fill the rest of the hand with single cards that merge with nothing.
    let fillers = ["mirror_shard", "coin_purse", "siege_ram", "last_laugh", "streak_banner", "rivals_notes", "piercing_quill", "perfect_mind"];
    for filler in fillers.iter().filter(|f| **f != name).take(hand_size - 2) {
        lobby.give_hand(0, filler);
    }
    assert_eq!(lobby.state.players[0].hand.len(), hand_size);
    assert_eq!(lobby.apply(Command::Buy { player: P0, index: 1 }), Err(RuleError::HandFull), "an ordinary card does not fit");

    lobby.apply(Command::Buy { player: P0, index: 0 }).expect("the third copy is taken");
    let p = &lobby.state.players[0];
    assert_eq!(p.hand.len(), hand_size - 1, "three copies became one golden card");
    assert!(p.hand.iter().any(|c| c.parts[0].golden && c.has_def(offered)));
}

#[test]
fn a_golden_card_takes_a_board_slot_only_when_the_hand_has_no_room() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let hand_size = lobby.content.tunables.hand_size as usize;
    let offered = lobby.state.players[0].shop[0];
    let name = lobby.content.card(offered).id.clone();
    lobby.give(0, &name);
    lobby.give(0, &name);
    let fillers = ["mirror_shard", "coin_purse", "siege_ram", "last_laugh", "streak_banner", "rivals_notes", "piercing_quill", "perfect_mind", "hall_of_mirrors", "thorned_mirror", "double_or_nothing"];
    for filler in fillers.iter().filter(|f| **f != name).take(hand_size) {
        lobby.give_hand(0, filler);
    }
    lobby.apply(Command::Buy { player: P0, index: 0 }).expect("it completes a golden card");
    let p = &lobby.state.players[0];
    assert_eq!(p.hand.len(), hand_size, "the hand stayed full");
    assert_eq!(p.board.len(), 1);
    assert!(p.board[0].parts[0].golden, "with no room in hand it stays on the board");
}

/// Gives player 0 two Whetstones on the board and a third in hand, and
/// places it: the three merge into a golden Whetstone in the hand. Returns
/// the golden card's id.
fn golden_whetstone_in_hand(lobby: &mut Lobby) -> CardInstanceId {
    lobby.give(0, "whetstone");
    lobby.give(0, "whetstone");
    let third = lobby.give_hand(0, "whetstone");
    lobby.apply(Command::Place { player: P0, card: third }).unwrap();
    let p = &lobby.state.players[0];
    assert!(p.board.is_empty());
    assert!(p.hand.len() == 1 && p.hand[0].parts[0].golden);
    p.hand[0].id
}

fn reward_cards(lobby: &Lobby, player: usize) -> Vec<CardInstanceId> {
    let def = lobby.content.golden_reward;
    lobby.state.players[player].hand.iter().filter(|c| c.has_def(def)).map(|c| c.id).collect()
}

#[test]
fn a_golden_card_offers_its_reward_when_it_is_placed_not_when_it_is_made() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "whetstone");
    lobby.give(0, "whetstone");
    let third = lobby.give_hand(0, "whetstone");
    let events = lobby.apply(Command::Place { player: P0, card: third }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::GoldenMade { .. })));
    assert!(!events.iter().any(|e| matches!(e, Event::RewardOffered { .. })), "nothing is offered on making");
    assert!(lobby.state.players[0].rewards.is_empty());
    assert_eq!(lobby.apply(Command::PickReward { player: P0, index: 0 }), Err(RuleError::BadIndex));

    let golden = lobby.state.players[0].hand[0].id;
    let events = lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::RewardOffered { player, .. } if *player == P0)));
    let choices = lobby.state.players[0].rewards[0].clone();
    assert_eq!(choices.len(), 3);
    assert!(choices.iter().all(|&d| lobby.content.card(d).tier == 2));
    assert!(choices[0] != choices[1] && choices[1] != choices[2] && choices[0] != choices[2], "three different cards");
    let gold = lobby.state.players[0].gold;
    lobby.apply(Command::PickReward { player: P0, index: 1 }).unwrap();
    let p = &lobby.state.players[0];
    assert_eq!((p.board.len(), p.hand.len(), p.gold), (1, 1, gold), "the reward is free");
    assert!(p.hand[0].has_def(choices[1]));
    assert!(p.rewards.is_empty());
}

#[test]
fn a_golden_card_gives_its_reward_once_however_often_it_is_placed() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    lobby.apply(Command::PickReward { player: P0, index: 0 }).unwrap();
    // Taken back into the hand and placed again: nothing more.
    let card = lobby.state.players[0].board.remove(0);
    lobby.state.players[0].hand.push(card);
    let events = lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    assert!(!events.iter().any(|e| matches!(e, Event::RewardOffered { .. })));
    assert!(lobby.state.players[0].rewards.is_empty());
}

#[test]
fn the_reward_options_come_from_the_tier_above_the_tavern_level_when_the_pick_opens() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    // Made at level 1, placed at level 3: tier 4.
    lobby.state.players[0].tavern = 3;
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    assert!(lobby.state.players[0].rewards[0].iter().all(|&d| lobby.content.card(d).tier == 4));
    // Left unpicked, then opened from the reward card at the highest level: tier 6.
    lobby.battle(&[3, 3]);
    let max = lobby.content.tunables.max_tavern_level;
    lobby.state.players[0].tavern = max;
    let card = reward_cards(&lobby, 0)[0];
    lobby.apply(Command::Place { player: P0, card }).unwrap();
    assert!(lobby.state.players[0].rewards[0].iter().all(|&d| lobby.content.card(d).tier == max));
}

#[test]
fn an_unpicked_golden_reward_becomes_a_reward_card_in_hand_when_the_shop_closes() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    let choices = lobby.state.players[0].rewards[0].clone();
    let events = lobby.apply(Command::EndShop).unwrap();
    let reward_def = lobby.content.golden_reward;
    assert!(events.iter().any(|e| matches!(e, Event::CardGained { player, def, .. } if *player == P0 && *def == reward_def)));
    let p = &lobby.state.players[0];
    assert!(p.rewards.is_empty());
    assert_eq!(p.hand.len(), 1, "only the reward card: no option was given");
    assert!(p.hand[0].has_def(reward_def));
    assert_eq!(p.board.len(), 1, "it does not place itself");
    // The three options went back to the pool.
    for def in choices {
        assert_eq!(lobby.state.pool[usize::from(def.0)], lobby.content.card(def).copies);
    }
}

#[test]
fn placing_the_reward_card_opens_the_pick_and_the_card_is_gone() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    lobby.battle(&[3, 3]);
    assert_eq!(lobby.state.phase, Phase::Shop);
    let card = reward_cards(&lobby, 0)[0];
    // It takes no slot, so a full board does not stop it.
    for filler in ["hourglass", "mirror_shard", "coin_purse", "siege_ram", "last_laugh", "streak_banner"] {
        lobby.give(0, filler);
    }
    let board = lobby.state.players[0].board.len();
    assert_eq!(board, lobby.content.tunables.board_slots as usize);
    let events = lobby.apply(Command::Place { player: P0, card }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::RewardOffered { player, .. } if *player == P0)));
    let p = &lobby.state.players[0];
    assert!(p.find_card(card).is_none(), "the reward card is neither in hand nor on the board");
    assert_eq!(p.board.len(), board);
    assert_eq!(p.rewards.len(), 1);
    let choices = p.rewards[0].clone();
    assert_eq!(choices.len(), 3);
    lobby.apply(Command::PickReward { player: P0, index: 2 }).unwrap();
    let p = &lobby.state.players[0];
    assert!(p.rewards.is_empty());
    assert!(p.hand.iter().any(|c| c.has_def(choices[2])));
    assert!(reward_cards(&lobby, 0).is_empty());
}

#[test]
fn a_reward_card_left_unopened_or_unpicked_stays_a_reward_card() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    lobby.battle(&[3, 3]);
    // Not placed for a whole turn: still there.
    lobby.battle(&[3, 3]);
    let card = reward_cards(&lobby, 0)[0];
    // Placed but not picked: a new reward card when the shop closes.
    lobby.apply(Command::Place { player: P0, card }).unwrap();
    lobby.battle(&[3, 3]);
    let cards = reward_cards(&lobby, 0);
    assert_eq!(cards.len(), 1);
    assert_ne!(cards[0], card);
    assert!(lobby.state.players[0].rewards.is_empty());
}

#[test]
fn a_golden_card_made_with_a_full_hand_is_on_the_board_and_offers_its_reward_at_once() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let hand_size = lobby.content.tunables.hand_size as usize;
    let offered = lobby.state.players[0].shop[0];
    let name = lobby.content.card(offered).id.clone();
    lobby.give(0, &name);
    lobby.give(0, &name);
    let fillers = ["mirror_shard", "coin_purse", "siege_ram", "last_laugh", "streak_banner", "rivals_notes", "piercing_quill", "perfect_mind", "hall_of_mirrors", "thorned_mirror", "double_or_nothing"];
    for filler in fillers.iter().filter(|f| **f != name).take(hand_size) {
        lobby.give_hand(0, filler);
    }
    let pool = lobby.state.pool.clone();
    let events = lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::RewardOffered { player, .. } if *player == P0)));
    let p = &lobby.state.players[0];
    assert!(p.board[0].parts[0].golden && !p.board[0].parts[0].reward_due);
    assert_eq!(p.rewards.len(), 1);
    // The pick needs room in the hand.
    assert_eq!(lobby.apply(Command::PickReward { player: P0, index: 0 }), Err(RuleError::HandFull));
    let sold = lobby.state.players[0].hand[0].id;
    lobby.apply(Command::Sell { player: P0, card: sold }).unwrap();
    lobby.apply(Command::PickReward { player: P0, index: 0 }).unwrap();
    assert_eq!(lobby.state.players[0].hand.len(), hand_size);

    // Still unpicked when the shop closes with a full hand: there is no room
    // for a reward card, so nothing is given and the options return to the pool.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, &name);
    lobby.give(0, &name);
    for filler in fillers.iter().filter(|f| **f != name).take(hand_size) {
        lobby.give_hand(0, filler);
    }
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    assert_eq!(lobby.state.players[0].rewards.len(), 1);
    lobby.apply(Command::EndShop).unwrap();
    let p = &lobby.state.players[0];
    assert!(p.rewards.is_empty());
    assert_eq!(p.hand.len(), hand_size);
    assert!(reward_cards(&lobby, 0).is_empty());
    for (i, card) in lobby.content.cards.iter().enumerate().filter(|(_, c)| c.tier == 2) {
        assert_eq!(lobby.state.pool[i], pool[i], "{}", card.id);
    }
}

#[test]
fn a_reward_card_is_not_a_real_card_it_sells_for_nothing_and_never_enters_the_pool() {
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    lobby.battle(&[3, 3, 3]);
    let card = reward_cards(&lobby, 0)[0];
    let reward_def = lobby.content.golden_reward;
    assert!(lobby.content.card(reward_def).is_token() && !lobby.content.card(reward_def).in_shop);
    // Benz cannot fuse it into a lasting card.
    let other = lobby.give(0, "hourglass");
    let action = AbilityAction::Combine { a: other, b: card };
    assert_eq!(lobby.apply(Command::UseAbility { player: P0, action }), Err(RuleError::NotUsable));

    // Sold: no gold, and the pool is as it was.
    let (pool, gold) = (lobby.state.pool.clone(), lobby.state.players[0].gold);
    lobby.apply(Command::Sell { player: P0, card }).unwrap();
    assert_eq!((lobby.state.pool.clone(), lobby.state.players[0].gold), (pool, gold));
    assert!(reward_cards(&lobby, 0).is_empty());

    // Left in the hand of a player who is eliminated: it is simply gone.
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    lobby.battle(&[3, 3, 3]);
    assert_eq!(reward_cards(&lobby, 0).len(), 1);
    lobby.state.players[0].hp = 1;
    lobby.battle(&[0, 6, 6]);
    assert!(!lobby.state.players[0].alive);
    assert!(lobby.state.players[0].hand.is_empty());
    assert_eq!(lobby.state.pool[usize::from(reward_def.0)], 0);
}

#[test]
fn darwins_golden_card_also_gives_its_reward_on_placement() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give_hand(1, "whetstone");
    lobby.give_hand(1, "whetstone");
    let def = lobby.content.card_id("whetstone").unwrap();
    let events = lobby.apply(Command::UseAbility { player: P1, action: AbilityAction::DarwinMerge { def } }).unwrap();
    assert!(!events.iter().any(|e| matches!(e, Event::RewardOffered { .. })));
    let golden = lobby.state.players[1].hand[0].id;
    lobby.apply(Command::Place { player: P1, card: golden }).unwrap();
    assert_eq!(lobby.state.players[1].rewards.len(), 1);
}

#[test]
fn a_golden_card_benz_combines_gives_its_reward_when_the_combined_card_is_on_the_board() {
    // Combined in hand: nothing yet, then the reward when the combined card is placed.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    let other = lobby.give_hand(0, "hourglass");
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Combine { a: golden, b: other } }).unwrap();
    assert!(lobby.state.players[0].rewards.is_empty());
    let combined = lobby.state.players[0].hand[0].id;
    lobby.apply(Command::Place { player: P0, card: combined }).unwrap();
    assert_eq!(lobby.state.players[0].rewards.len(), 1);

    // Combined onto a card that is on the board: that places it, so the reward comes at once.
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let golden = golden_whetstone_in_hand(&mut lobby);
    let on_board = lobby.give(0, "hourglass");
    let action = AbilityAction::Combine { a: on_board, b: golden };
    let events = lobby.apply(Command::UseAbility { player: P0, action }).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::RewardOffered { .. })));
    assert_eq!(lobby.state.players[0].rewards.len(), 1);
}

#[test]
fn the_golden_hunters_arrow_comes_when_it_is_made_and_its_pick_when_it_is_placed() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "hunter");
    lobby.give_hand(0, "hunter");
    lobby.give_hand(0, "hunter");
    // Buying any card checks for merges.
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    let hunter = lobby.content.card_id("hunter").unwrap();
    let arrow = lobby.content.card_id("arrow").unwrap();
    let p = &lobby.state.players[0];
    assert!(p.hand.iter().any(|c| c.has_def(arrow)), "the arrow is granted on making");
    assert!(p.rewards.is_empty());
    let golden = p.hand.iter().find(|c| c.has_def(hunter) && c.parts[0].golden).expect("golden hunter in hand").id;
    lobby.apply(Command::Place { player: P0, card: golden }).unwrap();
    assert_eq!(lobby.state.players[0].rewards.len(), 1);
}

#[test]
fn shields_cannot_be_made_golden() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "wooden_shield");
    lobby.give(0, "wooden_shield");
    let third = lobby.give_hand(0, "wooden_shield");
    lobby.apply(Command::Place { player: P0, card: third }).unwrap();
    assert_eq!(lobby.state.players[0].board.len(), 3);
}

#[test]
fn golden_aegis_keeps_its_cap_and_adds_ten_armor() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give_golden(0, "aegis");
    assert_eq!(lobby.state.players[0].armor(), 10);
    assert_eq!(lobby.stats(0).damage_cap, Some(8));
}

#[test]
fn consolation_gives_a_buckler_on_a_loss_and_three_bucklers_combine() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "consolation");
    lobby.battle(&[4, 2]);
    assert_eq!(lobby.state.players[1].hand.len(), 1);
    assert_eq!(lobby.state.players[1].armor(), 0, "a buckler in hand gives nothing");
    lobby.battle(&[4, 2]);
    lobby.battle(&[4, 2]);
    let hand = &lobby.state.players[1].hand;
    assert_eq!(hand.len(), 1);
    assert_eq!(hand[0].armor_left, 6);
}

// ---- Healing ---------------------------------------------------------------

#[test]
fn a_healer_heals_every_turn_but_never_above_starting_hp() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(1, "healer");
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.hp(1), 32, "no overheal");
    lobby.battle(&[4, 2]);
    assert_eq!(lobby.hp(1), 32 - 5 + 1);
}

#[test]
fn a_hunter_disables_the_opponents_next_heal_without_destroying_the_healer() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "hunter");
    lobby.give(1, "healer");
    lobby.battle(&[4, 2]);
    assert_eq!(lobby.hp(1), 32 - 5);
    assert_eq!(lobby.state.players[1].board.len(), 1);
}

#[test]
fn only_one_healer_and_one_hunter_fit_on_a_board() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    for name in ["healer", "hunter"] {
        lobby.give(0, name);
        let second = lobby.give_hand(0, name);
        assert_eq!(lobby.apply(Command::Place { player: P0, card: second }), Err(RuleError::BoardLimit));
    }
}

#[test]
fn a_golden_healer_heals_the_lobby_once_and_only_one_exists_per_match() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.players[0].hp = 10;
    lobby.state.players[1].hp = 10;
    lobby.give(0, "healer");
    lobby.give_hand(0, "healer");
    lobby.give_hand(0, "healer");
    // Any accepted shop action re-checks merges.
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    assert!(lobby.state.players[0].hand.iter().any(|c| c.parts[0].golden), "the golden healer is in hand");
    assert_eq!(lobby.hp(0), 10 + 35 / 4);
    assert_eq!(lobby.hp(1), 10 + 32 / 8);

    // A second set of three does not merge.
    lobby.give(1, "healer");
    lobby.give_hand(1, "healer");
    lobby.give_hand(1, "healer");
    lobby.apply(Command::Buy { player: P1, index: 0 }).unwrap();
    assert!(!lobby.state.players[1].board[0].parts[0].golden);
}

#[test]
fn a_healer_disabled_last_round_cannot_be_made_golden() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "hunter");
    lobby.give(1, "healer");
    lobby.battle(&[0, 0]);
    lobby.give_hand(1, "healer");
    lobby.give_hand(1, "healer");
    lobby.apply(Command::Buy { player: P1, index: 0 }).unwrap();
    assert!(!lobby.state.players[1].board[0].parts[0].golden);
}

#[test]
fn a_golden_hunter_gives_an_arrow_that_hits_whatever_the_result_then_vanishes() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "hunter");
    lobby.give_hand(0, "hunter");
    lobby.give_hand(0, "hunter");
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    let arrow_def = lobby.content.card_id("arrow").unwrap();
    let arrow = lobby.state.players[0].hand.iter().find(|c| c.has_def(arrow_def)).expect("arrow granted").id;
    lobby.apply(Command::Place { player: P0, card: arrow }).unwrap();
    let events = lobby.battle(&[0, 3]);
    assert_eq!(damage_to(&events, P1, DamageSource::Arrow), vec![(3, 0, 3)]);
    assert!(!lobby.state.players[0].board.iter().any(|c| c.has_def(arrow_def)));
}

#[test]
fn a_used_potion_heals_and_leaves_the_game() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let potion = lobby.content.potion;
    let pool = lobby.state.pool[usize::from(potion.0)];
    lobby.state.players[0].hp = 20;
    let card = lobby.give_hand(0, "potion");
    assert_eq!(lobby.apply(Command::Place { player: P0, card }), Err(RuleError::NotUsable));
    lobby.apply(Command::UseCard { player: P0, card }).unwrap();
    assert_eq!(lobby.hp(0), 25);
    assert!(lobby.state.players[0].hand.is_empty());
    assert_eq!(lobby.state.pool[usize::from(potion.0)], pool, "not returned to the pool");
}

// ---- Heroes ----------------------------------------------------------------

#[test]
fn darwin_makes_a_golden_card_from_two_copies_twice_per_game_once_a_round() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    for name in ["whetstone", "hourglass", "mirror_shard"] {
        lobby.give(1, name);
        lobby.give(1, name);
    }
    let def = |lobby: &Lobby, name| lobby.content.card_id(name).unwrap();
    let action = AbilityAction::DarwinMerge { def: def(&lobby, "whetstone") };
    lobby.apply(Command::UseAbility { player: P1, action }).unwrap();
    let action = AbilityAction::DarwinMerge { def: def(&lobby, "hourglass") };
    assert_eq!(lobby.apply(Command::UseAbility { player: P1, action: action.clone() }), Err(RuleError::UsedThisRound));
    lobby.battle(&[0, 0]);
    lobby.apply(Command::UseAbility { player: P1, action }).unwrap();
    lobby.battle(&[0, 0]);
    let darwin = &lobby.state.players[1];
    assert_eq!(darwin.board.iter().chain(&darwin.hand).filter(|c| c.parts[0].golden).count(), 2);
    let action = AbilityAction::DarwinMerge { def: def(&lobby, "mirror_shard") };
    assert_eq!(lobby.apply(Command::UseAbility { player: P1, action }), Err(RuleError::NoCharges));
}

#[test]
fn benz_combines_two_cards_into_one_slot_with_both_effects() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let a = lobby.give(0, "whetstone");
    let b = lobby.give(0, "iron_shield");
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Combine { a, b } }).unwrap();
    let p = &lobby.state.players[0];
    assert_eq!((p.board.len(), p.board[0].parts.len(), p.armor()), (1, 2, 6));
    assert_eq!(lobby.stats(0).damage_on_win, 1);
}

#[test]
fn benz_combines_at_most_once_a_round() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let a = lobby.give_hand(0, "whetstone");
    let b = lobby.give_hand(0, "hourglass");
    let c = lobby.give_hand(0, "mirror_shard");
    let d = lobby.give_hand(0, "coin_purse");
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Combine { a, b } }).unwrap();
    let again = Command::UseAbility { player: P0, action: AbilityAction::Combine { a: c, b: d } };
    assert_eq!(lobby.apply(again.clone()), Err(RuleError::UsedThisRound));
    assert_eq!(lobby.state.players[0].hero_state.uses, 1, "a refused use spends no charge");
    lobby.battle(&[0, 0]);
    lobby.apply(again).unwrap();
}

#[test]
fn shakespeare_uses_to_be_or_not_to_be_at_most_once_a_round() {
    let mut lobby = Lobby::new(&["shakespeare", "darwin"]);
    let to_be = Command::UseAbility { player: P0, action: AbilityAction::ToBeOrNotToBe };
    lobby.apply(to_be.clone()).unwrap();
    lobby.state.players[0].gold = 5;
    assert_eq!(lobby.apply(to_be.clone()), Err(RuleError::UsedThisRound));
    assert_eq!(lobby.state.players[0].gold, 5, "a refused use costs nothing");
    lobby.battle(&[0, 0]);
    lobby.apply(to_be).unwrap();
}

#[test]
fn herodotus_records_the_round_of_his_single_use() {
    let mut lobby = Lobby::new(&["herodotus", "darwin"]);
    lobby.state.players[0].tavern = 6;
    lobby.battle(&[2, 4]);
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Rewrite }).unwrap();
    let p = &lobby.state.players[0];
    assert_eq!((p.hero_state.uses, p.hero_state.used_turn), (1, lobby.state.turn));
}

#[test]
fn herophilus_reflects_a_quarter_of_the_damage_he_takes() {
    let mut lobby = Lobby::new(&["herophilus", "darwin"]);
    let events = lobby.battle(&[0, 6]);
    // 6 + 2 + 1 = 9 incoming, 25% back.
    assert_eq!(damage_to(&events, P1, DamageSource::Reflect), vec![(2, 0, 2)]);
}

#[test]
fn herodotus_revives_once_with_half_hp_when_his_ability_is_unlocked() {
    let mut lobby = Lobby::new(&["herodotus", "darwin"]);
    lobby.state.players[0].tavern = 6;
    lobby.state.players[0].hp = 1;
    let events = lobby.battle(&[0, 3]);
    assert!(events.iter().any(|e| matches!(e, Event::PlayerRevived { .. })));
    assert_eq!(lobby.hp(0), 35 / 2);
    lobby.state.players[0].hp = 1;
    lobby.battle(&[0, 3]);
    assert!(!lobby.state.players[0].alive, "the ability works once");
}

#[test]
fn herodotus_rewrite_refunds_his_loss_deals_max_damage_and_never_kills() {
    let mut lobby = Lobby::new(&["herodotus", "darwin"]);
    lobby.state.players[0].tavern = 6;
    lobby.battle(&[2, 4]);
    assert!(lobby.hp(0) < 35);
    lobby.state.players[1].hp = 5;
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Rewrite }).unwrap();
    assert_eq!(lobby.hp(0), 35, "the damage he took is refunded");
    let p = &lobby.state.players[1];
    assert_eq!((p.hp, p.temp_armor), (1, lobby.content.tunables.herodotus_shield));
    let again = lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Rewrite });
    assert_eq!(again, Err(RuleError::AbilityUnavailable));
}

#[test]
fn lumiere_replays_a_film_question_in_the_next_battle() {
    // Three players, so the endgame never starts while film is awaited.
    let mut lobby = Lobby::new(&["lumiere", "darwin", "newton"]);
    let film = lobby.content.hero(lobby.state.players[0].hero.unwrap()).category;
    // Play until a film question has come up.
    let question = loop {
        lobby.battle(&[0, 0, 0]);
        let last = lobby.state.players[0].last_battle.clone().unwrap();
        if let Some(q) = last.questions.into_iter().find(|q| q.category == film) {
            break q;
        }
    };
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Record { question: question.id } }).unwrap();
    lobby.apply(Command::EndShop).unwrap();
    let his = lobby.state.battles.iter().find(|b| b.fighters.iter().any(|f| f.player == Some(P0))).unwrap();
    assert!(his.plan.iter().any(|s| s.question == question));
}

#[test]
fn nietzsche_chooses_two_different_house_categories_from_the_options() {
    let mut lobby = Lobby::new(&["nietzsche", "darwin"]);
    let events = lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::Ubermensch }).unwrap();
    let options = events
        .iter()
        .find_map(|e| if let Event::HouseOptionsShown { options, .. } = e { Some(options.clone()) } else { None })
        .unwrap();
    assert_eq!(options.len(), 4);
    let same = AbilityAction::ChooseHouse { first: options[0], second: options[0] };
    assert_eq!(lobby.apply(Command::UseAbility { player: P0, action: same }), Err(RuleError::BadIndex));
    let pick = AbilityAction::ChooseHouse { first: options[0], second: options[1] };
    lobby.apply(Command::UseAbility { player: P0, action: pick }).unwrap();
    lobby.apply(Command::EndShop).unwrap();
    let mut house: Vec<_> =
        lobby.state.battles[0].plan.iter().filter(|s| s.owner.is_none()).map(|s| s.question.category).collect();
    house.sort();
    let mut wanted = vec![options[0], options[1]];
    wanted.sort();
    assert_eq!(house, wanted);
}

#[test]
fn shakespeare_skips_a_battle_when_more_than_three_players_are_left() {
    let mut lobby = Lobby::new(&["shakespeare", "darwin", "benz", "newton"]);
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::ToBeOrNotToBe }).unwrap();
    assert_eq!(lobby.state.players[0].gold, 0);
    assert_eq!(lobby.apply(Command::Reroll { player: P0 }), Err(RuleError::LockedOut));
    lobby.apply(Command::EndShop).unwrap();
    let fighting = |f: &Fighter| f.player == Some(P0);
    assert!(!lobby.state.battles.iter().any(|b| b.fighters.iter().any(fighting)));
}

#[test]
fn shakespeare_can_skip_only_a_limited_number_of_battles_per_game() {
    let mut lobby = Lobby::new(&["shakespeare", "darwin", "benz", "newton"]);
    let skips = lobby.content.hero(lobby.state.players[0].hero.unwrap()).param("skips");
    let skip = Command::UseAbility { player: P0, action: AbilityAction::ToBeOrNotToBe };
    for _ in 0..skips {
        lobby.apply(skip.clone()).unwrap();
        lobby.battle(&[0, 0, 0, 0]);
    }
    let gold = lobby.state.players[0].gold;
    assert_eq!(lobby.apply(skip), Err(RuleError::NoCharges));
    assert_eq!(lobby.state.players[0].gold, gold, "a refused skip costs nothing");
}

#[test]
fn each_placed_skull_disables_a_different_opponent_card_and_vanishes_after_the_round() {
    let mut lobby = Lobby::new(&["shakespeare", "darwin"]);
    for name in ["whetstone", "hourglass", "mirror_shard", "coin_purse"] {
        lobby.give(1, name);
    }
    // With three or fewer players left, the ability gives skulls for all his gold.
    lobby.apply(Command::UseAbility { player: P0, action: AbilityAction::ToBeOrNotToBe }).unwrap();
    assert_eq!(lobby.state.players[0].gold, 0);
    let skulls: Vec<_> = lobby.state.players[0].hand.iter().map(|c| c.id).collect();
    assert_eq!(skulls.len(), 3);
    for card in skulls {
        lobby.apply(Command::Place { player: P0, card }).unwrap();
    }
    let events = lobby.battle(&[2, 4]);
    let mut disabled: Vec<_> = events
        .iter()
        .filter_map(|e| if let Event::CardDisabled { player: P1, card } = e { Some(*card) } else { None })
        .collect();
    disabled.sort();
    disabled.dedup();
    assert_eq!(disabled.len(), 3);
    assert!(lobby.state.players[0].board.is_empty(), "skulls last one round");
    assert!(lobby.state.players[1].board.iter().all(|c| !c.disabled), "cards come back after the battle");
}

#[test]
fn skulls_appear_in_every_shop_only_while_shakespeare_is_in_the_lobby() {
    let saw_skull = |heroes: &[&str]| {
        let mut lobby = Lobby::new(heroes);
        let skull = lobby.content.skull;
        (0..60).any(|_| {
            lobby.state.players[1].gold = 10;
            lobby.apply(Command::Reroll { player: P1 }).unwrap();
            lobby.state.players[1].extras.contains(&skull)
        })
    };
    assert!(saw_skull(&["shakespeare", "darwin"]), "Darwin is offered skulls when Shakespeare is present");
    assert!(!saw_skull(&["benz", "darwin"]));
}

#[test]
fn skulls_cannot_disable_skulls() {
    let mut lobby = Lobby::new(&["shakespeare", "darwin"]);
    lobby.give(0, "skull");
    lobby.give(0, "whetstone");
    lobby.give(1, "skull");
    lobby.give(1, "hourglass");
    let events = lobby.battle(&[0, 0]);
    let disabled = events.iter().filter(|e| matches!(e, Event::CardDisabled { .. })).count();
    assert_eq!(disabled, 2, "each skull hits the one real card on the other side");
}

#[test]
fn a_board_full_of_skulls_leaves_both_players_with_hero_stats_only() {
    let mut lobby = Lobby::new(&["shakespeare", "darwin"]);
    for _ in 0..7 {
        lobby.give(0, "skull");
        lobby.give(1, "whetstone");
    }
    let events = lobby.battle(&[2, 4]);
    // No Whetstone counts: 2 + 2 + 1.
    assert_eq!(damage_to(&events, P0, DamageSource::Battle)[0].0, 5);
}

// ---- Lobby flow --------------------------------------------------------------

#[test]
fn with_an_odd_number_of_players_one_fights_a_ghost_that_takes_no_damage() {
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    let events = lobby.battle(&[6, 6, 6]);
    let ghosts = events.iter().filter(|e| matches!(e, Event::BattleStarted { b: None, ghost_of: Some(_), .. }));
    assert_eq!(ghosts.count(), 1);
    assert_eq!(lobby.state.alive_count(), 3);
}

#[test]
fn an_eliminated_players_cards_return_to_the_pool() {
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    lobby.apply(Command::Buy { player: P1, index: 0 }).unwrap();
    let def = lobby.state.players[1].hand[0].parts[0].def;
    lobby.state.players[1].hp = 1;
    let events = lobby.battle(&[6, 0, 6]);
    assert!(events.iter().any(|e| matches!(e, Event::PlayerEliminated { player, placement: 3 } if *player == P1)));
    let p1 = &lobby.state.players[1];
    assert!(p1.hand.is_empty() && p1.board.is_empty() && p1.shop.is_empty());
    // Every copy is back in the pool, or already offered in a survivor's next shop.
    let offered = lobby.state.players.iter().flat_map(|p| &p.shop).filter(|&&d| d == def).count() as u32;
    assert_eq!(lobby.state.pool[usize::from(def.0)] + offered, lobby.content.card(def).copies);
}

#[test]
fn two_players_keep_playing_regular_turns_until_the_minimum_endgame_turn() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let min_turn = lobby.content.tunables.endgame_min_turn;
    for _ in 1..min_turn {
        lobby.battle(&[0, 0]);
        assert_eq!(lobby.state.phase, Phase::Shop);
    }
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.phase, Phase::Endgame);
}

fn endgame_lobby(cards: &[(usize, &str)]) -> Lobby {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    for &(player, name) in cards {
        lobby.give(player, name);
    }
    lobby.state.turn = lobby.content.tunables.endgame_min_turn;
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.phase, Phase::Endgame);
    lobby
}

#[test]
fn endgame_damage_starts_at_one_and_rises_each_question_even_when_both_are_right() {
    let mut lobby = endgame_lobby(&[]);
    lobby.endgame_answer([false, false]);
    assert_eq!((lobby.hp(0), lobby.hp(1)), (34, 31));
    lobby.endgame_answer([true, true]);
    assert_eq!((lobby.hp(0), lobby.hp(1)), (34, 31));
    lobby.endgame_answer([true, false]);
    assert_eq!((lobby.hp(0), lobby.hp(1)), (34, 28));
}

#[test]
fn armor_carries_into_the_endgame_but_regular_cards_stop_working() {
    let mut lobby = endgame_lobby(&[(0, "iron_shield"), (0, "hall_of_mirrors"), (0, "second_opinion")]);
    let events = lobby.endgame_answer([false, true]);
    assert_eq!(damage_to(&events, P0, DamageSource::EndgameStep), vec![(1, 1, 0)]);
    assert!(damage_to(&events, P1, DamageSource::Reflect).is_empty());
    assert_eq!(lobby.apply(Command::UseHelp { player: P0, help: Help::SecondOpinion }), Err(RuleError::NotUsable));
}

#[test]
fn endgame_effects_work_and_only_one_applies_per_player() {
    let mut lobby = endgame_lobby(&[(0, "second_wind"), (0, "narrowed_choice"), (1, "narrowed_choice")]);
    let endgame = lobby.state.endgame.clone().unwrap();
    assert_eq!(endgame.removed[0], 0, "player 0 brought Second Wind, the first endgame card on the board");
    assert_eq!(endgame.removed[1].count_ones(), 1, "three options instead of four");
    lobby.endgame_answer([false, true]);
    assert_eq!(lobby.hp(0), 35, "the first hit is ignored");
    lobby.endgame_answer([false, true]);
    assert_eq!(lobby.hp(0), 33);
}

#[test]
fn a_player_can_choose_which_endgame_effect_to_bring() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.give(0, "second_wind");
    let chosen = lobby.give(0, "narrowed_choice");
    lobby.apply(Command::ChooseEndgameEffect { player: P0, card: chosen }).unwrap();
    lobby.state.turn = lobby.content.tunables.endgame_min_turn;
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.endgame.clone().unwrap().removed[0].count_ones(), 1);
}

#[test]
fn the_oracle_answers_one_endgame_question_in_total() {
    let mut lobby = endgame_lobby(&[(0, "oracle")]);
    lobby.apply(Command::UseHelp { player: P0, help: Help::Oracle }).unwrap();
    lobby.apply(Command::SubmitAnswer { player: P1, choice: None }).unwrap();
    assert_eq!(lobby.hp(0), 35);
    assert_eq!(lobby.apply(Command::UseHelp { player: P0, help: Help::Oracle }), Err(RuleError::NoCharges));
}

#[test]
fn if_both_die_on_the_same_endgame_question_the_one_with_more_hp_wins() {
    let mut lobby = endgame_lobby(&[]);
    lobby.state.players[0].hp = 1;
    lobby.state.players[1].hp = 1;
    lobby.state.players[1].temp_armor = 0;
    lobby.state.players[0].hp = 2;
    lobby.state.endgame.as_mut().unwrap().step = 5;
    let events = lobby.endgame_answer([false, false]);
    assert_eq!(lobby.state.phase, Phase::Finished);
    assert!(events.contains(&Event::MatchFinished { winner: P0 }));
    assert_eq!(lobby.state.players[1].placement, Some(2));
    assert!(!lobby.state.shared_win);
}

#[test]
fn if_both_die_on_the_same_endgame_question_with_equal_hp_both_win() {
    let mut lobby = endgame_lobby(&[]);
    lobby.state.players[0].hp = 2;
    lobby.state.players[1].hp = 2;
    lobby.state.endgame.as_mut().unwrap().step = 5;
    let events = lobby.endgame_answer([false, false]);
    assert_eq!(lobby.state.phase, Phase::Finished);
    assert!(lobby.state.shared_win);
    assert!(events.contains(&Event::SharedVictory { a: P0, b: P1 }));
    let mut placements = [lobby.state.players[0].placement, lobby.state.players[1].placement];
    placements.sort();
    assert_eq!(placements, [Some(1), Some(2)]);
}

/// A two-player lobby with escalation from turn 8: ×1.5, ×2, then ×3 for good.
fn escalating_lobby() -> Lobby {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.content.tunables.escalation_turn = 8;
    lobby.content.tunables.escalation_percent = vec![150, 200, 300];
    lobby
}

#[test]
fn escalation_leaves_damage_alone_before_its_turn() {
    let mut lobby = escalating_lobby();
    lobby.state.turn = 7;
    assert_eq!(lobby.content.tunables.escalation_percent(7), 100);
    let events = lobby.battle(&[4, 2]);
    // Margin 2 + base 2 + level 1.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(5, 0, 5)]);
}

#[test]
fn escalation_multiplies_battle_damage_one_factor_per_turn_and_the_last_repeats() {
    let t = &escalating_lobby().content.tunables;
    assert_eq!([7, 8, 9, 10, 11, 30].map(|turn| t.escalation_percent(turn)), [100, 150, 200, 300, 300, 300]);
    let mut lobby = escalating_lobby();
    lobby.state.turn = 9;
    assert_eq!(owl_core::battle::damage_percent(&lobby.state, &lobby.content), 200, "the shop shows it too");
    for (turn, expected) in [(8, 7), (9, 10), (10, 15), (25, 15)] {
        let mut lobby = escalating_lobby();
        lobby.state.turn = turn;
        let events = lobby.battle(&[4, 2]);
        // 5 times the turn's factor, rounded down: 7, 10, 15.
        assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(expected, 0, expected)], "turn {turn}");
    }
}

#[test]
fn escalation_rounds_down_but_leaves_at_least_one() {
    let mut t = escalating_lobby().content.tunables;
    t.escalation_percent = vec![101];
    assert_eq!(t.escalate(1, 8), 1);
    assert_eq!(t.escalate(0, 8), 0, "no damage stays no damage");
    assert_eq!(t.escalate(99, 8), 99);
    assert_eq!(t.escalate(100, 8), 101);
}

#[test]
fn escalation_applies_after_the_per_battle_cap() {
    let mut lobby = escalating_lobby();
    lobby.state.turn = 9;
    lobby.state.players[0].tavern = 6;
    lobby.give_golden(0, "whetstone");
    let events = lobby.battle(&[6, 0]);
    let cap = lobby.content.tunables.max_battle_damage;
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(cap * 2, 0, cap * 2)]);
}

#[test]
fn escalation_multiplies_overtime_tie_damage() {
    let mut lobby = escalating_lobby();
    lobby.content.tunables.overtime_turn = 8;
    lobby.state.turn = 9;
    lobby.state.players[0].tavern = 6;
    let events = lobby.battle(&[3, 3]);
    // Half of 0 + 2 + 6 is 4, doubled.
    assert_eq!(damage_to(&events, P0, DamageSource::Overtime), vec![(8, 0, 8)]);
    assert_eq!(damage_to(&events, P1, DamageSource::Overtime), vec![(8, 0, 8)]);
}

#[test]
fn escalation_multiplies_arrow_damage() {
    let mut lobby = escalating_lobby();
    lobby.give(0, "hunter");
    lobby.give_hand(0, "hunter");
    lobby.give_hand(0, "hunter");
    lobby.apply(Command::Buy { player: P0, index: 0 }).unwrap();
    let arrow_def = lobby.content.card_id("arrow").unwrap();
    let arrow = lobby.state.players[0].hand.iter().find(|c| c.has_def(arrow_def)).expect("arrow granted").id;
    lobby.apply(Command::Place { player: P0, card: arrow }).unwrap();
    lobby.state.turn = 9;
    let events = lobby.battle(&[0, 3]);
    assert_eq!(damage_to(&events, P1, DamageSource::Arrow), vec![(6, 0, 6)]);
}

#[test]
fn reflect_grows_with_escalated_damage() {
    let mut lobby = escalating_lobby();
    lobby.state.turn = 9;
    lobby.give(1, "hall_of_mirrors");
    let events = lobby.battle(&[4, 2]);
    // 10 incoming instead of 5, and 30% of it goes back.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(10, 0, 10)]);
    assert_eq!(damage_to(&events, P0, DamageSource::Reflect), vec![(3, 0, 3)]);
}

#[test]
fn escalation_never_touches_the_endgame() {
    let mut lobby = escalating_lobby();
    lobby.content.tunables.escalation_turn = 1;
    lobby.state.turn = lobby.content.tunables.endgame_min_turn;
    lobby.battle(&[0, 0]);
    assert_eq!(lobby.state.phase, Phase::Endgame);
    assert_eq!(owl_core::battle::damage_percent(&lobby.state, &lobby.content), 100);
    let endgame = lobby.state.endgame.clone().unwrap();
    let darwin_side = endgame.players.iter().position(|&p| p == P1).unwrap();
    let mut right = [true, true];
    right[darwin_side] = false;
    let first = lobby.endgame_answer(right);
    assert_eq!(damage_to(&first, P1, DamageSource::EndgameStep), vec![(1, 0, 1)]);
    let second = lobby.endgame_answer(right);
    assert_eq!(damage_to(&second, P1, DamageSource::EndgameStep), vec![(2, 0, 2)]);
}

#[test]
fn ready_then_buy_clears_ready() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let events = lobby.apply(Command::SetReady { player: P0, ready: true }).expect("ready in the shop");
    assert_eq!(events, vec![Event::ReadyChanged { player: P0, ready: true }]);
    assert!(lobby.state.players[0].ready);
    let events = lobby.apply(Command::Buy { player: P0, index: 0 }).expect("buys");
    assert!(!lobby.state.players[0].ready);
    assert!(events.contains(&Event::ReadyChanged { player: P0, ready: false }));
}

#[test]
fn a_shop_command_by_a_player_not_ready_sends_no_ready_event() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let events = lobby.apply(Command::Buy { player: P0, index: 0 }).expect("buys");
    assert!(!events.iter().any(|e| matches!(e, Event::ReadyChanged { .. })));
}

#[test]
fn setting_an_unchanged_ready_mark_sends_no_event() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let events = lobby.apply(Command::SetReady { player: P0, ready: false }).expect("not ready");
    assert!(events.is_empty(), "already not ready");
    lobby.apply(Command::SetReady { player: P0, ready: true }).expect("ready");
    let events = lobby.apply(Command::SetReady { player: P0, ready: true }).expect("ready again");
    assert!(events.is_empty(), "already ready");
    assert!(lobby.state.players[0].ready);
}

#[test]
fn all_ready_ignores_bots_and_dead() {
    let mut lobby = Lobby::new(&["benz", "darwin", "newton"]);
    lobby.state.players[2].alive = false;
    let bots = [false, true, false];
    assert!(!owl_core::shop::all_ready(&lobby.state, &bots));
    lobby.apply(Command::SetReady { player: P0, ready: true }).expect("ready");
    assert!(owl_core::shop::all_ready(&lobby.state, &bots));
    lobby.apply(Command::SetReady { player: P0, ready: false }).expect("not ready");
    assert!(!owl_core::shop::all_ready(&lobby.state, &bots));
}

#[test]
fn locked_out_counts_as_ready() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.state.players[0].locked_out = true;
    assert!(owl_core::shop::all_ready(&lobby.state, &[false, true]));
}

#[test]
fn ready_resets_next_turn() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::SetReady { player: P0, ready: true }).expect("ready");
    lobby.apply(Command::SetReady { player: P1, ready: true }).expect("ready");
    let turn = lobby.state.turn;
    lobby.battle(&[3, 3]);
    assert_eq!((lobby.state.phase, lobby.state.turn), (Phase::Shop, turn + 1));
    assert!(lobby.state.players.iter().all(|p| !p.ready));
}

#[test]
fn set_ready_outside_shop_is_wrong_phase() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    lobby.apply(Command::EndShop).expect("shop ends");
    assert_eq!(lobby.apply(Command::SetReady { player: P0, ready: true }), Err(RuleError::WrongPhase));
}

// ---- Hooks for the new heroes ----------------------------------------------

/// Adds a test-only card to the lobby's content: never in the shop, never
/// golden. A pooled one has one copy, which is out of the pool (in play).
fn test_card(lobby: &mut Lobby, id: &str, effects: Vec<Effect>, one_battle: bool, pooled: bool) -> owl_core::CardDefId {
    let base = lobby.content.card_id("whetstone").expect("card exists");
    let mut def = lobby.content.card(base).clone();
    def.id = id.into();
    def.name = id.into();
    def.golden_effects = effects.clone();
    def.effects = effects;
    def.golden_text = None;
    def.can_golden = false;
    def.category = None;
    def.in_shop = false;
    def.max_on_board = None;
    def.merges_into = None;
    def.copies = u32::from(pooled);
    def.one_battle = one_battle;
    lobby.content.cards.push(def);
    lobby.state.pool.push(0);
    owl_core::CardDefId((lobby.content.cards.len() - 1) as u16)
}

fn give_test_hand(lobby: &mut Lobby, player: usize, def: owl_core::CardDefId) -> CardInstanceId {
    let card = lobby.state.new_instance(def, false, 1, &lobby.content);
    let id = card.id;
    lobby.state.players[player].hand.push(card);
    id
}

fn give_test_board(lobby: &mut Lobby, player: usize, def: owl_core::CardDefId) -> CardInstanceId {
    let card = lobby.state.new_instance(def, false, 1, &lobby.content);
    let id = card.id;
    lobby.state.players[player].board.push(card);
    id
}

#[test]
fn silenced_opponent_loses_battle_passive() {
    use owl_core::stats::fold_stats_with;
    let lobby = Lobby::new(&["herophilus", "pheidippides", "marie_curie", "newton"]);
    let [herophilus, pheidippides, curie] = [0, 1, 2].map(|i| &lobby.state.players[i]);
    assert_eq!(fold_stats(herophilus, &lobby.content).reflect, 25);
    assert_eq!(fold_stats_with(herophilus, &lobby.content, false).reflect, 0);
    let marathon = fold_stats(pheidippides, &lobby.content);
    assert!(marathon.damage_per_correct > 0 && marathon.damage_per_correct_max > 0);
    let silenced = fold_stats_with(pheidippides, &lobby.content, false);
    assert_eq!((silenced.damage_per_correct, silenced.damage_per_correct_max), (0, 0));
    assert_eq!(fold_stats(curie, &lobby.content).armor_pierce_percent, 50);
    assert_eq!(fold_stats_with(curie, &lobby.content, false).armor_pierce_percent, 0);
    // The +25% category bonus is not a battle passive and stays.
    assert_eq!(fold_stats_with(herophilus, &lobby.content, false).weights, fold_stats(herophilus, &lobby.content).weights);
    assert_eq!(fold_stats_with(herophilus, &lobby.content, true), fold_stats(herophilus, &lobby.content));
}

#[test]
fn strike_hits_on_tie_and_loss() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let knight = test_card(&mut lobby, "test_knight", vec![Effect::Strike { amount: 2 }], false, false);
    give_test_board(&mut lobby, 0, knight);
    assert_eq!(lobby.stats(0).strike, 2);
    let events = lobby.battle(&[3, 3]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(2, 0, 2)], "a tie");
    assert!(damage_to(&events, P0, DamageSource::Battle).is_empty());

    let events = lobby.battle(&[2, 4]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(2, 0, 2)], "a loss");
    assert_eq!(damage_to(&events, P0, DamageSource::Battle), vec![(5, 0, 5)]);
}

#[test]
fn strike_adds_to_win_up_to_cap() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let knight = test_card(&mut lobby, "test_knight", vec![Effect::Strike { amount: 2 }], false, false);
    give_test_board(&mut lobby, 0, knight);
    let events = lobby.battle(&[4, 2]);
    // Margin 2 + base 2 + level 1 = 5, and 2 from the strike in the same hit.
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(7, 0, 7)]);

    lobby.state.players[0].tavern = 6;
    let events = lobby.battle(&[6, 0]);
    let cap = lobby.content.tunables.max_battle_damage;
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(cap, 0, cap)]);

    // A strike against a player who takes no damage at all deals nothing.
    let windmill = test_card(&mut lobby, "test_windmill", vec![Effect::DamageCap { amount: 0 }], false, false);
    give_test_board(&mut lobby, 1, windmill);
    let events = lobby.battle(&[3, 3]);
    assert!(damage_to(&events, P1, DamageSource::Battle).iter().all(|&(amount, ..)| amount == 0));
}

#[test]
fn advance_stage_card_vanishes_and_counts() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let note = test_card(&mut lobby, "test_note", vec![Effect::AdvanceHeroStage], false, false);
    // It needs no board slot.
    for _ in 0..lobby.content.tunables.board_slots {
        lobby.give(0, "whetstone");
    }
    for stage in 1..=2 {
        let card = give_test_hand(&mut lobby, 0, note);
        let events = lobby.apply(Command::Place { player: P0, card }).expect("placed");
        assert!(events.contains(&Event::CardUsed { player: P0, card }));
        let p = &lobby.state.players[0];
        assert_eq!(p.hero_state.stage, stage);
        assert!(p.find_card(card).is_none(), "the card is gone");
        assert_eq!(p.board.len(), lobby.content.tunables.board_slots as usize);
    }
}

#[test]
fn displaces_removes_target_either_order() {
    for knight_first in [true, false] {
        let mut lobby = Lobby::new(&["benz", "darwin"]);
        let windmill = test_card(&mut lobby, "test_windmill", vec![Effect::DamageCap { amount: 0 }], true, false);
        let knight = test_card(
            &mut lobby,
            "test_knight",
            vec![Effect::Strike { amount: 2 }, Effect::Displaces { card: "test_windmill".into() }],
            true,
            false,
        );
        let k = give_test_hand(&mut lobby, 0, knight);
        let w = give_test_hand(&mut lobby, 0, windmill);
        let order = if knight_first { [k, w] } else { [w, k] };
        let mut events = Vec::new();
        for card in order {
            events.extend(lobby.apply(Command::Place { player: P0, card }).expect("placed"));
        }
        let p = &lobby.state.players[0];
        assert!(p.find_card(w).is_none(), "the windmill goes, knight first: {knight_first}");
        assert!(p.board.iter().any(|c| c.id == k), "the knight stays");
        assert!(events.contains(&Event::CardUsed { player: P0, card: w }));
    }
}

#[test]
fn one_battle_card_removed_after_battle() {
    let mut lobby = Lobby::new(&["benz", "darwin"]);
    let once = test_card(&mut lobby, "test_once", vec![Effect::DamageOnWin { amount: 1 }], true, true);
    let kept = lobby.give(0, "whetstone");
    let first = give_test_board(&mut lobby, 0, once);
    let skulled = give_test_board(&mut lobby, 1, once);
    lobby.state.players[1].board.last_mut().unwrap().disabled = true;
    let events = lobby.battle(&[3, 3]);
    assert!(lobby.state.players[0].find_card(first).is_none());
    assert!(lobby.state.players[1].find_card(skulled).is_none(), "a disabled one goes too");
    assert!(lobby.state.players[0].find_card(kept).is_some());
    assert!(events.contains(&Event::CardUsed { player: P0, card: first }));
    assert_eq!(lobby.state.pool[usize::from(once.0)], 2, "both copies are back in the pool");
}

// ---- Beethoven and Goethe ------------------------------------------------

const FRENZIED_NOTES: Command = Command::UseAbility { player: P0, action: AbilityAction::FrenziedNotes };
const POETRY_BOOST: Command = Command::UseAbility { player: P0, action: AbilityAction::PoetryBoost };

fn category(lobby: &Lobby, slug: &str) -> usize {
    lobby.content.tunables.categories.iter().position(|c| c == slug).expect("category exists")
}

#[test]
fn beethoven_needs_four_rounds_of_payment() {
    let mut lobby = Lobby::new(&["beethoven", "darwin"]);
    lobby.state.players[0].gold = 2;
    assert_eq!(lobby.apply(FRENZIED_NOTES), Err(RuleError::NotEnoughGold));
    for round in 1..=4 {
        lobby.state.players[0].gold = 10;
        let events = lobby.apply(FRENZIED_NOTES).expect("a payment");
        assert!(events.contains(&Event::AbilityUsed { player: P0 }));
        assert_eq!(lobby.state.players[0].gold, 7, "a payment costs 3");
        assert_eq!(lobby.state.players[0].hero_state.uses, round);
        let again = if round < 4 { RuleError::UsedThisRound } else { RuleError::NoCharges };
        assert_eq!(lobby.apply(FRENZIED_NOTES), Err(again), "once a round, four in all");
        lobby.battle(&[3, 3]);
    }
    lobby.state.players[0].gold = 10;
    assert_eq!(lobby.apply(FRENZIED_NOTES), Err(RuleError::NoCharges), "paid in full");
    assert_eq!(lobby.state.players[0].gold, 10);
}

#[test]
fn notes_only_in_beethovens_shop_after_unlock() {
    let mut lobby = Lobby::new(&["beethoven", "darwin"]);
    let note = lobby.content.card_id("note").expect("the note exists");
    let saw_note = |lobby: &mut Lobby, player: PlayerId| {
        (0..60).any(|_| {
            lobby.state.players[player.index()].gold = 10;
            lobby.apply(Command::Reroll { player }).unwrap();
            lobby.state.players[player.index()].extras.contains(&note)
        })
    };
    assert!(!saw_note(&mut lobby, P0), "not before the fourth payment");
    lobby.state.players[0].hero_state.uses = 4;
    assert!(!saw_note(&mut lobby, P1), "never in another player's shop");
    assert!(saw_note(&mut lobby, P0), "60 rolls at 30% offer one");
    for _ in 0..20 {
        lobby.state.players[0].gold = 10;
        lobby.apply(Command::Reroll { player: P0 }).unwrap();
        assert!(lobby.state.players[0].extras.iter().filter(|&&d| d == note).count() <= 1, "one at a time");
    }
    // Bought and placed, it moves him to his first phase and takes no slot.
    let index = lobby.state.players[0].extras.iter().position(|&d| d == note).expect("still offered");
    lobby.state.players[0].gold = 10;
    lobby.apply(Command::BuyExtra { player: P0, index }).unwrap();
    assert_eq!(lobby.state.players[0].gold, 9, "a note costs 1");
    let card = lobby.state.players[0].hand.last().unwrap().id;
    lobby.apply(Command::Place { player: P0, card }).unwrap();
    assert_eq!(lobby.state.players[0].hero_state.stage, 1);
    assert!(lobby.state.players[0].board.is_empty());
}

#[test]
fn note_cycles_three_phases() {
    let mut lobby = Lobby::new(&["beethoven", "darwin"]);
    let music = category(&lobby, "music");
    let before = lobby.stats(0);
    assert_eq!((before.strike, before.weights[music], before.reflect), (0, 25, 0), "no phase before a note");
    let expected = [(2, 25, 0), (0, 40, 0), (0, 25, 15), (2, 25, 0)];
    for (stage, want) in (1..).zip(expected) {
        let card = lobby.give_hand(0, "note");
        lobby.apply(Command::Place { player: P0, card }).unwrap();
        assert_eq!(lobby.state.players[0].hero_state.stage, stage);
        let s = lobby.stats(0);
        assert_eq!((s.strike, s.weights[music], s.reflect), want, "stage {stage}");
    }
}

#[test]
fn goethe_boost_lasts_one_battle() {
    let mut lobby = Lobby::new(&["goethe", "darwin"]);
    let poetry = category(&lobby, "poetry");
    assert_eq!(lobby.stats(0).weights[poetry], 25);
    lobby.state.players[0].gold = 2;
    assert_eq!(lobby.apply(POETRY_BOOST), Err(RuleError::NotEnoughGold));
    lobby.state.players[0].gold = 10;
    lobby.apply(POETRY_BOOST).unwrap();
    assert_eq!(lobby.state.players[0].gold, 7);
    assert_eq!(lobby.stats(0).weights[poetry], 40);
    assert!(lobby.apply(POETRY_BOOST).is_err(), "once a round");
    lobby.battle(&[3, 3]);
    assert!(!lobby.state.players[0].hero_state.boosted);
    assert_eq!(lobby.stats(0).weights[poetry], 25, "back after the battle");
    lobby.state.players[0].gold = 10;
    lobby.apply(POETRY_BOOST).expect("a new round, a new boost");
    lobby.state.players[0].hero_state.used_turn = 0;
    assert_eq!(lobby.apply(POETRY_BOOST), Err(RuleError::AlreadyUsed), "already boosted");
}

#[test]
fn prometheus_keeps_newton_sword_and_goethe_boost() {
    use owl_core::stats::fold_stats_with;
    let mut lobby = Lobby::new(&["goethe", "newton"]);
    let poetry = category(&lobby, "poetry");
    lobby.state.players[0].gold = 10;
    lobby.apply(POETRY_BOOST).unwrap();
    lobby.state.players[1].hero_state.uses = 2;
    // An active's result is not a battle passive, so silence leaves it.
    let newton = fold_stats_with(&lobby.state.players[1], &lobby.content, false);
    assert_eq!(newton.sword_uses, 2);
    let goethe = fold_stats_with(&lobby.state.players[0], &lobby.content, false);
    assert_eq!(goethe.weights[poetry], 40);
    let goethe = &lobby.content.hero(lobby.state.players[0].hero.unwrap());
    assert_eq!(goethe.category_bonus, 25);
}

#[test]
fn prometheus_silences_beethoven_phase() {
    use owl_core::stats::fold_stats_with;
    let mut lobby = Lobby::new(&["beethoven", "goethe"]);
    let music = category(&lobby, "music");
    for stage in 1..=3 {
        lobby.state.players[0].hero_state.stage = stage;
        let s = fold_stats_with(&lobby.state.players[0], &lobby.content, false);
        assert_eq!((s.strike, s.weights[music], s.reflect), (0, 25, 0), "stage {stage}");
    }
    // In a real battle, Syncope deals nothing on a tie against Goethe.
    lobby.state.players[0].hero_state.stage = 1;
    let events = lobby.battle(&[3, 3]);
    assert!(damage_to(&events, P1, DamageSource::Battle).iter().all(|&(amount, ..)| amount == 0));
    assert_eq!(lobby.hp(1), lobby.state.players[1].max_hp);
}

#[test]
fn syncope_strikes_on_a_tie_without_goethe() {
    let mut lobby = Lobby::new(&["beethoven", "darwin"]);
    lobby.state.players[0].hero_state.stage = 1;
    let events = lobby.battle(&[3, 3]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(2, 0, 2)]);
}

#[test]
fn herophilus_reflects_nothing_against_goethe() {
    let mut lobby = Lobby::new(&["goethe", "herophilus"]);
    let events = lobby.battle(&[4, 2]);
    assert!(!damage_to(&events, P1, DamageSource::Battle).is_empty(), "Herophilus takes the hit");
    assert!(damage_to(&events, P0, DamageSource::Reflect).is_empty(), "and sends nothing back");

    let mut lobby = Lobby::new(&["darwin", "herophilus"]);
    let events = lobby.battle(&[4, 2]);
    assert!(!damage_to(&events, P0, DamageSource::Reflect).is_empty(), "he reflects against anyone else");
}

// ---- Miguel de Cervantes -------------------------------------------------

const DON_QUIXOTE: Command = Command::UseAbility { player: P0, action: AbilityAction::DonQuixote };

/// Places a card from the hand, through the command a host would send.
fn place(lobby: &mut Lobby, player: usize, card: CardInstanceId) -> Vec<Event> {
    lobby.apply(Command::Place { player: PlayerId(player as u8), card }).expect("placed")
}

#[test]
fn don_quixote_three_times_once_a_round() {
    let mut lobby = Lobby::new(&["cervantes", "darwin"]);
    let knight = lobby.content.card_id("knight").expect("the knight exists");
    let windmill = lobby.content.card_id("windmill").expect("the windmill exists");
    for round in 1..=3 {
        let gold = lobby.state.players[0].gold;
        let before = lobby.state.players[0].hand.len();
        let events = lobby.apply(DON_QUIXOTE).expect("a use");
        assert!(events.contains(&Event::AbilityUsed { player: P0 }));
        let p = &lobby.state.players[0];
        assert_eq!(p.gold, gold, "it is free");
        assert_eq!(p.hand.len(), before + 2);
        assert_eq!(p.hero_state.uses, round);
        for def in [knight, windmill] {
            let card = p.hand.iter().find(|c| c.has_def(def)).expect("in the hand").id;
            assert!(events.contains(&Event::CardGained { player: P0, card, def }));
        }
        let again = if round < 3 { RuleError::UsedThisRound } else { RuleError::NoCharges };
        assert_eq!(lobby.apply(DON_QUIXOTE), Err(again), "once a round, three in all");
        lobby.state.players[0].hand.clear();
        lobby.battle(&[3, 3]);
    }
    assert_eq!(lobby.apply(DON_QUIXOTE), Err(RuleError::NoCharges), "three uses a game");
}

#[test]
fn don_quixote_needs_two_free_places_in_hand() {
    let mut lobby = Lobby::new(&["cervantes", "darwin"]);
    let hand = lobby.content.tunables.hand_size as usize;
    for _ in 0..hand - 1 {
        lobby.give_hand(0, "whetstone");
    }
    assert_eq!(lobby.apply(DON_QUIXOTE), Err(RuleError::HandFull));
    assert_eq!(lobby.state.players[0].hero_state.uses, 0, "no charge spent");
    lobby.state.players[0].hand.pop();
    lobby.apply(DON_QUIXOTE).expect("two places free");
    assert_eq!(lobby.state.players[0].hand.len(), hand);
}

#[test]
fn windmill_blocks_all_damage_including_reflect_and_arrow() {
    // He loses to a reflect player: the battle deals him nothing.
    let mut lobby = Lobby::new(&["cervantes", "herophilus"]);
    let windmill = lobby.give_hand(0, "windmill");
    place(&mut lobby, 0, windmill);
    let hp = lobby.hp(0);
    let events = lobby.battle(&[2, 4]);
    assert!(damage_to(&events, P0, DamageSource::Battle).iter().all(|&(amount, ..)| amount == 0));
    assert_eq!(lobby.hp(0), hp);

    // He wins against a reflect player: he deals his own, nothing comes back.
    let windmill = lobby.give_hand(0, "windmill");
    place(&mut lobby, 0, windmill);
    let hp_herophilus = lobby.hp(1);
    let events = lobby.battle(&[4, 2]);
    assert!(lobby.hp(1) < hp_herophilus, "he still deals his own damage");
    assert!(damage_to(&events, P0, DamageSource::Reflect).iter().all(|&(amount, ..)| amount == 0));
    assert_eq!(lobby.hp(0), hp);

    // An Arrow from the opponent deals nothing either.
    let mut lobby = Lobby::new(&["cervantes", "darwin"]);
    let windmill = lobby.give_hand(0, "windmill");
    place(&mut lobby, 0, windmill);
    lobby.give(1, "arrow");
    let hp = lobby.hp(0);
    let events = lobby.battle(&[3, 3]);
    let arrow = damage_to(&events, P0, DamageSource::Arrow);
    assert!(!arrow.is_empty(), "the arrow was fired");
    assert!(arrow.iter().all(|&(amount, ..)| amount == 0));
    assert_eq!(lobby.hp(0), hp);
}

#[test]
fn knight_strikes_on_loss() {
    let mut lobby = Lobby::new(&["cervantes", "darwin"]);
    let knight = lobby.give_hand(0, "knight");
    place(&mut lobby, 0, knight);
    let events = lobby.battle(&[2, 4]);
    assert_eq!(damage_to(&events, P1, DamageSource::Battle), vec![(2, 0, 2)]);
}

#[test]
fn knight_topples_windmill_either_order() {
    for knight_first in [true, false] {
        let mut lobby = Lobby::new(&["cervantes", "darwin"]);
        let k = lobby.give_hand(0, "knight");
        let w = lobby.give_hand(0, "windmill");
        let order = if knight_first { [k, w] } else { [w, k] };
        let events: Vec<Event> = order.into_iter().flat_map(|card| place(&mut lobby, 0, card)).collect();
        let p = &lobby.state.players[0];
        assert!(p.find_card(w).is_none(), "the windmill falls, knight first: {knight_first}");
        assert!(p.board.iter().any(|c| c.id == k), "the knight stays");
        assert!(events.contains(&Event::CardUsed { player: P0, card: w }));
    }
}

#[test]
fn knight_and_windmill_gone_after_battle() {
    let mut lobby = Lobby::new(&["cervantes", "darwin"]);
    for name in ["knight", "windmill"] {
        let def = lobby.content.card_id(name).unwrap();
        let card = lobby.give_hand(0, name);
        place(&mut lobby, 0, card);
        let events = lobby.battle(&[3, 3]);
        assert!(lobby.state.players[0].find_card(card).is_none(), "{name} lasts one battle");
        assert!(events.contains(&Event::CardUsed { player: P0, card }));
        assert_eq!(lobby.state.pool[usize::from(def.0)], 0, "{name} never enters the pool");
    }
    // They sell for nothing.
    let card = lobby.give_hand(0, "knight");
    let gold = lobby.state.players[0].gold;
    lobby.apply(Command::Sell { player: P0, card }).unwrap();
    assert_eq!(lobby.state.players[0].gold, gold);
}

// ---- Homer ---------------------------------------------------------------

const BARD_OF_THE_CENTURIES: Command = Command::UseAbility { player: P0, action: AbilityAction::BardOfTheCenturies };

#[test]
fn homer_needs_three_rounds_of_payment() {
    let mut lobby = Lobby::new(&["homer", "darwin"]);
    lobby.state.players[0].gold = 2;
    assert_eq!(lobby.apply(BARD_OF_THE_CENTURIES), Err(RuleError::NotEnoughGold));
    for round in 1..=3 {
        lobby.state.players[0].gold = 10;
        let events = lobby.apply(BARD_OF_THE_CENTURIES).expect("a payment");
        assert!(events.contains(&Event::AbilityUsed { player: P0 }));
        assert_eq!(lobby.state.players[0].gold, 7, "a payment costs 3");
        assert_eq!(lobby.state.players[0].hero_state.uses, round);
        let again = if round < 3 { RuleError::UsedThisRound } else { RuleError::NoCharges };
        assert_eq!(lobby.apply(BARD_OF_THE_CENTURIES), Err(again), "once a round, three in all");
        lobby.battle(&[3, 3]);
    }
    lobby.state.players[0].gold = 10;
    assert_eq!(lobby.apply(BARD_OF_THE_CENTURIES), Err(RuleError::NoCharges), "paid in full");
    assert_eq!(lobby.state.players[0].gold, 10);
}

#[test]
fn bards_in_every_shop_after_unlock_only_while_homer_alive() {
    let mut lobby = Lobby::new(&["homer", "darwin", "benz"]);
    let bard = lobby.content.card_id("bard").expect("the bard exists");
    let saw_bard = |lobby: &mut Lobby, player: PlayerId| {
        (0..80).any(|_| {
            lobby.state.players[player.index()].gold = 10;
            lobby.apply(Command::Reroll { player }).unwrap();
            lobby.state.players[player.index()].extras.contains(&bard)
        })
    };
    assert!(!saw_bard(&mut lobby, P1), "not before the third payment");
    assert!(!saw_bard(&mut lobby, P0), "not in his own shop either");
    lobby.state.players[0].hero_state.uses = 3;
    assert!(saw_bard(&mut lobby, P1), "in another player's shop");
    assert!(saw_bard(&mut lobby, P0), "and in his own");
    for _ in 0..20 {
        lobby.state.players[1].gold = 10;
        lobby.apply(Command::Reroll { player: P1 }).unwrap();
        assert!(lobby.state.players[1].extras.iter().filter(|&&d| d == bard).count() <= 1, "one at a time");
    }
    // Bought, it costs 1 gold.
    lobby.state.players[1].extras = vec![bard];
    lobby.state.players[1].gold = 10;
    lobby.apply(Command::BuyExtra { player: P1, index: 0 }).unwrap();
    assert_eq!(lobby.state.players[1].gold, 9, "a bard costs 1");
    // Once Homer is out, no more Bards.
    lobby.state.players[0].alive = false;
    assert!(!saw_bard(&mut lobby, P1), "a dead Homer brings no bards");
}

#[test]
fn bard_adds_music_for_one_battle() {
    let mut lobby = Lobby::new(&["darwin", "benz"]);
    let music = category(&lobby, "music");
    let bard = lobby.content.card_id("bard").unwrap();
    assert_eq!(lobby.stats(0).weights[music], 0);
    let first = lobby.give_hand(0, "bard");
    place(&mut lobby, 0, first);
    assert_eq!(lobby.state.players[0].board.len(), 1, "it takes a board slot");
    assert_eq!(lobby.stats(0).weights[music], 25);
    let second = lobby.give_hand(0, "bard");
    place(&mut lobby, 0, second);
    assert_eq!(lobby.stats(0).weights[music], 50, "several stack");
    let events = lobby.battle(&[3, 3]);
    for card in [first, second] {
        assert!(lobby.state.players[0].find_card(card).is_none(), "it lasts one battle");
        assert!(events.contains(&Event::CardUsed { player: P0, card }));
    }
    assert_eq!(lobby.stats(0).weights[music], 0, "gone after the battle");
    assert_eq!(lobby.state.pool[usize::from(bard.0)], 0, "it never enters the pool");
    // It sells for nothing.
    let card = lobby.give_hand(0, "bard");
    let gold = lobby.state.players[0].gold;
    lobby.apply(Command::Sell { player: P0, card }).unwrap();
    assert_eq!(lobby.state.players[0].gold, gold);
}

#[test]
fn benz_cannot_combine_a_card_that_lasts_one_battle() {
    for id in ["bard", "knight"] {
        let mut lobby = Lobby::new(&["benz", "darwin"]);
        let short = lobby.give_hand(0, id);
        let other = lobby.give(0, "hourglass");
        let before = lobby.state.clone();
        for action in [AbilityAction::Combine { a: other, b: short }, AbilityAction::Combine { a: short, b: other }] {
            assert_eq!(lobby.apply(Command::UseAbility { player: P0, action }), Err(RuleError::NotUsable), "{id}");
        }
        assert_eq!(lobby.state, before, "nothing changes when {id} is refused");
    }
}

#[test]
fn bard_is_not_a_music_card_for_the_tribe_bonus() {
    let mut lobby = Lobby::new(&["darwin", "benz"]);
    for _ in 0..3 {
        lobby.give(0, "bard");
    }
    assert!(lobby.stats(0).bonus_categories.is_empty(), "three bards give no category bonus");
    lobby.give(0, "apprentice_music");
    lobby.give(0, "scholar_music");
    assert!(lobby.stats(0).bonus_categories.is_empty(), "nor do they complete one");
}

#[test]
fn royalty_paid_next_shop_for_others_not_self() {
    let mut lobby = Lobby::new(&["homer", "darwin", "benz"]);
    let bard = lobby.content.card_id("bard").unwrap();
    lobby.state.players[0].hero_state.uses = 3;
    // Another player's Bard pays him 1 at his next shop.
    lobby.state.players[1].extras = vec![bard];
    lobby.state.players[1].gold = 10;
    lobby.apply(Command::BuyExtra { player: P1, index: 0 }).unwrap();
    assert_eq!(lobby.state.players[0].bonus_gold, 1);
    // His own Bard pays him nothing.
    lobby.state.players[0].extras = vec![bard];
    lobby.state.players[0].gold = 10;
    lobby.apply(Command::BuyExtra { player: P0, index: 0 }).unwrap();
    assert_eq!(lobby.state.players[0].bonus_gold, 1);
    // A normal card pays nothing.
    lobby.state.players[2].gold = 10;
    lobby.apply(Command::Buy { player: PlayerId(2), index: 0 }).unwrap();
    assert_eq!(lobby.state.players[0].bonus_gold, 1);
    lobby.battle(&[3, 3, 3]);
    let (homer, benz) = (lobby.state.players[0].gold, lobby.state.players[2].gold);
    assert_eq!(homer, benz + 1, "paid at his next shop");

    // Out of the match, he collects nothing.
    lobby.state.players[0].alive = false;
    lobby.state.players[1].extras = vec![bard];
    lobby.state.players[1].gold = 10;
    lobby.apply(Command::BuyExtra { player: P1, index: 0 }).unwrap();
    assert_eq!(lobby.state.players[0].bonus_gold, 0);
}

#[test]
fn homer_endgame_bias_is_30_poetry_20_music() {
    let lobby = Lobby::new(&["homer", "darwin"]);
    let (poetry, music) = (category(&lobby, "poetry"), category(&lobby, "music"));
    let bias = owl_core::heroes::endgame_bias(&lobby.state.players[0], &lobby.content);
    assert_eq!(bias, vec![(Category(poetry as u8), 30), (Category(music as u8), 20)]);
    // Regular battles keep 25% Poetry.
    let stats = lobby.stats(0);
    assert_eq!((stats.weights[poetry], stats.weights[music]), (25, 0));
}

#[test]
fn other_heroes_endgame_bias_unchanged() {
    let content = load_content();
    for hero in content.heroes.iter().filter(|h| h.id != "homer") {
        let mut lobby = Lobby::new(&[hero.id.as_str(), if hero.id == "darwin" { "benz" } else { "darwin" }]);
        // Goethe's boost is for a regular battle only.
        lobby.state.players[0].hero_state.boosted = true;
        let bias = owl_core::heroes::endgame_bias(&lobby.state.players[0], &lobby.content);
        assert_eq!(bias, vec![(hero.category, hero.category_bonus)], "{}", hero.id);
    }
}

#[test]
fn homer_endgame_category_comes_from_his_data_and_a_bad_one_fails_to_load() {
    let content = load_content();
    let homer = &content.heroes[usize::from(content.hero_id("homer").unwrap().0)];
    let music = content.tunables.categories.iter().position(|c| c == "music").unwrap();
    assert_eq!(homer.endgame_category, Some(Category(music as u8)));

    // A copy of the data with a misspelt category, then with none, does not load.
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let dir = std::env::temp_dir().join(format!("owl-homer-data-{}", std::process::id()));
    for sub in ["cards", "heroes"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
        for entry in std::fs::read_dir(data.join(sub)).unwrap() {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.join(sub).join(path.file_name().unwrap())).unwrap();
        }
    }
    std::fs::copy(data.join("tunables.toml"), dir.join("tunables.toml")).unwrap();
    let homer_file = dir.join("heroes/homer.toml");
    let original = std::fs::read_to_string(&homer_file).unwrap();
    std::fs::write(&homer_file, original.replace("endgame_category = \"music\"", "endgame_category = \"musik\"")).unwrap();
    assert!(Content::load(&dir).is_err(), "an unknown category fails loudly");
    let without: String = original.lines().filter(|l| !l.starts_with("endgame_category")).map(|l| format!("{l}
")).collect();
    std::fs::write(&homer_file, without).unwrap();
    assert!(Content::load(&dir).is_err(), "Homer must name it");
    std::fs::write(&homer_file, &original).unwrap();
    assert!(Content::load(&dir).is_ok(), "the copy itself loads");
    std::fs::remove_dir_all(&dir).unwrap();
}
