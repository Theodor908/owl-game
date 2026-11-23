# Owl

Mobile game (Android, iOS): an auto-battler built on general-knowledge questions. 6 players per lobby, shop phases followed by 1v1 battles fought on questions. Rules v1. Numbers in the tunables table are placeholders to be tuned; everything else is decided unless listed under Open. The prototype and all content are in English.

## Game rules

### Decided
- 2 to 6 players per lobby. Battles are 1v1, pairings rotate so everyone meets everyone. With an odd number of players alive, one player fights a ghost.
- No minions fighting. Players buy cards from a shop and place them on their board; cards modify questions and battle outcomes (category odds, extra answer time, damage, defense).
- Each hero has one or two passives. When a lobby starts, each player is dealt a private offer of 2 distinct heroes, drawn at random from the whole list; no hero appears in two players' offers, so each hero appears at most once per lobby. A player may pick only a hero from their own offer, and cannot see anyone else's offer. A player who has not picked when the hero-select timer runs out gets a random one of their two. The hero list must hold at least 2 heroes per player in the lobby (12 for 6 players).
- A player can see an opponent's board only during their battle.
- Everyone can see a short summary of each player: tavern level, the category they hold the most cards of on the board and how many (no category is named when two or more share the lead), golden cards on the board, current win streak, and the score of their last two battles. The summary never shows the cards themselves.
- A player who has been eliminated may spectate: they see each living player's board, shop offers, gold, level, hand size and current battle (question, answers on offer and score, never the correct answer), delayed by a set time. Spectating ends when the match ends. A living player never sees any of this.
- Every card that can be golden states in its data what the golden version does (`golden_text`), so a player can preview it.
- All damage, reflected damage included, is applied through the character itself, which checks armor, armor bypass and HP and fires its own triggers.
- Questions are multiple choice with 4 options. No typing.
- A battle has 6 questions, the same for both players, answered simultaneously:
  - 2 from the house, fully random category.
  - 4 from the players' category weights, split 2-2 or 3-1 depending on weights. Never 4-0.
