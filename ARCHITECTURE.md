# Architecture (v1)

Game rules are in `CLAUDE.md`, heroes in `HEROES.md`, cards in `CARDS.md`. This file says how the software is built so that the prototype grows into the real game without rewrites.

## 1. Principles

1. **One rules engine, used by everything.** All rules live in `core`, a pure Rust library. The server, bots, the ghost, the simulator, tests and replays all call the same function. Nothing else knows a rule.
2. **Commands in, events out.** `core` takes a command and returns the list of events that happened. State changes only by applying events. This gives replays, a full audit trail and animation cues from one mechanism.
3. **Deterministic.** `core` never reads the clock and never creates randomness. Time arrives as commands from the server; randomness comes from one seeded RNG per lobby. Seed plus command log reproduces any match exactly.
4. **Authoritative host, thin client.** The client renders state and sends intentions. It holds no rules. Online, the host is the server, and the client never receives a correct answer before the reveal. Offline, the host is the same Rust code running inside the app.
5. **Content is data.** Cards, heroes, tunables and questions are files. Adding a card does not touch the engine unless it needs a new kind of effect.
6. **Seams where the real game will differ.** Question storage, accounts, matchmaking and persistence sit behind traits. The prototype plugs in simple versions; the real game swaps implementations.

## 2. Repository layout

```
crates/
  core/       rules engine (package `owl-core`). No IO beyond loading content, no async, no clock.
  protocol/   (package `owl-protocol`) message types shared by both hosts and tools (serde), protocol version.
  lobby/      (package `owl-lobby`) drives one lobby: phase timers, bots, ghost, per-player filtering. No network, no async runtime.
  server/     (package `owl-server`) online host: Axum + WebSocket, one task per match, accounts and friends in SQLite.
  local/      offline host: Godot extension (GDExtension) wrapping `lobby` for single-player on the device.
  sim/        CLI (package `owl-sim`): thousands of bot lobbies, balance reports.
  release/    (package `owl-release`) describes and signs a published build; `local` uses its checking half. See `docs/server.md`.
client/       Godot 4 project, GDScript.
data/
  cards/      one file per card
  heroes/     one file per hero
  questions/  prototype question bank
  tunables.toml
docs/
```

## 3. The rules engine (`core`)

```rust
pub fn apply(
    state: &mut GameState,
    cmd: Command,
    rng: &mut LobbyRng,
    content: &Content,      // cards, heroes, tunables
) -> Result<Vec<Event>, RuleError>;
```

- `Command`: something a player or the server asks for (`BuyCard`, `PlaceCard`, `SubmitAnswer`, `UseAbility`, `PhaseTimeout`, ...).
- `Event`: something that happened (`CardBought`, `CategoryRevealed`, `AnswerRevealed`, `DamageDealt`, `PlayerEliminated`, ...). Events carry enough detail to animate and to explain themselves.
- `RuleError`: the command was not legal (not enough gold, wrong phase, no charges left). State is unchanged.
- `GameState` is plain data and serializable, so it can be snapshotted, stored and diffed.

Identifiers: `LobbyId`, `PlayerId`, `CardDefId` (the card type, for example `whetstone`), `CardInstanceId` (one physical copy in one lobby), `QuestionId`.

A card instance holds a list of card definitions plus a golden flag, not a single `CardDefId`. A plain card has one definition; a Benz combined card has two. Effects are read from every definition in the list.

## 4. How one game is handled

Each lobby is one actor that owns that lobby's `GameState` and RNG and processes commands one at a time from an inbox. The actor logic lives in the `lobby` crate and does not know where it runs. Online, `server` runs each lobby as an async task. Offline, `local` runs a single lobby inside the app, with the human player and five bots.

```
player sockets ─┐
bots / ghost  ──┼─> lobby inbox ─> core::apply ─> events ─> per-player filter ─> sockets
timers        ──┘                                      └─> match log
```

