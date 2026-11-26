# Ready button, 60 s shop, three new heroes and three new categories

Date: 2026-11-26. Status: draft for approval.

## Why
- Shop phases were too short for real decisions. They become 60 s, and a Ready button keeps the match from dragging when everyone is done early.
- Three heroes extend the cast past the 12 a full lobby needs and bring three new categories: Music, Poetry and Prose.

## Goals
- Shop phase of 60 s, ended early when every living human is ready.
- Beethoven (Music), Goethe (Poetry) and Miguel de Cervantes (Prose), each in its own hero module, with portrait, face and ability art.
- Music, Poetry and Prose as broad categories, with their Apprentice, Scholar and Master cards and a category icon.
- Placeholder math questions for the new categories, to be replaced later.

## Non-goals
- Real trivia for the new categories.
- Rebalancing existing heroes or cards beyond what the sim run reports.

## Anti-goals
- A ready player must never be hurried by someone else's ready: the shop only closes early when all humans chose to be ready.
- Goethe must not make the hero pick meaningless: his passive touches only his current opponent, only during their battle.

## Approaches considered (ready state)
- **Ready button, close when all living humans are ready (chosen).** Owl lobbies are at most 6 friends and bots are ready at once, so it often pays off.
- No ready, a timer that grows with the turn (Hearthstone Battlegrounds). Simpler, but a game with few humans waits for nothing.
- Ready per player that only skips one's own wait (async, like Super Auto Pets). Does not fit simultaneous 1v1 battles.

## Rules

### Shop and ready
- `shop_seconds` becomes 60.
- During the shop phase a living player can mark themselves ready and take it back.
- Any shop action (buy, sell, place, reroll, freeze, level, ability) by a ready player clears their ready. *(my call)*
- The shop closes as soon as every living human player is ready. Bots and ghosts count as ready. A disconnected human counts as not ready, so the timer still runs out.
- Shakespeare's "not to be" makes him ready for the round, since he can do nothing else.
- Closing early is the same as the timer running out: unpicked golden rewards become reward cards and so on.
- Every player sees who is ready in the player summary (a tick).
- Where the code lives: the ready flag and the "everyone ready" test are in `core`; `lobby` ends the phase when core says so. New command and event, protocol version bump, `docs/protocol.md`.

### Categories
- Music, Poetry and Prose join the broad categories (13 in all). Each gets the [Category] Apprentice, Scholar and Master cards like every other category.
- Until real questions exist, each has 100 generated arithmetic questions (`placeholder = true`, `source = "generated"`). Answers are computed; wrong options are nearby numbers. Difficulty: easy is one small operation, medium a larger product or difference, hard two operations.

### Beethoven
- Title: "Composer of the Ninth". +25% Music. HP 35 (placeholder).
- **Frenzied Notes** (active): pay 3 gold. Must be paid 4 times, in 4 different rounds (the once-a-round rule). After the fourth payment, Notes can appear in his shop.
- **Note**: appears only in Beethoven's shop, as an extra card beside the normal offers (like skulls), 30% chance per shop roll, cost 1 gold (placeholders). Not in the pool, never golden, sells for nothing.
- Placing a Note moves Beethoven to his next phase and the Note disappears; it takes no board slot. The phase holds until the next Note. Order: Syncope, Passionate Composer, Can't Hear the Critics, then back to Syncope. Before the first Note he has no phase.
  - **Syncope**: 25% Music and +2 damage to the opponent in every battle, whatever the result (a tie included).
  - **Passionate Composer**: 40% Music in total from the hero.
  - **Can't Hear the Critics**: 25% Music and 15% reflect (within the 60% cap).
- The +2 is ordinary damage: armor, reflect and escalation apply, and it counts toward the per-battle cap.

### Goethe
- Title: "Poet of Weimar". +25% Poetry. HP 35 (placeholder).
- **Prometheus** (passive): during his battle, his current opponent's battle passives do not work: Herophilus's Open Heart, Pheidippides's Marathon, Marie Curie's Half-Life, Beethoven's phase effects. Not affected: the opponent's +25% category bonus, shop actives and their results (cards, gold), and cards. Applies to a ghost opponent too. It does not carry into the endgame (no ability does), so Half-Life works there.
- **Active**: pay 3 gold, once per round: his Poetry bonus is 40% for the next battle instead of 25%, then back to 25%.