- Each question: reveal category, then the question, then the timer starts. Same sequence in every format.
- 9 seconds to answer. Cards or heroes can add 3 seconds, hard cap 12.
- A question ends the moment both players lock in.
- Battle damage on a win: (own correct answers - opponent's correct answers) + base 2 + own tavern level, plus card bonuses. Damage grows over the match through tavern level; a bigger margin deals more.
- Armor is bought in the shop and also comes from cards and passives.
- A reflect build is available to everyone through cards; heroes can make it better. Reflect sends a percentage of the damage its holder takes back at the opponent. A player's total reflect, hero and cards together, is capped at 80%. Reflect carries no damage penalty for its holder.
- A tie on correct answers is a tie: no damage, whatever the score (0-0 through 6-6). Answer speed never decides anything (latency makes it unfair).
- Tie-breaker card: on a tie where both players got at least one question right (including all correct), the holder deals damage equal to half their tavern level, minimum 1. If both players hold one, nothing happens.
- 0-0 card: a separate card that deals damage on a 0-0 tie, capped at 1.
- Reflect only triggers when its holder is about to take damage. It does nothing on ties.
- Overtime: the lobby has a turn counter (as in TFT). After a set turn, a tie no longer deals zero: the higher damage of the two players is split in half and both take it. Tie card effects still apply on top.
- Starting HP depends on the hero.
- Armor takes a card slot on the board. A shield only counts once it is placed.
- Players have a hand of up to 10 cards as well as a board. Cards bought from the house go to the hand, and the player places them on the board when they choose. Cards only take effect once placed.
- Merging (3 identical cards into a golden one, Darwin's 2-card golden, Benz's combine) counts cards in hand as well as on the board. The player is never forced to place a card.
- A golden card goes to the player's hand when it is made, wherever its copies were, and the player places it when they choose. If the hand has no room, it takes the board slot one of its copies left.
- A player with a full hand may still buy a card if buying it makes a golden card at once.
- Categories act as tribes, with a bonus at 3 cards of the same category.
- 3-1 split: a player gets 3 of the 4 weighted questions when their total weight investment beats the opponent's by a set threshold; otherwise 2-2.
- Shop phase lasts 25-30 s. Board has 7 slots.
- Three identical cards merge into a golden one. Shields cannot be made golden.
- There are 6 tavern tiers. Every card has a tier number; a card can appear in a player's shop only if its tier is at or below that player's tavern level.
- A player can freeze their shop, for free, to keep its cards for the next turn.
- The card pool is shared. Sold cards return to it, and so do all cards of an eliminated player. A golden or combined card returns every copy it was made from.
- A golden card doubles its numbers by default. A card can declare its own golden effect in its data instead (example: golden Aegis keeps its damage cap and adds 10 armor).
- Placing a golden card on the board gives a reward: the player picks 1 of 3 cards from the next tavern tier up, added to the hand for free. The tier is counted from the player's tavern level at the moment the pick opens. A player at tavern level 6 is offered tier 6 cards.
- The reward is offered once per golden card, the first time it is placed. Making the card gives nothing, and placing it again after taking it back gives nothing. A golden card that went straight onto the board because the hand had no room counts as placed: its reward is offered at once.
- A golden card made by any route gives the reward: three copies, Darwin's 2-card golden, and a golden card that Benz has combined with another card (when the combined card is on the board). The golden healer's heal and the golden hunter's Arrow still come when the card is made; only the pick waits for placement.
- A reward still unpicked when the shop closes becomes a reward card in the player's hand, and its three options go back to the pool. The reward card does not place itself. The player places it later, like any card; that opens a new pick of 1 of 3 and the reward card disappears. It never stays on the board and needs no free slot. A reward card left unopened, or opened and not picked, stays a reward card.
- A reward card is not a real card: it sells for nothing, it cannot be combined, and it never enters the pool, whether sold or left in the hand of an eliminated player.
- The hand always has room for the reward card, because the golden card left it. The exception is a hand that is full when the shop closes: then there is no room for any card and the unpicked reward is lost.
- A golden category card counts as 3 cards of its category toward the category bonus, and raises that bonus from +2 to +3 damage per correct question. The bonus does not stack: extra category cards beyond the 3 needed add no further damage.
- Healing exists in three forms, and like damage it is applied through the character:
  - Healing never raises a player above their starting HP.
  - Healer: a board card that heals its owner a set amount every turn. A player can have only one Healer on the board.
  - Golden healer: made when the player has a Healer on the board that was not disabled last round and 2 more Healers in hand. When made, it instantly heals its owner for a quarter of their starting HP and every other player in the lobby for an eighth of theirs. Its per-turn healing stays the same as a non-golden healer's, not doubled. Only one golden healer can be made per match, across all players.
  - Healer and Hunter are tier 6 cards.
  - Potion: a legendary one-use card, bought in the shop, that instantly heals a set amount. It has no tier: it can appear in any player's shop at any tavern level, with a small chance each round that grows as the match goes on. It shows as an extra card beside the normal shop offers, like Shakespeare's skulls, and never replaces one. Once used it is removed from the game and never returns to the pool; potions are the only cards that do this.
- Hunter: a card that attacks healers. If the opponent has a Healer, the Hunter disables it for its next heal; it does not destroy it. The Hunter stays on the board permanently, so against an opponent with no Healer it is a wasted slot. A player can have only one Hunter on the board. A golden Hunter, made from copies in hand, does the same and also gives a one-time Arrow card: it deals a set damage to the holder's next opponent whatever the battle result, then vanishes. Arrow damage is ordinary damage: it goes through armor and HP and can be reflected.
- Shakespeare's skulls disable a random card, but a Healer on the opponent's board is more likely to be picked than any other card. Skulls only disable; nothing destroys a Healer.
- Special cards (Second Opinion, The Oracle, the eight difficulty cards) have a single copy in the pool, so they can never be golden. The three endgame effect cards are special too and have no golden version.
- Every question has a difficulty: easy, medium or hard. Each question of a regular battle first gets a base difficulty, drawn at random from the odds in the tunables. Players see the difficulty with the category.
- Difficulty cards move that difficulty. "Lower" is one step easier (hard to medium, medium to easy) and "raise" one step harder. "Greatly lower" and "greatly raise" count as two steps, so alone they make a question always easy or always hard.
- A difficulty card touches the questions of its holder's battle, for both players, house questions included. An "all categories" card touches every question. A "top category" card touches only questions of the holder's top category: the category with the highest weight percentage, hero and cards together, at the start of the battle. Several categories tied for the highest are all top. A holder with no weights has no top category, and the card does nothing.
- The steps from both players' boards add up and cancel each other first; only the result is capped at easy and hard. A raise and a lower give no change; two raises give two steps.
- Difficulty cards work in ghost battles from the ghost's board. They do nothing in the endgame, where a question keeps its base difficulty.
- The question bank gives a question of the wanted category and difficulty that neither player has seen. If none is left it takes the nearest difficulty (for medium: easy, then hard; for easy: medium, then hard; for hard: medium, then easy), and after that a placeholder sum.
- Equal base gold income for everyone; bonus gold only from specific cards or heroes. Unspent gold does not carry over to the next turn. Cleopatra is the exception (see `HEROES.md`).
- Herodotus's ability, both the rewrite and the revive, needs the highest tavern level.
- A hero ability with a limited number of uses per match can be used at most once per round.
- Streak Banner counts the battles won in a row before the current one.
- Reflect never exceeds the damage taken.
- When two reflect players meet, the lower percentage is subtracted from both. Only the player with a percentage left has reflect active, at that remaining value (50% vs 25%: the first reflects 25%, the second nothing; equal values cancel). The rest of the damage goes through as normal.
- Instant-answer legendary: works on one question per regular battle, and on one endgame question in total.

### Card pool and collections
- Every lobby starts from a base pool available to everyone.
- Cards from players' collections can alter that pool: a collection card is a variant of a specific base card and can be randomly chosen to replace it for everyone in the lobby (example: base card gives +1 damage, a collection variant gives +3 and another effect).
- A collection never gives its owner private power. It only changes what the whole lobby can buy.
- Balance principle: defensive builds raise the floor, knowledge raises the ceiling. Knowing more must be rewarded.

### Endgame (last two players)
- Different format: one question at a time, 10 seconds. The category is random but biased by both players' hero category bonuses, added into one draw: with a +25% History hero against a +25% Physics hero, each question has a 25% chance to be History, 25% to be Physics, and the remaining 50% is drawn uniformly from all categories.
- Damage starts at 1 and goes up one step per question, with no cap.
- Heroes and regular cards stop working, except hero passives explicitly marked as carrying into the endgame.
- HP and armor carry over.
- Special endgame effects carry over too: expensive, high-tier shop cards (example: choose between 3 options instead of 4).
- Both correct: no damage, but the step still increases. Both wrong: both take the current damage. If both die on the same question, higher remaining HP wins. With equal HP both players win; the order on the podium is random.

- One endgame effect per player. A player holding several chooses one before the endgame starts.

### Heroes
The 12 starting heroes, their abilities and their tunable numbers are in `HEROES.md`: the 12 a 6-player lobby needs.

### Tunables (placeholders, not decisions)
Every value here is a starting guess so `core` and `sim` can run. They live in one config file (`data/tunables.toml`), never as constants in code. Change them from `sim` results.

| Name | Placeholder |
|---|---|
| Max tavern level | 6 |
| Gold per turn | 3 on turn 1, +1 per turn, cap 10 |
| Card cost / sell value / reroll | 3 / 1 / 1 gold |
| Tavern upgrade cost (to level 2..6) | 5, 7, 8, 9, 10, each 1 cheaper per turn not bought |
| Shop size by tavern level | 3, 4, 4, 5, 5, 6 |
| Copies of each card in the shared pool, by tier | 12, 11, 10, 8, 6, 4 |
| Base term in the battle damage formula | 2 |
| Max damage from one regular battle | 15 |
| Overtime starts | turn 12 |
| Earliest turn the endgame can start | 5 |
| 3-1 split threshold | weight total leads the opponent's by 40 points |
| Endgame answer time | 10 s |
| Healer healing per turn | 1 HP |
| Potion healing / cost | 5 HP / 3 gold |
| Potion chance to appear in a shop, per round | 2% on turn 1, +1% per turn |
| Potions in the pool | 4 |
| Arrow damage | 3 |
| Spectating delay | 5 s |
| Herodotus rewrite shield | 3 armor |
| Heroes offered to each player | 2 |
| Skull: how much likelier a Healer is to be picked than another card | 3 times |
| Base difficulty of a question: easy / medium / hard | 40% / 40% / 20% |

### Category weights
- A weight is a percentage chance that one of the player's weighted questions comes from a given category. Example: 25% history means each of that player's weighted questions has a 25% chance to be history; the remainder is drawn uniformly from all categories.
- A player's percentages add up across hero and cards, capped at 100% in total.
- "Total weight investment" for the 3-1 split is that sum.
- Prototype categories (10): History, Flora and Fauna, Cars and Mechanics, Film, Physics, Philosophy, Theater, Anatomy, Geography, Sport.

The card pool is in `CARDS.md`. Hero numbers are in `HEROES.md`.

### Further rules
- Hero +25% bonuses apply to the player's own weighted questions, not to the 2 house questions.
- Categories form a two-level taxonomy (broad category, subcategory). Hero bonuses and category cards refer to broad categories.
- Ghost: a snapshot of a random other player's board, with answers simulated from that player's accuracy so far in the match. Damage dealt to the ghost is ignored.
- The endgame starts when two players remain and at least 5 turns have been played (tunable). Until then, two remaining players keep playing regular turns. This is also how a lobby that starts with exactly 2 players works.
- Reflect is calculated on the incoming damage, before the reflecting character's armor. It triggers on any damage its holder takes, overtime tie damage included, but never on damage that is itself a reflection.
- The 0-0 card cancels out when both players hold it, like the tie-breaker.
- Skulls are placed on Shakespeare's own board and take a slot each; he can fill the board with them. In a battle each skull disables a different random card on the opponent's board. Placed skulls disappear after one round. He collects them over several rounds, from his shop and from "to be".
- While Shakespeare is in the lobby, skulls can appear in every player's shop, at the same chance and cost, and work the same way for whoever places them. Only Shakespeare has "to be". Skulls cannot disable other skulls.
- Newton's Flaming Laser Sword and the Second Opinion card can be used on the same question, which leaves only the correct answer. This is intended.
- Collection variants (after the prototype): each one declares the base card it replaces; a lobby has a cap on replacements, split evenly across players.

### Open
Numbers, all in the tunables table: the overtime turn, the 3-1 split threshold, the minimum turn for the endgame, hero HP and ability costs. Tune them from `sim` results.

Rules not yet decided (ask before implementing any of them):
- Target match length. Intended to be close to Hearthstone Battlegrounds; check with `sim`.

Agreed directions, not yet designed (each needs rules written before it is built):
- Question difficulty as a stat that damage can key off. Difficulty itself is decided: chance sets it and the difficulty cards move it (see Decided). Open: damage or other effects that depend on it, and the round number as a third thing that sets it.
- A player decision at category reveal (stake or double-down before the question shows).
- A mechanical identity per category.
- Shop-phase engines: buy, sell and reroll triggers, scaling.
- In-battle streaks, subcategory cards, scouting.
- Team mode (a future mode, not started). Decided so far: eight players in four teams of two; a team has one shared health pool, the starting health of its two heroes added together; teammates may pass each other cards, but only cards that are not golden, and nothing else (no gold). Not decided: how the two players' answers to a question combine, and where four more heroes come from (eight players need sixteen for the two-per-player offer). Nothing of it is built.
### Out of scope for the prototype
Monetization, languages, target audience, packs, collection variants (base pool only), leaderboards, matchmaking with strangers. The prototype goal is a working lobby: shop, battles, endgame.

### Accounts and playing online
- An account is a username and a password, nothing else. A username is unique whatever its case.
- A player finds another by their exact username and asks to be friends; the other has to accept.
- A player may also play as a guest, without an account: the server makes up a name, there is no password, and the guest is erased when its session ends or it leaves. A guest can do what an account can, except change its name or have a password.
- A player opens a lobby and invites friends who are online, up to 6 seats in all. A lobby is joined by its code: an invitation carries it, and the owner can also tell it to anyone, which is how a guest gets in. The owner may put bots in empty seats and starts the match; a match needs at least two seats taken.
- The game updates itself on start, before the menus open: it downloads a newer pack and native library from the server, checks their signatures and starts again; a build that needs a new APK shows only a Download button. With no answer from the server it opens as it is, so offline play always works. Development runs skip this. `release.bat` publishes a build; see `docs/server.md`.
- The server keeps as little as it needs and lets a player export or erase everything it holds about them. See `docs/gdpr.md` and `docs/privacy.md`.

## Architecture
See `ARCHITECTURE.md`. In short: all rules live in `core` (pure, deterministic Rust, commands in and events out); a runtime-independent `lobby` crate drives each match; the server hosts lobbies online and a Godot extension hosts a single lobby offline for single-player; the Godot client renders state and holds no rules; cards, heroes and numbers are data files.

## Working rules for Claude Code

- Every game rule lives in `core` and has a test. Never put rules in `server` or the client.
- After any change in `core`: `cargo test -p owl-core` and `cargo clippy --all-targets`. (The packages are named `owl-core` and `owl-sim`, because a crate called `core` would clash with Rust's own.)
- After any balance change (cards, damage, armor): run `cargo run --release -p owl-sim -- --lobbies 5000` and report placement rates per archetype (high knowledge, low knowledge with a defensive build, reflect) before calling the change done.
- Protocol changes bump the version (`crates/protocol`) and are documented in `docs/protocol.md`.
- The server holds no game rules either: it forwards commands to `lobby` and sends each player the view `lobby` gives them. After any change in `server` or `protocol`: `cargo test -p owl-server`.
- Passwords, usernames and tokens are handled only in `crates/server/src/crypto.rs` and `store.rs`. Never log them, never store them another way, and never send a password or a token over anything but HTTPS/WSS. A new piece of personal data needs a line in `docs/privacy.md` and in the export, and must be removed by account deletion.
- A downloaded pack or library is used only if it is signed with the release key. The private key lives outside the project (`%USERPROFILE%\.owl\release.key`) and is never committed, logged or copied to the server. The check lives in `client/scripts/boot.gd` and `crates/release`; a change to either, or to `client/project.godot`, needs a full APK release.
- Code is modular and follows SOLID and current Rust best practice (Rust API Guidelines, clippy clean). Each hero's ability lives in its own module behind a shared trait; adding a hero must not require editing another hero. Check current documentation for Rust, godot-rust and Godot rather than relying on memory.
- Do not invent game rules. If a rule is missing or two rules conflict, ask. Numbers in the tunables table may be changed from `sim` results without asking, as long as the change is reported.
- Do not generate trivia questions without a verifiable source. Every question has a source and passes human review.

## Animation and graphics

What Claude Code can do alone, since all of it is text in Godot:
- `.tscn` scenes and `.tres` resources
- animations with `AnimationPlayer` and `Tween` (hover, select, buy, hit, answer reveal)
- particles (`GPUParticles2D`) and `.gdshader` shaders (glow, foil for legendaries, dissolve)
- phone UI layout, transitions between phases

What comes from outside (a person or other tools): card illustrations, heroes, icons, fonts, sound and music. They go in `client/assets/` under names agreed in advance; until then use placeholders of the same dimensions.

Workflow:
1. Each animation is a separate scene that runs on its own with test data (`client/scenes/preview/`).
2. Parameters (durations, curves, colors) are `@export`, so a person can tune them in the editor.
3. Visual check: run the preview scene and capture PNG frames (Godot Movie Maker, `--write-movie`); Claude Code looks at the frames before calling an animation done. The final call on "looks good" belongs to a person.
4. The client reacts to server events (`CardBought`, `CategoryRevealed`, `QuestionShown`, `AnswerRevealed`, `DamageDealt`); each event has an animation, and game logic never waits on animations.

## Build order

1. `core` + `sim`, no graphics: the full game loop with bots. Done.
2. `lobby`: timers, bots, ghost and per-player views, tested with a fake clock. Done.
3. Spike: a trivial Rust extension exported from Godot to a real Android device and a real iPhone. Do this early; it is the riskiest build step.
4. `local` + `LocalTransport` and the Godot client with placeholders: offline single-player against bots, complete game, ugly. Developed and played in a desktop window sized like a phone; mouse clicks stand in for taps.
5. Animations, effects, sound.
6. `server` + `protocol` + the client's online layer: a real lobby over WebSocket, same client. Done on the development machine; not yet run on a Raspberry Pi or across the internet.
7. Phone builds and device testing (iPhone builds need a Mac with Xcode).
8. Accounts and friends: done with step 6. Matchmaking with strangers, ratings and leaderboards are not planned: online play stays in lobbies with friends, joined by code.