- No locks and no shared mutable state between lobbies. A crash in one lobby cannot corrupt another.
- Bots and the ghost are command producers attached to the lobby exactly like a player connection. `core` cannot tell them apart from humans.
- Timers belong to the host. `lobby` is told the current time on every step and emits a timeout command when a deadline has passed; `core` then advances the phase. The host supplies the clock (tokio online, the Godot frame loop offline), so `lobby` stays testable with a fake clock.
- Because lobbies are independent, running many processes later is a routing problem (which process hosts which lobby), not a redesign.

### Hidden information
Before events leave the lobby they pass through a per-player filter. Each player gets their own view: their hand, shop and gold are private; the correct answer is stripped from every question until the reveal; the opponent's answer is hidden until both have locked in or the timer expires. A player's board is visible to their opponent only during their battle; outside battles, other players' boards are not sent at all.

### Timing and latency
- Deadlines are sent to clients as server timestamps. The client shows a countdown; the server decides.
- An answer counts if the server receives it before the deadline plus a small grace window (tunable).
- Answer speed never affects results (game rule), so latency only matters at the deadline itself.
- Reveal pauses (category reveal, answer reveal) are fixed server-side durations, so both players stay in step regardless of device speed. Game logic never waits for a client animation.

### Disconnects
A player slot survives a dropped socket. Missed questions count as unanswered. On reconnect the client receives a full snapshot of its view plus current deadlines. Absent for a tunable number of turns: a bot takes the slot.

## 5. How a turn is defined

A lobby is a state machine. Every transition is caused by a command, either from a player or a timeout. When the lobby's state is created, each player is dealt an offer of 2 distinct heroes with the lobby's RNG; offers never overlap, so every hero appears at most once per lobby, and creating a lobby fails if the content has fewer than 2 heroes per player. In `HeroSelect`, each player picks a hero from their own offer; the offer is part of the player's private state (`me.hero_offer` in the `PlayerView`) and is never sent to anyone else. The phase has a deadline, and a player who has not picked gets a random hero from their own offer.

```
WaitingForPlayers -> HeroSelect -> Turn(1) -> Turn(2) -> ... -> Endgame -> Finished

Turn(n):
  Shop            all players at once, shared deadline
  Pairing         instant: pair living players, add a ghost if odd
  Battle          one BattleState per pair, running in parallel
  Resolve         damage, eliminations, end-of-battle effects
  -> Endgame if 2 players are alive and the minimum turn has been reached, else Turn(n+1)
```

### Battle
Each pair has its own `BattleState`, because each pair has its own questions and its own pace.

1. **Plan.** At battle start the server builds the 6-question plan: 2 house slots and 4 weighted slots (2-2, or 3-1 by the split rule), in shuffled order. Each slot rolls its category from its owner's weights, after effects that alter the plan (Übermensch, Record). Each slot also rolls a base difficulty, which the difficulty cards on both boards then move. Then a question of that category and difficulty is drawn that neither player has seen in this match; `QuestionSource::draw` is told the wanted difficulty and answers with the difficulty of the question it found.
2. **Per question:** `CategoryReveal` -> `QuestionOpen` (timer starts) -> both lock in or timeout -> `AnswerReveal`.
3. After question 6 the battle is scored. A pair that finishes early waits for the others.

The turn's `Resolve` runs when every battle is done.

### Endgame
A loop on a single pair: `CategoryReveal` -> `QuestionOpen` -> `AnswerReveal` -> apply the current damage step -> step + 1, until one player is dead. Only HP, armor, category bonuses and the chosen endgame effect are read; everything else is ignored.

## 6. How effects are calculated

Three kinds of effect, all declared in data:

1. **Modifiers.** Passive contributions to a number: category weight, answer time, reflect percent, flat damage. At the start of each battle `core` folds every active source (hero, cards on the board, temporary statuses such as a skull) into a `BattleStats` snapshot per player. Cards in hand contribute nothing.
2. **Triggers.** React to a hook and produce events: Consolation adds a shield to the hand when its holder loses.
3. **Activated abilities.** Used by a command (`UseAbility`), validated for phase, cost and charges: Flaming Sword, Übermensch, To Be or Not To Be, Second Opinion.

Hooks are a closed list, for example: `ShopStart`, `ShopRoll`, `Buy`, `Place`, `BattlePlan`, `QuestionOpen`, `BattleScored`, `DamageCalc`, `DamageTaken`, `BattleEnd`, `Death`, `EndgameStart`.

