# Ready button, three heroes, three categories, lobby reconnect — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 60 s shop that closes early when every living human is ready; Beethoven, Goethe and Cervantes with Music, Poetry and Prose; a lobby place that survives the app going to the background.

**Architecture:** Rules in `owl-core` behind data (cards, heroes, tunables); the hero trait gains three generic hooks so no hero names another; `owl-lobby` closes the shop from a core predicate; the server keeps a dropped lobby member for a grace period; the Godot client only renders and sends commands.

**Tech Stack:** Rust (owl-core, owl-lobby, owl-protocol, owl-server, owl-sim), Godot 4 GDScript client, TOML data, Python for the one-off question generator, Playwright + Gemini for art.

**Spec:** `docs/superpowers/specs/2026-11-26-ready-and-three-heroes-design.md`

## Global Constraints

- Every rule lives in `core` with a test; never in `server`, `lobby` glue or the client.
- After any change in `core`: `cargo test -p owl-core` and `cargo clippy --all-targets`, both clean.
- After any change in `server` or `protocol`: `cargo test -p owl-server`.
- Protocol changes bump `VERSION` in `crates/protocol/src/lib.rs` (7 → 8) and are documented in `docs/protocol.md`.
- Numbers live in `data/` (tunables, hero `[params]`, card files), never as constants in code.
- Adding a hero must not edit another hero's module (moving existing passives to the new `battle_passive` hook in Task 3 is the one sanctioned exception: it is a trait refactor, not a behaviour change).
- One-sentence, plain commit messages (user's style).
- Titles: "Composer of the Ninth", "Poet of Weimar", "Author of Don Quixote". Ability names: Frenzied Notes, Prometheus, Don Quixote. Phase names: Syncope, Passionate Composer, Can't Hear the Critics.

## Review Focus

1. A ready player who then buys/sells/places must become not-ready, or the shop closes under their hands — test in Task 1.
2. A lobby of only bots plus one human: pressing Ready must close the shop at once (bots never press anything) — test in Task 2.
3. Windmill must stop reflected damage coming back at Cervantes too, not only the battle hit — test in Task 6.
4. Prometheus must not remove Newton's bought sword uses or Goethe's own paid boost (they live in `modify_stats`, not `battle_passive`) — test in Task 5.
5. The owner of an unstarted lobby dropping and returning within the grace keeps the lobby for everyone — test in Task 8.

---

### Task 1: Ready state and the 60 s shop in core

**Files:**
- Modify: `data/tunables.toml` (`shop_seconds = 60`)
- Modify: `crates/core/src/api.rs` (Command, Event)
- Modify: `crates/core/src/state.rs` (`Player.ready`)
- Modify: `crates/core/src/engine.rs` (dispatch, clear on other shop commands)
- Modify: `crates/core/src/shop.rs` (`set_ready`, `all_ready`; clear `ready` at `TurnStarted`)
- Test: `crates/core/tests/rules.rs`

**Interfaces:**
- Produces: `Command::SetReady { player: PlayerId, ready: bool }`; `Event::ReadyChanged { player: PlayerId, ready: bool }`; `Player.ready: bool` (serde default false); `pub fn shop::all_ready(state: &GameState, is_bot: &[bool]) -> bool` — true when every alive player is a bot, ready, or `locked_out` (Shakespeare's "not to be").

- [ ] **Step 1: Write failing tests** in `rules.rs`: `ready_then_buy_clears_ready` (SetReady true → Buy → `players[0].ready == false` and a `ReadyChanged{ready:false}` event); `all_ready_ignores_bots_and_dead` (`all_ready(&state, &[false, true])` true once player 0 ready); `locked_out_counts_as_ready`; `ready_resets_next_turn`; `set_ready_outside_shop_is_wrong_phase`.
- [ ] **Step 2:** `cargo test -p owl-core ready` → FAIL (no such command).
- [ ] **Step 3: Implement.** `SetReady` only in `Phase::Shop` for an alive player. In `engine::apply`, after any successful player shop command other than `SetReady` (Buy, BuyExtra, Sell, Place, UseCard, Reroll, ToggleFreeze, UpgradeTavern, PickReward, UseAbility, ChooseEndgameEffect) clear that player's `ready` and push `ReadyChanged{ready:false}` if it was set. Clear all `ready` where the turn starts. Set `shop_seconds = 60`.
- [ ] **Step 4:** `cargo test -p owl-core` and `cargo clippy --all-targets` → PASS.
- [ ] **Step 5: Commit** "Players can mark themselves ready in a 60 second shop."

### Task 2: Lobby closes the shop early; view, protocol, client button

**Files:**
- Modify: `crates/lobby/src/lib.rs` (`owner()`, `fire_deadlines`)
- Modify: `crates/lobby/src/view.rs` (ready per player in the public summary)
- Modify: `crates/protocol/src/lib.rs` (`VERSION = 8`), `docs/protocol.md`
- Modify: `client/scripts/screens/shop.gd` (Ready button, ticks on the player list), `client/scripts/online.gd` / `transport.gd` if commands are whitelisted
- Test: `crates/lobby/src/lib.rs` tests module

**Interfaces:**
- Consumes: `shop::all_ready`, `Command::SetReady`, `Event::ReadyChanged`.
- Produces: `ready: bool` on each entry of the view's player summary.

- [ ] **Step 1: Failing tests:** `shop_closes_when_only_human_is_ready` (1 human + 2 bots, SetReady at t=1s, `tick(1s)` → phase is Battle); `shop_waits_for_second_human` (2 humans, one ready → still Shop at t=59s, Battle at t=60s).
- [ ] **Step 2:** `cargo test -p owl-lobby` → FAIL.
- [ ] **Step 3:** In `fire_deadlines`, `Phase::Shop => (now >= deadline || shop::all_ready(&state, &is_bot)).then_some(EndShop)` where `is_bot[i] = matches!(seats[i], Seat::Bot(_))`; bots must have shopped this turn first (`shopped[i] == turn`) so a human's instant ready does not cut the bots' shopping. Add `SetReady` to `owner()`. Expose `ready` in the summary. Bump protocol to 8, document `SetReady` / `ReadyChanged` and the summary field. Client: a Ready stamp beside the timer that toggles `SetReady`, shows pressed state from the view, and a small tick beside each ready player in the list.
- [ ] **Step 4:** `cargo test -p owl-lobby`, `cargo test -p owl-server` → PASS; run the client (`play.bat`) against bots, press Ready, see the shop close.
- [ ] **Step 5: Commit** "The shop closes as soon as every living player is ready."

### Task 3: Generic hooks for the new heroes

**Files:**
- Modify: `crates/core/src/content.rs` (`Effect::Strike { amount }`, `Effect::Displaces { card: String }`, `Effect::AdvanceHeroStage`; `CardDef.one_battle: bool` from TOML `one_battle`, default false; `HeroKind::{Beethoven, Goethe, Cervantes}`)
- Modify: `crates/core/src/stats.rs` (`BattleStats.strike`; `fold_stats_with(player, content, passives: bool)`; `fold_stats` = with `true`)
- Modify: `crates/core/src/state.rs` (`HeroState.stage: u32`, `HeroState.boosted: bool`)
- Modify: `crates/core/src/heroes/mod.rs` (trait: `battle_passive`, `silences_opponent`, `on_battle_end`; move Herophilus, Pheidippides, Marie Curie's `modify_stats` bodies to `battle_passive`)
- Modify: `crates/core/src/battle.rs` (silence, strike, one-battle removal, `on_battle_end`)
- Modify: `crates/core/src/shop.rs` (`place`: AdvanceHeroStage and Displaces)
- Test: `crates/core/tests/rules.rs`

**Interfaces:**
- Produces:
  - `fn battle_passive(&self, hero: &HeroDef, player: &Player, stats: &mut BattleStats) {}` — skipped when silenced; runs after `modify_stats`, before caps.
  - `fn silences_opponent(&self, hero: &HeroDef) -> bool { false }`
  - `fn on_battle_end(&self, hero: &HeroDef, player: &mut Player) {}` — called in `resolve_battle` for each real side.
  - `Strike { amount }` adds to `stats.strike`. In `resolve_battle`: the winner's damage becomes `(win_damage(..) + strike).min(max_battle_damage)` before escalation; a side that did not win deals `escalate(strike.min(max_battle_damage))` as `DamageSource::Battle`.
  - `AdvanceHeroStage`: placing the card adds 1 to `hero_state.stage`, the card vanishes (like the reward card: no slot), `CardUsed` event.
  - `Displaces { card }`: after any placement, if a card with this effect and a card whose def id is `card` are both on the board, the `card` instance is removed (returned via `return_instance`) with a `CardUsed` event.
  - `one_battle = true` cards are removed from the board at the end of the battle they were on the board for (as Arrow is today).
  - Windmill needs no new effect: `DamageCap { amount = 0 }` already stops every hit including reflect back (`take_damage` with `damage_cap`).

- [ ] **Step 1: Failing tests:** `silenced_opponent_loses_battle_passive` (Herophilus vs a test hero whose `silences_opponent` is true → Herophilus `reflect == 0`; uses a test-only content where Goethe kind is set); `strike_hits_on_tie_and_loss`; `strike_adds_to_win_up_to_cap`; `advance_stage_card_vanishes_and_counts`; `displaces_removes_target_either_order`; `one_battle_card_removed_after_battle`; existing Herophilus/Pheidippides/Curie tests still pass.
- [ ] **Step 2:** `cargo test -p owl-core` → new tests FAIL.
- [ ] **Step 3:** Implement as in Interfaces. In `build_battle`, silence side B when A's hero `silences_opponent`, and side A when B's (ghost: its source player's hero), via `fold_stats_with(.., false)`.
- [ ] **Step 4:** `cargo test -p owl-core`, `cargo clippy --all-targets` → PASS.
- [ ] **Step 5: Commit** "Heroes can silence an opponent's battle passives, and cards can strike whatever the result."

### Task 4: Categories and placeholder questions

**Files:**
- Modify: `data/tunables.toml` (`categories` += `"music", "poetry", "prose"`)
- Create: `tools/placeholder_questions.py`; Create: `data/questions/{music,poetry,prose}.toml`
- Modify: `crates/lobby/src/bank.rs` (`Entry.placeholder: bool`, serde default)
- Modify: client category display names if a name map exists; add `client/assets/categories/{music,poetry,prose}.png` placeholders (same size as `history.png`) until Task 9
- Test: bank load test in `crates/lobby/src/bank.rs`

- [ ] **Step 1: Failing test:** `bank_loads_all_categories` — loading `data/questions` yields ≥100 questions for each of the 13 categories and no check errors.
- [ ] **Step 2:** `cargo test -p owl-lobby bank` → FAIL.
- [ ] **Step 3:** Script, seeded, writes 100 per file, ids `music-0001…`, `subcategory = "Arithmetic"`, `placeholder = true`, `source = "generated"`, `reviewed = false`. Easy: `a ± b` with a,b ≤ 20; medium: `a × b` (a,b 6–19) or 3-digit difference; hard: `a × b + c` or `a ÷ b − c` with exact division. Wrong options: answer ±1, ±2, ±10 (distinct, positive). Texts unique within a file. Run it.
- [ ] **Step 4:** `cargo test -p owl-lobby` and `cargo test -p owl-core` (category count changes) → PASS.
- [ ] **Step 5: Commit** "Music, Poetry and Prose join the categories with generated arithmetic as placeholder questions."

### Task 5: Beethoven and Goethe

**Files:**
- Create: `crates/core/src/heroes/beethoven.rs`, `crates/core/src/heroes/goethe.rs`; register in `heroes/mod.rs`
- Create: `data/heroes/beethoven.toml`, `data/heroes/goethe.toml`, `data/cards/note.toml`
- Modify: `crates/core/src/api.rs` (`AbilityAction::FrenziedNotes`, `AbilityAction::Prometheus`)
- Test: `crates/core/tests/rules.rs`

Data:
- Beethoven: category music, hp 35, bonus 25; params `cost = 3`, `payments = 4`, `note_chance = 30`, `syncope = 2`, `passion = 15`, `critics_reflect = 15`.
- Note: tier 0, kind token, cost 1, golden false, shop false, copies 0, effect `AdvanceHeroStage`.
- Goethe: category poetry, hp 35, bonus 25; params `cost = 3`, `boost = 15`.

Behaviour:
- Beethoven `use_ability(FrenziedNotes)`: `once_a_round`, gold ≥ cost, `uses < payments`; pay, `uses += 1`, `used_turn = turn`, `AbilityUsed`.
- `on_shop_roll`: only for his own shop (`players[idx].hero` is this hero), `uses >= payments`, no Note already in extras, `rng.percent(note_chance)` → push Note.
- `battle_passive` with `stage > 0`, phase `(stage − 1) % 3`: 0 → `strike += syncope`; 1 → `weights[music] += passion`; 2 → `reflect += critics_reflect`.
- Goethe `silences_opponent` → true. `use_ability(Prometheus)`: `once_a_round`, gold ≥ cost, not already `boosted`; pay, `boosted = true`. `modify_stats`: if boosted, `weights[poetry] += boost`. `on_battle_end`: `boosted = false`.

- [ ] **Step 1: Failing tests:** `beethoven_needs_four_rounds_of_payment`; `notes_only_in_beethovens_shop_after_unlock`; `note_cycles_three_phases` (stage 1/2/3/4 → strike 2 / music 40 / reflect 15 / strike 2); `goethe_boost_lasts_one_battle` (40 then 25); `prometheus_keeps_newton_sword_and_goethe_boost`; `prometheus_silences_beethoven_phase`.
- [ ] **Step 2:** `cargo test -p owl-core` → FAIL.
- [ ] **Step 3:** Implement the two modules and data.
- [ ] **Step 4:** `cargo test -p owl-core`, `cargo clippy --all-targets` → PASS.
- [ ] **Step 5: Commit** "Beethoven and Goethe join the heroes."

### Task 6: Cervantes

**Files:**
- Create: `crates/core/src/heroes/cervantes.rs`; register
- Create: `data/heroes/cervantes.toml`, `data/cards/knight.toml`, `data/cards/windmill.toml`
- Modify: `crates/core/src/api.rs` (`AbilityAction::DonQuixote`)
- Test: `crates/core/tests/rules.rs`

Data: Cervantes prose, hp 35, bonus 25, params `uses = 3`. Knight: tier 0, token, golden false, shop false, copies 0, `one_battle = true`, effects `Strike { amount = 2 }`, `Displaces { card = "windmill" }`. Windmill: same flags, `one_battle = true`, effect `DamageCap { amount = 0 }`. Tokens sell for nothing (existing rule).

Behaviour: `use_ability(DonQuixote)`: `once_a_round`, `uses < 3`, hand needs 2 free places; push Knight and Windmill to hand with `CardGained`, `uses += 1`, `used_turn = turn`.

- [ ] **Step 1: Failing tests:** `don_quixote_three_times_once_a_round`; `windmill_blocks_all_damage_including_reflect_and_arrow`; `knight_strikes_on_loss`; `knight_topples_windmill_either_order`; `knight_and_windmill_gone_after_battle`.
- [ ] **Step 2–4:** FAIL → implement → `cargo test -p owl-core`, clippy PASS.
- [ ] **Step 5: Commit** "Miguel de Cervantes joins the heroes."

### Task 7: Bots, client abilities, docs, sim

**Files:**
- Modify: `crates/lobby/src/bot.rs` (`hero_ability`: Beethoven pays when gold allows from turn 2; Goethe boosts when ≥ 3 gold left after buying; Cervantes uses Don Quixote and places the Knight, the Windmill when HP ≤ 1/3)
- Modify: `client/scripts/hero_texts.gd` (3 entries), `client/scripts/ability.gd` (`LIMIT`: Beethoven "payments", Cervantes "uses"; Goethe/Beethoven once a turn), `client/scripts/screens/shop.gd` (ability button → `FrenziedNotes` / `Prometheus` / `DonQuixote`; Beethoven's current phase shown under his portrait)
- Modify: `HEROES.md`, `CARDS.md`, `CLAUDE.md` (13 categories, 15 heroes, 60 s shop, ready rule)
- [ ] **Step 1:** `cargo test -p owl-lobby` (bot tests) PASS.
- [ ] **Step 2:** `cargo run --release -p owl-sim -- --lobbies 5000`; record placement per archetype and per new hero.
- [ ] **Step 3:** Play one offline match with each new hero in the client; ability button works.
- [ ] **Step 4: Commit** "Bots and the client know the three new heroes."

### Task 8: Lobby place survives a dropped connection

**Files:**
- Modify: `crates/server/src/hub.rs` (`Client.away_since: Option<Instant>` or a separate `away: HashMap<String, Instant>`; `disconnect`, `connect`, sweep), `crates/server/src/lib.rs` (sweep timer), lobby state message (`away: Vec<String>`)
- Modify: `client/scripts/online.gd` (`_notification`: on `NOTIFICATION_APPLICATION_RESUMED` / `NOTIFICATION_APPLICATION_FOCUS_IN`, if no open socket, `_retry_at = 0`), `client/scripts/screens/online_lobby.gd` (show "away")
- Modify: `docs/protocol.md` (VERSION stays 8 from Task 2 if released together)
- Test: `crates/server` hub tests

**Interfaces:** `const LOBBY_GRACE: Duration = 3 min` as a server setting; `pub fn Hub::sweep_away(&mut self, now: Instant)` leaves every member away longer than the grace.

- [ ] **Step 1: Failing tests:** `dropped_member_keeps_place_within_grace`; `dropped_member_leaves_after_grace`; `dropped_owner_keeps_lobby_open`; `match_starts_with_away_member_who_rejoins`.
- [ ] **Step 2:** `cargo test -p owl-server` → FAIL.
- [ ] **Step 3:** `disconnect` in an unstarted lobby marks away instead of `leave`; `connect` clears it (lobby code already recovered); `sweep_away` runs every 10 s; lobby state lists who is away.
- [ ] **Step 4:** `cargo test -p owl-server` PASS; on the phone, switch apps for 30 s in a lobby and come back.
- [ ] **Step 5: Commit** "A player who leaves the app for a moment keeps their lobby place."

### Task 9: Art

**Files:** `client/assets/heroes/{beethoven,goethe,cervantes}.jpg` and `_face.jpg`; `client/assets/abilities/{beethoven,goethe,cervantes}.png`; `client/assets/categories/{music,poetry,prose}.png`; `client/assets/cards/{note,knight,windmill}.jpg`; scripts `.playwright-mcp/gen20.js…`.

- [ ] **Step 1:** Generate with Gemini in the browser, same prompts style and sizes as the existing files (read one existing gen script and image sizes first).
- [ ] **Step 2:** Crop/cut out with `cut.py`; match dimensions of the existing files.
- [ ] **Step 3:** Look at each in the client (hero select, shop); show the user.
- [ ] **Step 4: Commit** "Portraits, icons and card art for the three new heroes."

### Task 10: Homer

**Files:**
- Create: `crates/core/src/heroes/homer.rs`; register `HeroKind::Homer` in `content.rs` and `heroes/mod.rs`
- Create: `data/heroes/homer.toml`, `data/cards/bard.toml`
- Modify: `crates/core/src/heroes/mod.rs` (new hook), `crates/core/src/endgame.rs` (use it), `crates/core/src/api.rs` (`AbilityAction::BardOfTheCenturies`)
- Modify: the buy path in `crates/core/src/shop.rs` only if a generic "on any player's buy" hook is needed for royalties (see Interfaces)
- Modify: `crates/lobby/src/bot.rs`, `client/scripts/hero_texts.gd`, `client/scripts/ability.gd`, `client/scripts/screens/shop.gd`, `HEROES.md`, `CARDS.md`, `CLAUDE.md` (16 heroes)
- Test: `crates/core/tests/rules.rs`

Data: Homer poetry, hp 35, bonus 25, title "The Blind Poet"; params `cost = 3`, `payments = 3`, `bard_chance = 20`, `royalty = 1`, `endgame_poetry = 30`, `endgame_music = 20`. Bard: tier 0, token, cost 1, golden false, shop false, copies 0, `one_battle = true`, effect `CategoryWeight { percent = 25 }` with category music — check that a token with a category does not count toward the tribe bonus; if it would, add a card-level flag rather than special-casing the id.

**Interfaces:**
- `fn endgame_bias(&self, hero: &HeroDef) -> Vec<(Category, u32)>` on the hero trait; default `vec![(hero.category, hero.category_bonus)]`. `endgame.rs` adds each pair into the draw weights instead of reading `category_bonus` directly. Homer returns `[(poetry, 30), (music, 20)]` from params.
- `fn on_any_buy(&self, hero: &HeroDef, state: &mut GameState, buyer: usize, def: CardDefId, env: &mut Env<'_>) {}` — called for every hero in the lobby after a successful buy (shop or extra), like `on_shop_roll`. Homer: if `def` is the Bard and the buyer is not Homer, `bonus_gold += royalty` for Homer's player.
- Homer `on_shop_roll`: for any player's shop, if Homer is alive, has `uses >= payments`, no Bard already in that shop's extras, `rng.percent(bard_chance)` → push Bard.
- `use_ability(BardOfTheCenturies)`: `NoCharges` when `uses >= payments`, then `once_a_round`, then gold; pay, `uses += 1`, `used_turn = turn`.

- [ ] **Step 1: Failing tests:** `homer_needs_three_rounds_of_payment`; `bards_in_every_shop_after_unlock_only_while_homer_alive`; `bard_adds_music_for_one_battle`; `bard_is_not_a_music_card_for_the_tribe_bonus`; `royalty_paid_next_shop_for_others_not_self`; `homer_endgame_bias_is_30_poetry_20_music`; `other_heroes_endgame_bias_unchanged`.
- [ ] **Step 2–4:** FAIL → implement → `cargo test --workspace`, clippy PASS; bot pays the three payments when gold allows and buys a Bard when it is offered and gold is left; client text "Lyrical Peak" (passive) + "Bard of the Centuries" (active) with ability button `BardOfTheCenturies`; docs. Run `cargo run --release -p owl-sim -- --lobbies 5000` and report Homer's line and the archetypes.
- [ ] **Step 5: Commit** "Homer joins the heroes."