### Miguel de Cervantes
- Title: "Author of Don Quixote". +25% Prose. HP 35 (placeholder).
- **Don Quixote** (active, free): puts one Knight and one Windmill card in his hand. 3 uses per game, once per round.
- **Windmill**: takes a board slot. In the next battle he takes no damage at all (battle, reflected, Arrow, overtime tie); he still deals his own. Removed after that battle.
- **Knight**: takes a board slot. In the next battle he deals +2 damage to the opponent whatever the result (ordinary damage, as Syncope). Removed after that battle.
- If a Knight and a Windmill are ever on his board together, whichever was placed second, the Windmill disappears.
- Knight and Windmill are not pool cards: never golden, sell for nothing, never enter the pool.

### Homer (added 2026-11-26)
- Title: "The Blind Poet". +25% Poetry. HP 35 (placeholder).
- **Lyrical Peak** (passive, endgame only): in the endgame his hero bias is 30% Poetry and 20% Music instead of 25% Poetry. Regular battles keep 25% Poetry. Prometheus never meets it (Prometheus works only in regular battles).
- **Bard of the Centuries** (active): pay 3 gold, 3 times, in 3 different rounds (the once-a-round rule). After the third payment, Bards can appear in every player's shop while Homer is alive.
- **Bard**: an extra card beside the normal offers (like skulls), 20% chance per shop roll, cost 1 gold. Takes a board slot; +25% Music on its holder's weighted questions; lasts one battle, then vanishes. A token: never golden, never in the pool, sells for nothing, not a Music card for the category bonus. Several stack, within the 100% weight cap.
- **Royalties**: each time another player buys a Bard, Homer gets +1 gold at his next shop (as Coin Purse pays). His own Bards pay him nothing. *(timing and "while alive" are my calls)*

### Staying in a lobby when the app goes to the background
Cause, found in `crates/server/src/hub.rs` (`disconnect`): when a connection closes, a player in a match keeps their seat, but a player in a lobby that has not started is removed at once, and if they own it the lobby closes. A phone puts a game in the background and soon stops its network (Android and iOS both do; no game keeps a socket open there without a foreground service), so switching apps for a moment throws the player out of the lobby.

Approaches:
- **Keep the place for a while and reconnect on return (chosen).** What mobile games do: they do not fight the OS, they make a drop harmless.
- Keep the connection alive in the background (Android foreground service with a notification). Not possible on iOS, drains battery, and still fails when the OS kills the app.

Design:
- When a player in a lobby that has not started drops, the server keeps their place for 3 minutes, a fixed time in the server (`LOBBY_GRACE`). The lobby stays open even if they own it. The others see them as away.
- If they reconnect within that time, they are back in the lobby as if nothing happened. If not, they leave as today.
- If the owner starts the match while someone is away, the match starts with them seated; they join it on return (as a running match already does).
- The client reconnects at once when the app returns to the foreground (`NOTIFICATION_APPLICATION_RESUMED` / focus in), instead of waiting for the 3 s retry. A guest is unaffected: it lives as long as its session, not its connection.
- `cargo test -p owl-server` gets tests for: drop and return within the grace, grace running out, owner dropping, start while someone is away.

## Art
Generated with Gemini in the browser (Playwright scripts in `.playwright-mcp/`), in the existing engraved reference-book style: portrait and face for each hero, an ability icon for each, a category icon for Music, Poetry and Prose, card art for Note, Knight and Windmill.

## Done means
- `cargo test -p owl-core`, `cargo clippy --all-targets`, `cargo test -p owl-server` pass.
- `cargo run --release -p owl-sim -- --lobbies 5000` run and placement rates reported, the three new heroes included.
- HEROES.md, CARDS.md, CLAUDE.md (categories, shop time), protocol docs updated.

## Left to the builder
Numbers marked placeholder, the UI layout of the Ready button and tick, and the phase indicator on Beethoven's portrait.