Within one hook, effects resolve in a fixed order: hero first, then board slots left to right. The order is part of the rules, so results never depend on hash-map iteration or timing.

### Hero abilities
Hero abilities that do not fit the data-driven effect kinds (Herodotus's rewrite, Benz's combine, Shakespeare's skip) are code. Each hero has its own module in `core` implementing one shared trait, with default no-op methods for the hooks it does not use. The engine calls the trait at each hook and knows no hero by name; a registry maps hero ids to implementations. Adding a hero means adding one module, one registry entry and its tests, without touching the engine or another hero. Hero numbers still come from the data files.

### Damage pipeline
Damage has two halves: working out what is dealt, and the character taking it.

**Dealing.** After a battle is scored, `core` builds a `Damage` value for each player who is owed one:

1. **Outcome.** Compare correct counts: win, loss, or tie (flag 0-0 and overtime).
2. **Base damage:** answer margin + base term (tunable) + the winner's tavern level.
3. **Additive bonuses:** Whetstone, Streak Banner, category bonus.
4. **Multipliers:** Double or Nothing, Perfect Mind.
5. **Per-battle cap.**
6. **Tie effects:** tie-breaker, 0-0 card, overtime split. These create damage where the outcome alone gives none.

```rust
pub struct Damage {
    pub amount: u32,
    pub source: DamageSource,     // battle, tie effect, reflect, endgame step, ability
    pub armor_ignored: u32,       // Siege Ram
    pub reflectable: bool,        // false for damage that is itself a reflection
    pub reflect_pierce: u32,      // percent of the receiver's reflect to ignore
}
```

**Taking.** Every point of damage in the game, from any source, goes through one method. Nothing else changes HP or armor.

```rust
// core/src/damage.rs
pub fn take_damage(state: &mut GameState, player: usize, damage: Damage, cap: Option<u32>, ...) -> DamageOutcome;
```

Inside, in this order:

1. Receiver caps (Aegis).
2. Armor absorbs what it can, unless the damage bypasses it; depleted shields break and free their slot.
3. The remainder comes off HP.
4. Internal triggers fire: reflect (if `reflectable`, using the netted percentages, never more than the damage taken), then death and death triggers (Herodotus revive).
5. Reflect is not applied directly. It produces a new `Damage` with `reflectable = false`, and the opponent's own `take_damage` handles it, including their armor. That flag is what prevents loops.

`take_damage` returns a `DamageOutcome` (absorbed by armor, HP lost, reflected, died, revived), and `core` turns it into events. The `DamageDealt` event carries the full breakdown: base, each bonus with its source card, armor absorbed, amount reflected. The client can show exactly why a number is what it is, and tests can assert on each step.

Reflect is calculated on the incoming amount, before the reflecting character's armor.

## 7. How abilities and cards are shown

The client never works out what is allowed or what a card does.

- **State.** The server sends each client a `PlayerView`: own hero, HP, armor, gold, tavern level, hand, board, shop; public information about the others.
- **What is usable.** For every ability and every card in hand, the view includes `usable`, the cost, remaining charges and, if not usable, a reason code. The UI enables or greys out buttons from that, with no rule knowledge.
- **Text.** Card and hero text is generated from the same data files the server uses (a template plus the numbers), so displayed text cannot drift from actual behavior. Golden and combined cards get their text the same way.
- **Feedback.** Every triggered effect produces an event naming its source (`EffectTriggered { source: CardInstanceId, ... }`). The client maps event types to animations and plays them in order.

## 8. Content as data

```toml
# data/cards/whetstone.toml
id = "whetstone"
name = "Whetstone"
tier = 1
kind = "damage"
text = "+1 damage on win"

[[effects]]
effect = "DamageOnWin"
amount = 1          # golden doubles numeric fields by default
```

- A card can carry an optional `[[golden_effects]]` list. If present, the golden version uses it instead of doubling. `golden = false` marks cards that cannot be made golden (shields); `unique = true` marks single-copy special cards.

- `effect` is a closed enum in `core` (`content::Effect`); each variant carries its own numbers and implies when it applies. Unknown effects, fields, categories or card references fail at load time, not mid-match.
- A file with `per_category = true` expands into one card per category.
- Optional `replaces = "<base card id>"` for future collection variants.
- A new effect kind means a new enum variant, its implementation, and a test, in one change.
- `data/tunables.toml` holds every number that is not on a card or hero.
- Content is validated at server start and in CI: every card loads, every referenced id exists, every text template resolves.

## 9. Protocol

JSON over WebSocket for now.

```json
{ "v": 1, "seq": 42, "type": "Events", "payload": [ ... ] }
```

- Client to server: commands, each with a client-generated id so the server can acknowledge or reject it.
- Server to client: `Snapshot` (full `PlayerView`, on join and reconnect), `Events` (ordered, numbered), `Rejected` (command id plus reason).
- `seq` lets the client detect a gap and ask for a fresh snapshot.
- `v` is the protocol version. The server refuses clients it cannot serve.
- Joining a lobby always presents a session token, even in the prototype (a guest token), so real accounts slot in without changing the handshake.

## 10. Storage seams

| Concern | Trait | Prototype | Real game |
|---|---|---|---|
| Questions | `QuestionSource` | files in `data/questions/` | PostgreSQL online, with answer statistics and per-player history; bundled pack offline |
| Match log | `MatchLog` | file per match (seed + commands + content hash + `core` version) | PostgreSQL or object storage |
| Identity | `server::store::Store` | accounts, sessions and friends in one SQLite file (see `docs/server.md`) | the same, or an auth provider |
| Matchmaking | `Matchmaker` | one in-memory queue | separate service, Redis |
| Progress, collections | `ProfileStore` | not implemented | PostgreSQL |

## 11. Client (Godot)

- `Transport` (autoload): one interface with two implementations. `RemoteTransport` talks to the server over WebSocket. `LocalTransport` talks to the `local` extension in the same process. Both carry the same protocol messages, so every scene and script above this layer is identical online and offline.
- `Store` (autoload): the latest `PlayerView`, updated by snapshots and events.
- `Director`: a queue that plays events as animations in order. State in `Store` is already final; the director only decides what the player sees and when.
- Scenes: lobby with hero select, shop, battle, endgame, results. One reusable card scene that renders from card data.
- The client still contains no rules. Rust is present on the device only as the offline host, behind `Transport`.
- First playable target is a desktop build; phone export follows once the loop works.

### Offline single-player
- One human and five bots, run by `local` using the same `lobby` and `core` as the server.
- The app ships with a bundled question pack and the content files. Correct answers are therefore on the device, which is acceptable only because no other human is in the match.
- Offline results must not feed leaderboards, ranked progress or anything of value, since they cannot be trusted. Decide later whether they grant limited progress.
- `local` must be compiled for each target (desktop, each Android ABI, iOS). This is the riskiest build step in the project, so it gets an early spike: a trivial extension exported to a real Android device and a real iPhone before any gameplay depends on it.

## 12. Bots, ghost, simulator

- A bot is a policy with two parts: a knowledge profile (chance to answer correctly per category) and a buying strategy (specialist, tank, scorer, generalist).
- The ghost is a bot built from a snapshot of another player's board and their accuracy so far. Damage dealt to it is discarded.
- `sim` runs full lobbies of bots through `core` with no server and no clock, and reports placement by hero, archetype and knowledge level. Every balance change is checked there.

## 13. Testing

- Unit tests per rule and per damage-pipeline stage.
- Scenario tests: a seed and a command list, asserting on the resulting events. Bug reports become scenario tests by attaching the match log.
- Property tests: HP and armor never go negative, reflect never exceeds damage taken, the shared pool never loses or duplicates a card instance, a lobby always terminates.
- Protocol tests: a scripted client plays a full match against a real server.

## 14. Prototype versus real game

Built properly from the start, because changing them later is expensive: `core` and its command/event model, determinism, the runtime-independent `lobby` crate, the client `Transport` interface, per-player filtering, the protocol envelope and versioning, content as data, the storage traits.

Deliberately simple in the prototype: one server process, file-based questions, guest identity, no persistence beyond match logs, JSON on the wire, placeholder art.
