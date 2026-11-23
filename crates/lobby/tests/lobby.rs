//! Lobby behaviour, driven with a fake clock: the test decides what time it is.

use std::sync::Arc;

use owl_core::{Command, Content, Event, Help, HeroId, Phase, PlayerId, RuleError, SyntheticQuestions};
use owl_lobby::bot::{Bot, Knowledge, Strategy};
use owl_lobby::placeholder::{PlaceholderQuestions, Texts};
use owl_lobby::{Lobby, LobbyError, Millis, Outgoing, Seat, replay};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const SEED: u64 = 11;

fn content() -> Arc<Content> {
    Arc::new(Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")).expect("data loads"))
}

fn humans(count: usize) -> Lobby {
    let seats = (0..count).map(|_| Seat::Human).collect();
    Lobby::new(content(), seats, SEED, Box::new(SyntheticQuestions::default()), 0).expect("enough heroes")
}

/// The heroes a player was offered.
fn offer(lobby: &Lobby, player: PlayerId) -> Vec<HeroId> {
    lobby.state().players[player.index()].hero_offer.clone()
}

fn bot(content: &Content, seat: u8) -> Seat {
    let knowledge = [Knowledge::Low, Knowledge::Mid, Knowledge::High][usize::from(seat % 3)];
    Seat::Bot(Box::new(Bot::new(PlayerId(seat), knowledge, Strategy::Generalist, content, u64::from(seat))))
}

fn has(events: &[Outgoing], pred: impl Fn(&Event) -> bool) -> bool {
    events.iter().any(|o| pred(&o.event))
}

/// Two humans, heroes picked, in the first shop phase at time 1.
fn in_shop() -> Lobby {
    let mut lobby = humans(2);
    for player in [P0, P1] {
        let hero = offer(&lobby, player)[0];
        lobby.handle(0, player, Command::PickHero { player, hero }).unwrap();
    }
    lobby.tick(1);
    assert_eq!(lobby.state().phase, Phase::Shop);
    lobby
}

/// The same lobby with the shop over: the first question was shown at `SHOP_END`.
const SHOP_END: Millis = 1 + 30_000;
/// How long a synthetic question (20 words) is read before its answers open:
/// 6 s, plus 150 ms a word.
const READING: Millis = 9_000;
/// When the first question's answers open: the category for 2 s, then the reading time.
const ANSWERS_OPEN: Millis = SHOP_END + 2_000 + READING;
fn in_battle() -> Lobby {
    let mut lobby = in_shop();
    lobby.tick(SHOP_END);
    assert_eq!(lobby.state().phase, Phase::Battle);
    lobby
}

#[test]
fn hero_select_ends_at_its_deadline_and_gives_missing_players_a_hero_from_their_offer() {
    let mut lobby = humans(3);
    let hero = offer(&lobby, P0)[1];
    lobby.handle(5, P0, Command::PickHero { player: P0, hero }).unwrap();
    lobby.tick(29_999);
    assert_eq!(lobby.state().phase, Phase::HeroSelect);
    lobby.tick(30_000);
    assert_eq!(lobby.state().phase, Phase::Shop);
    assert_eq!(lobby.state().players[0].hero, Some(hero));
    for p in &lobby.state().players {
        assert!(p.hero.is_some_and(|h| p.hero_offer.contains(&h)), "the hero given comes from the player's offer");
    }
}

#[test]
fn a_pick_outside_the_players_offer_is_refused() {
    let mut lobby = humans(2);
    let theirs = offer(&lobby, P1)[0];
    assert_eq!(
        lobby.handle(1, P0, Command::PickHero { player: P0, hero: theirs }),
        Err(LobbyError::Rule(RuleError::HeroNotOffered))
    );
    assert_eq!(lobby.state().players[0].hero, None);
}

#[test]
fn a_player_sees_their_own_hero_offer_and_nobody_elses() {
    let lobby = humans(6);
    let view = lobby.view(P0);
    assert_eq!(view.me.hero_offer, offer(&lobby, P0));
    assert_eq!(view.me.hero_offer.len(), 2);

    // Nothing else in the view names a hero: nobody has picked yet.
    let json = serde_json::to_value(&view).expect("the view is plain data");
    assert_eq!(json["me"]["hero_offer"], serde_json::json!(offer(&lobby, P0)));
    for public in json["players"].as_array().expect("the public player list") {
        assert!(public.get("hero_offer").is_none(), "public information carries no offer");
        assert!(public["hero"].is_null());
    }
    let text = json.to_string();
    assert_eq!(text.matches("hero_offer").count(), 1, "exactly one offer is sent: the viewer's own");
}

#[test]
fn a_lobby_with_too_few_heroes_for_its_seats_cannot_be_opened() {
    let mut content = Content::clone(&content());
    content.heroes.truncate(5);
    let seats = (0..3).map(|_| Seat::Human).collect();
    let lobby = Lobby::new(Arc::new(content), seats, SEED, Box::new(SyntheticQuestions::default()), 0);
    assert_eq!(lobby.err(), Some(LobbyError::Rule(RuleError::NotEnoughHeroes)));
}

#[test]
fn bots_pick_a_hero_from_their_own_offer() {
    let content = content();
    let mut seats = vec![Seat::Human];
    seats.extend((1..6).map(|seat| bot(&content, seat)));
    let mut lobby = Lobby::new(content, seats, SEED, Box::new(SyntheticQuestions::default()), 0).expect("enough heroes");
    lobby.tick(0);
    assert_eq!(lobby.state().phase, Phase::HeroSelect, "the human has not picked yet");
    assert_eq!(lobby.state().players[0].hero, None);
    for p in &lobby.state().players[1..] {
        assert!(p.hero.is_some_and(|h| p.hero_offer.contains(&h)), "a bot picks from its own offer at once");
    }
}

#[test]
fn hero_select_ends_as_soon_as_everyone_has_picked() {
    let lobby = in_shop();
    assert_eq!(lobby.state().turn, 1);
}

#[test]
fn the_shop_phase_ends_at_its_deadline() {
    let mut lobby = in_shop();
    lobby.tick(SHOP_END - 1);
    assert_eq!(lobby.state().phase, Phase::Shop);
    lobby.tick(SHOP_END);
    assert_eq!(lobby.state().phase, Phase::Battle);
}

#[test]
fn players_cannot_send_host_commands_or_act_for_someone_else() {
    let mut lobby = in_shop();
    assert_eq!(lobby.handle(2, P0, Command::EndShop), Err(LobbyError::HostOnly));
    assert_eq!(lobby.handle(2, P0, Command::QuestionTimeout { battle: 0 }), Err(LobbyError::HostOnly));
    assert_eq!(lobby.handle(2, P0, Command::Reroll { player: P1 }), Err(LobbyError::NotYours));
    assert!(lobby.handle(2, P0, Command::Buy { player: P0, index: 0 }).is_ok());
}

#[test]
fn an_answer_is_refused_before_the_question_opens() {
    let mut lobby = in_battle();
    let answer = Command::SubmitAnswer { player: P0, choice: Some(0) };
    // The category is shown for 2 seconds before the question opens.
    assert_eq!(lobby.handle(SHOP_END + 1_999, P0, answer.clone()), Err(LobbyError::TooEarly));
    assert_eq!(lobby.handle(ANSWERS_OPEN, P0, answer), Ok(()));
}

#[test]
fn an_answer_or_a_help_is_refused_while_the_question_is_being_read() {
    let mut lobby = in_battle();
    let answer = Command::SubmitAnswer { player: P0, choice: Some(0) };
    let help = Command::UseHelp { player: P0, help: Help::SecondOpinion };
    let reading = SHOP_END + 2_000;
    for now in [reading, ANSWERS_OPEN - 1] {
        assert_eq!(lobby.handle(now, P0, answer.clone()), Err(LobbyError::StillReading));
        assert_eq!(lobby.handle(now, P0, help.clone()), Err(LobbyError::StillReading));
    }
    assert_eq!(LobbyError::StillReading.to_string(), "the answers are not open yet");
    assert!(lobby.log().iter().all(|c| !matches!(c, Command::SubmitAnswer { .. } | Command::UseHelp { .. })));
    assert_eq!(lobby.handle(ANSWERS_OPEN, P0, answer), Ok(()));
}

#[test]
fn the_answer_deadline_counts_from_the_end_of_the_reading_time() {
    assert_eq!(Millis::from(content().tunables.reading_ms(SyntheticQuestions::TYPICAL_WORDS)), READING);
    let mut lobby = in_battle();
    let shown = lobby.drain(P0).into_iter().find(|o| matches!(o.event, Event::QuestionShown { .. })).expect("shown");
    assert_eq!(shown.opens_at, Some(SHOP_END + 2_000));
    assert_eq!(shown.answers_at, Some(ANSWERS_OPEN));
    assert_eq!(shown.deadline, Some(ANSWERS_OPEN + 9_000));
    let battle = lobby.view(P0).battle.expect("in a battle");
    assert_eq!((battle.opens_at, battle.answers_at, battle.deadline), (shown.opens_at, shown.answers_at, shown.deadline));
    // Nothing times out while the question is being read.
    lobby.tick(ANSWERS_OPEN + 9_000 + 300);
    assert_eq!(lobby.state().battles[0].current, 0, "still on the first question");
}

#[test]
fn both_players_read_a_question_for_the_same_time_set_by_its_words() {
    // Placeholder sums: "What is 12 + 34?" and four numbers, 9 words.
    let texts = Texts::default();
    let seats = vec![Seat::Human, Seat::Human];
    let source = Box::new(PlaceholderQuestions::new(SEED, texts.clone()));
    let mut lobby = Lobby::new(content(), seats, SEED, source, 0).expect("enough heroes");
    for player in [P0, P1] {
        let hero = offer(&lobby, player)[0];
        lobby.handle(0, player, Command::PickHero { player, hero }).unwrap();
    }
    lobby.tick(1);
    lobby.tick(SHOP_END);
    let shown = |events: Vec<Outgoing>| {
        events.into_iter().find(|o| matches!(o.event, Event::QuestionShown { .. })).expect("shown")
    };
    let (a, b) = (shown(lobby.drain(P0)), shown(lobby.drain(P1)));
    let question = lobby.state().battles[0].plan[0].question;
    let (text, options) = texts.get(question.id).expect("text");
    let words = text.split_whitespace().count() + options.iter().map(|o| o.split_whitespace().count()).sum::<usize>();
    assert_eq!((question.words, words), (9, 9));
    let reading = 6_000 + 9 * 150;
    for item in [&a, &b] {
        assert_eq!(item.opens_at, Some(SHOP_END + 2_000));
        assert_eq!(item.answers_at, Some(SHOP_END + 2_000 + reading));
    }
    let (mine, theirs) = (lobby.view(P0).battle.unwrap(), lobby.view(P1).battle.unwrap());
    assert_eq!((mine.answers_at, theirs.answers_at), (a.answers_at, a.answers_at));
}

#[test]
fn a_ghost_is_not_shown_as_answered_until_the_answers_open() {
    let mut lobby = humans(3);
    for player in [P0, P1, PlayerId(2)] {
        let hero = offer(&lobby, player)[0];
        lobby.handle(0, player, Command::PickHero { player, hero }).unwrap();
    }
    lobby.tick(1);
    lobby.tick(SHOP_END);
    let fighter = (0..3).map(PlayerId).find(|&p| lobby.view(p).battle.is_some_and(|b| b.ghost_of.is_some()));
    let fighter = fighter.expect("someone fights a ghost");
    assert!(!lobby.view(fighter).battle.expect("in a battle").opponent_locked);
    lobby.tick(ANSWERS_OPEN);
    assert!(lobby.view(fighter).battle.expect("in a battle").opponent_locked);
}

#[test]
fn the_endgame_question_is_read_before_its_ten_seconds_start() {
    let mut lobby = in_shop();
    let mut now = SHOP_END;
    while lobby.state().phase != Phase::Endgame {
        now += 250;
        lobby.tick(now);
        assert!(now < 60 * 60 * 1000, "two idle players never reached the endgame");
    }
    let shown = lobby
        .drain(P0)
        .into_iter()
        .rev()
        .find(|o| matches!(o.event, Event::QuestionShown { .. }))
        .expect("the first endgame question");
    let (opens_at, answers_at) = (shown.opens_at.unwrap(), shown.answers_at.unwrap());
    assert_eq!(answers_at, opens_at + READING);
    assert_eq!(shown.deadline, Some(answers_at + 10_000));
    let answer = Command::SubmitAnswer { player: P0, choice: Some(0) };
    assert_eq!(lobby.handle(answers_at - 1, P0, answer.clone()), Err(LobbyError::StillReading));
    assert_eq!(lobby.handle(answers_at, P0, answer), Ok(()));
}

#[test]
fn an_answer_counts_within_the_grace_window_and_is_refused_after_it() {
    let closes = ANSWERS_OPEN + 9_000;
    let mut lobby = in_battle();
    assert_eq!(lobby.handle(closes + 300, P0, Command::SubmitAnswer { player: P0, choice: Some(0) }), Ok(()));
    assert_eq!(
        lobby.handle(closes + 301, P1, Command::SubmitAnswer { player: P1, choice: Some(0) }),
        Err(LobbyError::TooLate)
    );
}

#[test]
fn an_unanswered_question_times_out_and_the_next_one_is_paced_by_the_reveal_pause() {
    let closes = ANSWERS_OPEN + 9_000;
    let mut lobby = in_battle();
    lobby.drain(P0);
    lobby.tick(closes + 300);
    assert!(lobby.drain(P0).is_empty(), "still inside the grace window");
    lobby.tick(closes + 301);
    let events = lobby.drain(P0);
    assert!(has(&events, |e| matches!(e, Event::AnswerRevealed { a_correct: false, b_correct: false, .. })));
    // Next question: answer shown for 2.5 s, category for 2 s, 4 s to read, then 9 s to answer.
    let shown = events.iter().find(|o| matches!(o.event, Event::QuestionShown { .. })).expect("next question");
    assert_eq!(shown.opens_at, Some(closes + 301 + 4_500));
    assert_eq!(shown.answers_at, Some(closes + 301 + 4_500 + READING));
    assert_eq!(shown.deadline, Some(closes + 301 + 4_500 + READING + 9_000));
}

#[test]
fn a_question_ends_the_moment_both_players_lock_in() {
    let mut lobby = in_battle();
    lobby.drain(P0);
    let opens = ANSWERS_OPEN;
    lobby.handle(opens + 100, P0, Command::SubmitAnswer { player: P0, choice: Some(0) }).unwrap();
    assert!(!has(&lobby.drain(P0), |e| matches!(e, Event::AnswerRevealed { .. })));
    lobby.handle(opens + 200, P1, Command::SubmitAnswer { player: P1, choice: Some(0) }).unwrap();
    assert!(has(&lobby.drain(P0), |e| matches!(e, Event::AnswerRevealed { .. })));
}

#[test]
fn a_players_shop_actions_are_sent_only_to_that_player() {
    let mut lobby = in_shop();
    lobby.drain(P0);
    lobby.drain(P1);
    lobby.handle(2, P0, Command::Buy { player: P0, index: 0 }).unwrap();
    assert!(has(&lobby.drain(P0), |e| matches!(e, Event::CardBought { .. })));
    assert!(lobby.drain(P1).is_empty());
}

#[test]
fn battle_events_reach_only_the_two_players_fighting() {
    let mut lobby = humans(4);
    lobby.tick(30_000);
    for seat in 0..4 {
        lobby.drain(PlayerId(seat));
    }
    lobby.tick(60_000);
    for seat in 0..4 {
        let events = lobby.drain(PlayerId(seat));
        let battles = events.iter().filter(|o| matches!(o.event, Event::BattleStarted { .. })).count();
        let questions = events.iter().filter(|o| matches!(o.event, Event::QuestionShown { .. })).count();
        assert_eq!((battles, questions), (1, 1), "seat {seat} sees its own battle and no other");
    }
}

#[test]
fn an_opponents_board_is_visible_only_during_the_battle() {
    let mut lobby = in_shop();
    lobby.handle(2, P1, Command::Buy { player: P1, index: 0 }).unwrap();
    let card = lobby.state().players[1].hand[0].id;
    lobby.handle(3, P1, Command::Place { player: P1, card }).unwrap();

    let view = lobby.view(P0);
    assert!(view.battle.is_none(), "no board is shown in the shop");
    assert!(view.phase_deadline.is_some());
    assert_eq!(view.me.gold, 3);

    lobby.tick(SHOP_END);
    let battle = lobby.view(P0).battle.expect("in a battle");
    assert_eq!(battle.opponent, Some(P1));
    assert_eq!(battle.opponent_board, lobby.state().players[1].board);
    assert_eq!((battle.question_number, battle.question_count), (1, 6));
    assert_eq!(battle.deadline, Some(ANSWERS_OPEN + 9_000));
}

/// One idle human and five bots, run until `stop` says so. Returns the lobby and the time.
fn bots_lobby_until(stop: impl Fn(&Lobby) -> bool) -> (Lobby, Millis) {
    let content = content();
    let mut seats = vec![Seat::Human];
    seats.extend((1..6).map(|seat| bot(&content, seat)));
    let mut lobby =
        Lobby::new(content, seats, SEED, Box::new(SyntheticQuestions::default()), 0).expect("enough heroes");
    let mut now = 0;
    while !stop(&lobby) {
        now += 250;
        lobby.tick(now);
        assert!(now < 4 * 60 * 60 * 1000, "the lobby never got there");
    }
    (lobby, now)
}

#[test]
fn everyone_sees_each_players_streak_goldens_leading_category_and_last_two_battles() {
    let (lobby, _) = bots_lobby_until(|lobby| lobby.state().turn >= 6 && lobby.state().phase == Phase::Shop);
    let content = content();
    let view = lobby.view(P0);
    let mut some_category = false;
    for (public, player) in view.players.iter().zip(&lobby.state().players) {
        assert_eq!(public.win_streak, player.win_streak);
        let mut counts = vec![0u32; content.tunables.categories.len()];
        let mut goldens = 0;
        for part in player.board.iter().flat_map(|card| &card.parts) {
            goldens += u32::from(part.golden);
            if let Some(category) = content.card(part.def).category {
                counts[usize::from(category.0)] += 1;
            }
        }
        assert_eq!(public.goldens, goldens);
        let best = counts.iter().copied().max().unwrap();
        assert_eq!(public.top_category_cards, best);
        let leaders: Vec<usize> = (0..counts.len()).filter(|&i| best > 0 && counts[i] == best).collect();
        match leaders[..] {
            [only] => assert_eq!(public.top_category.map(|c| usize::from(c.0)), Some(only)),
            _ => assert_eq!(public.top_category, None, "a tie or an empty board names no category"),
        }
        some_category |= public.top_category.is_some();
        assert!(public.recent.len() <= 2, "only the last two battles are kept");
        assert!(!public.recent.is_empty() || !player.alive, "five turns in, a living player has fought");
    }
    assert!(some_category, "after five turns somebody leads with a category");
    // The summary is all that shows: no board is sent outside a battle.
    assert!(view.battle.is_none());
}

#[test]
fn an_eliminated_player_watches_the_living_after_a_delay_and_nobody_else_watches() {
    let (mut lobby, died_at) = bots_lobby_until(|lobby| lobby.state().players.iter().any(|p| !p.alive));
    assert!(!lobby.is_finished());
    let delay = Millis::from(content().tunables.spectate_delay_ms);
    let out = lobby.state().players.iter().find(|p| !p.alive).unwrap().id;
    let living = lobby.state().players.iter().find(|p| p.alive).unwrap().id;

    assert!(lobby.view(living).spectate.is_empty(), "the living watch nobody");
    assert!(lobby.view(out).spectate.is_empty(), "nothing is old enough to show yet");
    let boards_then: Vec<_> =
        lobby.state().players.iter().filter(|p| p.alive).map(|p| (p.id, p.board.clone())).collect();

    lobby.tick(died_at + delay - 1);
    assert!(lobby.view(out).spectate.is_empty(), "one millisecond short of the delay");
    lobby.tick(died_at + delay);
    let watched = lobby.view(out).spectate;
    assert!(!lobby.is_finished());
    assert!(lobby.view(living).spectate.is_empty());
    // What shows is the moment of elimination, not the present.
    let seen: Vec<_> = watched.iter().map(|w| (w.id, w.board.clone())).collect();
    assert_eq!(seen, boards_then);
    assert!(watched.iter().all(|w| w.id != out), "the eliminated are not on the list");
}

#[test]
fn an_eliminated_player_also_watches_the_battles_of_the_living() {
    let (mut lobby, now) = bots_lobby_until(|lobby| {
        lobby.state().phase == Phase::Battle && lobby.state().players.iter().any(|p| !p.alive)
    });
    let out = lobby.state().players.iter().find(|p| !p.alive).unwrap().id;
    let own_views: Vec<_> = lobby
        .state()
        .players
        .iter()
        .filter(|p| p.alive)
        .map(|p| (p.id, lobby.view(p.id).battle.map(|b| (b.question, b.question_number))))
        .collect();
    assert!(own_views.iter().any(|(_, battle)| battle.is_some()), "somebody is fighting");

    lobby.tick(now + Millis::from(content().tunables.spectate_delay_ms));
    let watched: Vec<_> = lobby
        .view(out)
        .spectate
        .into_iter()
        .map(|w| (w.id, w.battle.map(|b| (b.question, b.question_number))))
        .collect();
    // Each battle shows as its own fighter saw it at that moment, delay ago.
    assert_eq!(watched, own_views);
}

#[test]
fn nobody_watches_once_the_match_is_over() {
    let (lobby, _) = bots_lobby_until(Lobby::is_finished);
    for seat in 0..6 {
        assert!(lobby.view(PlayerId(seat)).spectate.is_empty());
    }
}

#[test]
fn bots_fill_seats_and_a_lobby_plays_itself_to_the_end() {
    let content = content();
    let mut seats = vec![Seat::Human];
    seats.extend((1..6).map(|seat| bot(&content, seat)));
    let mut lobby =
        Lobby::new(content.clone(), seats, SEED, Box::new(SyntheticQuestions::default()), 0).expect("enough heroes");

    // The human never touches the game; the clock and the bots carry it.
    let mut now = 0;
    while !lobby.is_finished() {
        now += 250;
        lobby.tick(now);
        assert!(now < 4 * 60 * 60 * 1000, "the lobby did not finish");
    }
    let mut placements: Vec<u32> = lobby.state().players.iter().filter_map(|p| p.placement).collect();
    placements.sort_unstable();
    assert_eq!(placements, vec![1, 2, 3, 4, 5, 6]);
    assert!(has(&lobby.drain(P0), |e| matches!(e, Event::MatchFinished { .. })));

    // The match log replays to exactly the same final state.
    let replayed =
        replay(&content, 6, SEED, &mut SyntheticQuestions::default(), lobby.log()).expect("the log replays");
    assert_eq!(&replayed, lobby.state());
}

#[test]
fn bots_answer_after_the_answers_open_and_before_the_deadline() {
    let content = content();
    let seats = vec![bot(&content, 0), bot(&content, 1)];
    let mut lobby = Lobby::new(content, seats, SEED, Box::new(SyntheticQuestions::default()), 0).expect("enough heroes");
    lobby.tick(0);
    lobby.tick(1);
    let shop_end = 1 + 30_000;
    lobby.tick(shop_end);
    assert_eq!(lobby.state().phase, Phase::Battle);
    lobby.drain(P0);
    // Bots think for 1.5 to 6 seconds, counted from when the answers open.
    let answers_open = shop_end + 2_000 + READING;
    lobby.tick(answers_open - 1);
    assert!(lobby.drain(P0).is_empty(), "no bot answers while the question is read");
    lobby.tick(answers_open + 1_499);
    assert!(lobby.drain(P0).is_empty(), "no bot answers this fast");
    lobby.tick(answers_open + 6_000);
    assert!(has(&lobby.drain(P0), |e| matches!(e, Event::AnswerRevealed { .. })));
}

#[test]
fn a_player_is_told_the_difficulty_of_the_question_with_its_category() {
    let mut lobby = in_battle();
    let question = lobby.state().battles[0].plan[0].question;
    for player in [P0, P1] {
        let battle = lobby.view(player).battle.expect("in a battle");
        assert_eq!((battle.category, battle.difficulty), (question.category, question.difficulty));
        let events = lobby.drain(player);
        assert!(has(&events, |e| matches!(*e, Event::CategoryRevealed { category, difficulty, .. }
            if category == question.category && difficulty == question.difficulty)));
    }
}

#[test]
fn a_bot_places_its_golden_card_opens_its_reward_card_and_picks() {
    use owl_core::{Env, GameState, LobbyRng, apply};
    let content = content();
    let mut rng = LobbyRng::from_seed(SEED);
    let mut questions = SyntheticQuestions::default();
    let mut state = GameState::new(2, &content, &mut rng).expect("enough heroes");
    let mut env = Env { content: &content, rng: &mut rng, questions: &mut questions };
    let mut bot = Bot::new(P0, Knowledge::Mid, Strategy::Generalist, &content, 5);
    bot.pick_hero(&mut state, &mut env);
    apply(&mut state, Command::StartMatch, &mut env).unwrap();

    // A golden card that has not given its reward, and a reward card from an earlier turn.
    let whetstone = content.card_id("whetstone").unwrap();
    let mut golden = state.new_instance(whetstone, true, 3, &content);
    golden.parts[0].reward_due = true;
    let golden_id = golden.id;
    let reward_card = state.new_instance(content.golden_reward, false, 1, &content);
    state.pool[usize::from(whetstone.0)] -= 3;
    state.players[0].hand.extend([golden, reward_card]);

    bot.shop(&mut state, &mut env);
    let me = &state.players[0];
    let placed = me.board.iter().find(|c| c.id == golden_id).expect("the golden card is on the board");
    assert!(!placed.parts[0].reward_due);
    assert!(!me.cards().any(|c| c.has_def(content.golden_reward)), "the reward card was opened");
    assert!(me.rewards.is_empty(), "both rewards were picked");
    let picks = bot.accepted.iter().filter(|c| matches!(c, Command::PickReward { .. })).count();
    assert_eq!(picks, 2);
}
