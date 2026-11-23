# Owl goes landscape: a buildable interface plan

Owl's interface can borrow the genre's layout almost whole for the shop and must be designed from scratch for the battle. The shop screen follows the fixed, non-scrolling pattern that Hearthstone Battlegrounds uses on phones (player list on one edge, shop row over board row, costs printed on the buttons), with Owl's hand kept permanently visible because 7 board slots, 10 hand cards and 8 shop offers all fit at legal touch sizes in a 1920x1080 frame. The question screen has no precedent: no phone trivia game was found documented in landscape, so this document specifies an original layout, two answer buttons under each thumb with the question and both boards in the centre, and gives the reasoning and the rejected alternatives. Every screen is specified against the per-player view the client already receives; 20 pieces of data the view lacks are listed in one table with what the interface does until each exists. The evidence for the reference games is thin in places (six phone frames from one video for Battlegrounds, launch-era text for Teamfight Tactics Mobile) and every claim in section 1 carries its evidence level. The visual direction is an engraved reference-book look in ink, paper, gilt, verdigris and vermilion, chosen to fit scholars and to stay clear of both reference games' identities. The existing portrait screens are to be replaced, not adapted.

## How this was researched, and its limits

Research date: 17 November 2026. The findings come from five sets of notes (Battlegrounds on phones, Teamfight Tactics Mobile and other auto-battlers, quiz question screens, landscape ergonomics and Godot, and an audit of the current client) plus the project's own `CLAUDE.md`, `ARCHITECTURE.md`, `CARDS.md`, `HEROES.md`, `crates/core/src/api.rs`, `crates/lobby/src/view.rs`, `crates/lobby/src/lib.rs` and `data/tunables.toml`.

Read these limits before relying on section 1:

- **Battlegrounds phone layout** rests on six still frames from one video published 27 May 2026 (roughly patch 35.x, inferred from the date) plus a wiki page that describes roughly patch 29.2. Hero select, the end-of-match screen, the triple animation and every gesture detail were not captured and are marked unverified.
- **Teamfight Tactics Mobile** sources are mostly from the March 2020 launch. No gameplay video was watched. Current HUD positions are unverified, and two sources disagree about where the shop panel opens.
- **Quiz apps.** No gameplay video or screenshot was inspected. No phone trivia game was found documented as landscape. The landscape question layout in section 2.3 is therefore original design reasoning built on one thumb-reach study and published touch-target numbers. It has not been tested on a device.
- **Godot specifics** (stretch settings, the safe-area conversion) come from documentation and engine source and are untested on a device.
- Evidence tags used below: **[frame]** a still frame was looked at; **[text]** a fetched page; **[snippet only]** a search-result summary, page not opened; **[recollection]** an unsourced memory recorded in the notes, to be treated as a hypothesis; **[derived]** arithmetic on sourced numbers; **unverified** could not be confirmed.
- Nothing about the reference games has been added from memory beyond what the notes contain, except two remarks in section 4 labelled "unverified, from general knowledge".

Reference-game interfaces change between patches. Unless a row says otherwise, Battlegrounds rows describe the May 2026 video (about patch 35.x) and TFT rows describe the March 2020 launch build.

## 1. Patterns found, with sources

### 1.1 Hearthstone Battlegrounds on a phone

Video for all [frame] rows: "Playing BG on Mobile is SO Weird!", Rdu Hearthstone, 27 May 2026, [youtube.com/watch?v=oEKyHSBcmzk](https://www.youtube.com/watch?v=oEKyHSBcmzk). Time codes are given per row. Positions are estimates from blurred camera footage of a real phone.

| # | Pattern | Evidence | Source | What Owl takes |
|---|---|---|---|---|
| B1 | The recruit screen is one fixed landscape screen with no scrolling. Player list is a narrow column on the left (about 6% of width), shop controls top centre, shop row directly above board row in the middle, timer plate on the right edge, hero bottom centre, a small collapsed hand and the gold counter bottom right. | [frame] 2:30, cross-checked at 0:45 | [video 2:30](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=150s) | One fixed screen; shop over board; list on one edge. Owl keeps the hand open (it fits) and moves the controls to the right edge for thumb reach. |
| B2 | Upgrade, refresh and freeze are single-tap buttons and each prints its current cost on the button. The upgrade discount shows only as a smaller number. | [frame] 2:30 ("6"), 0:45 ("11") | [video 0:45](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=45s) | Cost on the button, no tooltip. |
| B3 | Items with a variable price carry a cost coin on the card; fixed-price minions show none. | [frame] 2:30 | same | Owl prints a cost coin on every offer, because Owl prices vary (2, 3, 8). |
| B4 | Buying is a drag from the shop row to the hero; selling is a drag from the board to the shopkeeper. The two drop targets are at opposite ends of the screen. | [text] for buy (2022, input method not named); [snippet only] for sell | [Pocket Tactics](https://www.pockettactics.com/hearthstone/battlegrounds-mobile), [Icy Veins](https://www.icy-veins.com/hearthstone/hearthstone-battlegrounds-mechanics-guide) | Drag with opposed drop targets. Whether phone buying is drag-only is **unverified**. |
| B5 | The phone hand is a small fan in the corner with a separate enlarged state. | [frame] 2:30; [snippet only] patch 16.2 fix | [Fandom patch list](https://hearthstone.fandom.com/wiki/Battlegrounds/Patch_changes) | Not taken. How the fan opens and closes is **unverified** [recollection: tap]. |
| B6 | Three levels of card detail: a token on the rows (portrait, two numbers, tier badge, no name, no text), a miniature in the hand, a full card only when enlarged. | [frame] 2:30, 1:40 | [video 1:40](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=100s) | Three card sizes; no rules text on small cards. |
| B7 | With 7 minions and large numbers, neighbouring stat badges touch and overlap. | [frame] 2:30 | same | A warning: Owl puts one value per small card and never lets badges cross the card edge. |
| B8 | Colour carries state: outline on things the player can act on, buffed numbers in a second colour, a differently coloured frame for golden. | [frame]; the unaffordable state was not seen | same | The idea (state on the object), not the colours. Owl adds a shape or icon to every state. |
| B9 | Pick 1 of 3: a banner, three large cards (each about 15% of width and 50% of height) and a "Hide" button bottom left that lets the player look at the board underneath. Player list, hero and hand stay visible. | [frame] 1:40. The frame came from a shop spell, not confirmed to be a triple reward. | [video 1:40](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=100s) | The reward overlay with a Hide control. Whether an unpicked choice auto-resolves is **unverified**. |
| B10 | Three copies merge into a golden card, counting hand and board. The reward arrives as a card the player plays when they choose. A triple by any player is announced in the sidebar. | [text], wiki describing about patch 29.2 | [Hearthstone wiki](https://hearthstone.wiki.gg/wiki/Battlegrounds) | Reward that does not interrupt a drag. The merge animation is **unverified** [recollection: copies fly together]. |
| B11 | Player list: portraits sorted by health, top four numbered, the next opponent marked by a coloured border and a sideways offset. Health was not readable on collapsed rows at phone size. | [text] about patch 29.2; [frame] 2:30, 0:45 | [Hearthstone wiki](https://hearthstone.wiki.gg/wiki/Battlegrounds) | Sort by health; mark the next opponent by offset as well as colour. Owl has 6 rows, not 8, so it can print HP on the row. |
| B12 | Touching a row opens a detail panel beside the column and dims the rest: name, hero, health, tier, tribe count, combat history, hero power card. | [frame] 0:20; [text] about patch 29.2 | [video 0:20](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=20s) | Detail panel beside the row. Tap versus press-and-hold is **unverified**. A 2023 report says the panel overflowed the right edge on some phones [snippet only], [Blizzard forum](https://us.forums.blizzard.com/en/hearthstone/t/104774/1). |
| B13 | The timer escalates in three steps: a quiet number, a red number, then a burning rope across the middle of the play area between shop and board. | [frame] 2:30 ("88s"), 0:45 ("19s" with rope) | [video 0:45](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=45s) | Three-step escalation and a bar placed where the player is looking. The thresholds are **unverified**. The rope itself is Hearthstone's look; see 1.6. |
| B14 | In combat the opponent's hero replaces the shopkeeper, the opponent's row replaces the shop row, the timer plate reads "Combat", and the player list stays. | [frame] 1:10 | [video 1:10](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=70s) | Opponent appears where the shop was; persistent elements do not move. The transition between phases is **unverified**. |
| B15 | The recruit timer starts when combat starts, so a slow combat shortens the shop. Players complain. | [text] about patch 29.2; [snippet only] forums | [Hearthstone wiki](https://hearthstone.wiki.gg/wiki/Battlegrounds), [Blizzard forum](https://us.forums.blizzard.com/en/hearthstone/t/fix-the-timer-to-start-when-you-enter-the-tavern/83164) | A warning. Owl's current client has the same fault in small (section 2.4). |
| B16 | Hero select and end of match on a phone. | **Not captured.** A desktop thumbnail at 4:10 shows four portraits in one row with a number badge and name plate each. | [video 4:10](https://www.youtube.com/watch?v=oEKyHSBcmzk&t=250s) | Nothing verified. Owl's hero select (8 heroes, all visible, first pick locks) differs anyway. |
| B17 | No talk, developer post or article on Battlegrounds' mobile UI design was found; mobile fixes recur in patch notes. | absence of sources | [Blizzard 35.6 notes](https://playhearthstone.com/en-us/blog/24276665/) [snippet only] | The phone UI reads as an adaptation of the desktop screen. That is an inference from thin evidence. |

### 1.2 Teamfight Tactics Mobile

No video was watched. The only images inspected were five store screenshots with the HUD removed.

| # | Pattern | Evidence | Source | What Owl takes |
|---|---|---|---|---|
| T1 | The shop is a panel opened by a button; it opens by itself at the start of each round, and while it is open the stage tracker shrinks to just the time left. | [text], Riot's 2020 pre-launch post read through a publisher mirror | [VNG mirror of Riot post](https://dtcl.vnggames.com/en-us/news/riot-games/teamfight-tactics-mobile-update) | When a panel takes space, shrink a persistent element to its one time-critical number. **Source conflict:** Riot (2020) says the shop opens near the top; a July 2026 third-party article says it slides up from the bottom ([turboboost.gg](https://turboboost.gg/article/tft-mobile-vs-pc-differences-and-crossplay)). Neither was checked against a current capture. |
| T2 | Tap buys; drag moves and sells; tap inspects. Selling uses large drop zones. | buy and inspect [snippet only]; sell zones [snippet only] plus a 2020 tester | [Mobalytics](https://mobalytics.gg/?p=17475), [devtrackers.gg thread](https://devtrackers.gg/teamfight-tactics/p/15e47ea8-tft-mobile-early-access-feedback) | Tap for cheap or informational actions, drag for spatial ones. |
| T3 | Selling was hard to discover at launch; a tester asked for other UI to fade more during a drag. | [text], March 2020 | [Upcomer](https://upcomer.com/teamfight-tactics-mobile-first-impressions/) | The sell zone appears and labels itself the moment a drag starts; everything that is not a target dims. |
| T4 | Touch-and-hold thresholds caused friction; a later hold-to-confirm got a small fill bar. | [text] 2020; [text] patch 12.4 (2022) | [TFT 12.4 notes](https://teamfighttactics.leagueoflegends.com/en-au/news/game-updates/teamfight-tactics-patch-12-4-notes) | No long-press for core actions. Where a hold is used, it shows a fill. |
| T5 | Riot swapped two shop buttons once and reported the first game after it as "quite painful"; related controls are placed together on purpose. | [text], 10 December 2019 (PC) | [TFT Interface Update](https://teamfighttactics.leagueoflegends.com/en-gb/news/game-updates/tft-interface-update/) | Button positions never move between states or screens. |
| T6 | Players not yet fought this stage get an icon by their avatar. | [text] patch 12.4; scope today not checked | [TFT 12.4 notes](https://teamfighttactics.leagueoflegends.com/en-au/news/game-updates/teamfight-tactics-patch-12-4-notes) | A next-opponent marker. Owl's pairing rotates, so a single definite marker is possible if the host exposes it. |
| T7 | Player names on the right edge were called clutter by a tester. | [text], early 2020 | [devtrackers.gg thread](https://devtrackers.gg/teamfight-tactics/p/15e47ea8-tft-mobile-early-access-feedback) | The minimum row is portrait plus health. |
| T8 | Augment pick: a modal 1 of 3 with its own timer and a random pick on timeout. | [snippet only]; set and patch not retrieved | [progameguides](https://progameguides.com/teamfight-tactics/what-stages-do-you-get-augments-in-tft) | Matches Owl's rule that an unpicked golden reward gives a random option. |
| T9 | Small icons failed on phones: trait-member icons and carousel units were hard to tell apart. | [text], March 2020 | [Upcomer](https://upcomer.com/teamfight-tactics-mobile-first-impressions/) | Nothing that must be told apart is drawn below 88 px in Owl. |
| T10 | State is printed on the units themselves (health bar, star marker, items). | store screenshots, inspected | [Google Play listing](https://play.google.com/store/apps/details?id=com.riotgames.league.teamfighttactics&hl=en_US) | State goes on the card and the slot, not in a separate widget. |
| T11 | Current positions of gold, level, streak, timer and the player list. | **unverified** [recollection only] | none | Not used. |

### 1.3 Super Auto Pets and Dota Underlords

| # | Pattern | Evidence | Source | What Owl takes |
|---|---|---|---|---|
| S1 | One static screen with everything visible: 5 team slots, a few shop tiles, Roll bottom left, End Turn bottom right, four small counters. | [text], wiki page marked as a stub | [Super Auto Pets wiki](https://superautopets.wiki.gg/wiki/The_Basics) | Readability from a small budget, not from styling. Owl's budget (7, 10, 8, 6 players) sits between this and TFT. |
| S2 | A free roll is shown on the Roll button itself. Freeze is select, then press a button. | [text] | same | State on the button it affects. |
| S3 | Merge by dragging a duplicate onto its twin; a level-up offers a choice from the next tier. | [text] | same | Owl's merge is automatic by rule, so only the reward half applies. |
| S4 | Underlords' phone form was a large shop panel over the board; for PC it had to be undone. Its shop combine cue was made "brighter and more obvious". | [text], June 2019, game since dormant | [InvenGlobal](https://www.invenglobal.com/articles/8439/valve-has-changed-dota-underlords-ui-to-look-less-like-a-mobile-port) | A modal over the board is accepted on phones if it opens and closes fast. |
| S5 | How Super Auto Pets sells, and its phone layout. | **unverified** | none | Not used. |

### 1.4 Quiz question screens

| # | Pattern | Evidence | Source | What Owl takes |
|---|---|---|---|---|
| Q1 | QuizUp (2013 to 2021, historical): 7 questions, the same question to both players, 10 s, 4 options; the screen showed only score, question, answers and both players' names and pictures. | [text] | [Wikipedia](https://en.wikipedia.org/wiki/QuizUp), [Android Authority 2014](https://www.androidauthority.com/quizup-review-355780/) | During the answer window the player looks at the question and little else. QuizUp's exact layout is **unverified** [recollection]. |
| Q2 | A QuizUp match took 2 to 2.5 minutes, so about 17 to 21 s per question against a 10 s window. | [derived] | same | Owl's fixed pauses (2.0 s category, 2.5 s reveal) give at most 13.5 s per question, inside that range. |
| Q3 | Kahoot's phone answers are a 2x2 grid of tiles, each colour paired with a shape, with a timer bar. | caption evidence and [snippet only] | [screensdesign.com](https://screensdesign.com/apps/kahoot-play-create-quizzes/?vs=97291), [Kahoot accessibility](https://kahoot.com/accessibility/) | A fixed identity per option (Owl uses letters A to D) and a bar timer. |
| Q4 | **No phone trivia game was found documented in landscape.** The apps covered are, as far as could be established, portrait. | absence of sources; orientation of each app is [recollection] | none | The landscape layout must be reasoned, not copied. See 2.3. |
| Q5 | Help abilities elsewhere: remove two wrong answers, second attempt, extra time offered when the timer expires, auto-answer; limited per game. Trivia Crack disables the other power-ups once one is used (unofficial guide). | [snippet only] throughout | [etermax help](https://triviacrack.help.etermax.com/hc/en-us/articles/360024775733-Power-ups), [Engadget](https://www.engadget.com/battle-royale-quiz-game-trivia-royale-quizup-130038696.html) | Owl's rules want stacking (Sword plus Second Opinion), so an unused help stays enabled. Where the buttons sit in those apps is **unverified**. |
| Q6 | Escalation is done by rule (double final question, harder questions, elimination). How the stake is drawn is not documented. | [text] for rules | [Wikipedia: HQ](https://en.wikipedia.org/wiki/HQ_(video_game)) | Owl's uncapped rising damage gets one large current-stake number, not a ladder. Original reasoning. |
| Q7 | Accidental taps are frequent even with good hit areas; confirmation dialogs burden every user. | [text], e-commerce testing | [Baymard 2017](https://baymard.com/blog/handling-accidental-taps-on-touch-devices) | One tap locks an answer. Protection comes from size, gaps and placement. |
| Q8 | Average silent reading is 238 words per minute, so 9 s is about 36 words with no time to think. | [text] (search summary of the paper) and [derived] | [Brysbaert 2019](https://reader.ku.edu/sites/reader/files/2024-01/How%20many%20words%20do%20we%20read%20per%20minute%20(1).pdf) | A length guideline for question writers: see open question 13. |
| Q9 | Colour must not be the only signal of correct and wrong. | [text] | [WCAG 2.2 SC 1.4.1](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html) | Check and cross icons, outline weight and hatching on every reveal state. |
| Q10 | In Trivia Crack 2's real-time mode players stared at the question and missed the opponent's progress. | [snippet only]; article returned 403 | [etermax on Medium](https://medium.com/etermax-technology/tower-duel-developing-a-real-time-game-mode-in-trivia-crack-2-b139d930f278) | Opponent information during the window is a glanceable state change; anything to be read waits for the reveal. |

### 1.5 Ergonomics and platform numbers

All pixel figures are in a 1080-high reference layout ("ref px").

| # | Finding | Evidence | Source |
|---|---|---|---|
| E1 | Two-thumb landscape tapping is fastest in the left and right thirds and significantly slower in the centre third (5.2-inch phone, 18 right-handed seated participants). Right third had the lowest error rate, bottom-left quadrant the highest. | [text], controlled study | [Lehmann and Kipp 2018](http://michaelkipp.de/publication/LehmannKipp18.pdf) |
| E2 | No published figure for how far a thumb reaches inward in landscape on a 6.1 to 6.9 inch phone. | gap | none |
| E3 | Targets: 7 mm floor in the centre, 9 mm normal, 10 mm for reliable taps, 12 mm in corners. On a 6.1-inch phone that is 116, 150, 166 and 200 ref px. | [text] and [derived] | [Hoober 2017](https://www.uxmatters.com/mt/archives/2017/07/design-for-fingers-touch-and-people-part-3.php), [Google](https://support.google.com/accessibility/android/answer/7101858) |
| E4 | Text: about 32 ref px smallest, 48 body, 64 or more for anything read at a glance, about 128 for big numerals. | [derived] from Apple and Hoober sizes | [Apple HIG](https://developer.apple.com/tutorials/data/design/human-interface-guidelines/accessibility.json), [Hoober 2017](https://www.uxmatters.com/mt/archives/2017/05/design-for-fingers-touch-and-people-part-2.php) |
| E5 | iPhone landscape safe-area insets: 59 to 68 pt each side, 20 to 29 pt bottom, 20 pt top on iOS 26. Worst case 175 ref px per side, 75 bottom, 54 top. A 19.5:9 iPhone keeps about 2000 ref px of usable width out of 2340. | [text] secondary source, simulator-measured; [derived] | [Use Your Loaf](https://useyourloaf.com/blog/iphone-17-screen-sizes/) |
| E6 | Android: apps targeting SDK 35 always draw under the cutout; the cutout is on one short edge and moves with rotation; an inward edge swipe is the system back gesture. | [text] | [Android cutout guide](https://developer.android.com/develop/ui/views/layout/display-cutout), [gesture guide](https://developer.android.com/develop/ui/views/touch-and-input/gestures/gesturenav) |
| E7 | Godot: `canvas_items` stretch with `expand` aspect on a 1920x1080 base is the documented setup for landscape mobile. `DisplayServer.get_display_safe_area()` returns physical pixels and is not scaled by the stretch (issue open since 4.0). | [text], documentation and engine source; **untested on a device** | [Godot docs](https://docs.godotengine.org/en/latest/tutorials/rendering/multiple_resolutions.html), [issue 74835](https://github.com/godotengine/godot/issues/74835) |
| E8 | Godot Control drag-and-drop starts from emulated mouse motion past `gui/common/drag_threshold` (default 10); only one finger can drive it. `InputEventScreenTouch.long_press` exists on Android and iOS only. | [text], engine source; forum reports unverified | [viewport.cpp](https://raw.githubusercontent.com/godotengine/godot/master/scene/main/viewport.cpp) |

### 1.6 Patterns that cannot be separated from a game's look, with alternatives

| Pattern in the source | Why it is not taken as is | Owl's alternative |
|---|---|---|
| Shopkeeper character at top centre as the sell target and the visual anchor of the shop | A bartender in a tavern is Battlegrounds' identity, and the brief excludes it | No character. The shop is a shelf; the sell target is a labelled zone that appears on drag (2.2). |
| Burning rope as the final timer warning | A Hearthstone signature | A thin bar between shop and board that drains, then thickens and pulses, with a tick sound (3.5). |
| Freeze drawn as ice [recollection, unverified] | Tied to that game's fantasy theme | "Reserved": a ribbon band across the shelf and a tag on each held card. |
| Golden shown as a gold-coloured frame and glow | The most recognisable card treatment in that game | The card inverts to a dark "night edition" with gilt line work and a laurel mark (4.4). |
| Tier shown as stars on a banner | Star banners are the source's badge | Roman numeral on a bookmark tab. |
| Green outline for "playable" | The source's colour convention | A raised position and a corner notch; unavailable items lie flat with a hatch. |
| Cost-tier colours on shop tiles in TFT [recollection, unverified] | The source's colour scheme | Owl cards cost the same in most cases; tier is the numeral, not a colour. |

## 2. Owl's screens

### 2.0 Rules that apply to every screen

**Reference frame.** The viewport is 1920x1080. All layout sits in a **content frame of 1824x950** inside it: 48 px margin left and right, 54 px top, 76 px bottom. Those margins equal or exceed the worst iPhone insets at the top and bottom (54 and 75), so the vertical layout is the same on every phone. All sizes below are in ref px inside the content frame; percentages are of the content frame.

**Wider screens and notches.**

- Root scene: a full-rect background that ignores insets, and a `SafeFrame` (a `MarginContainer`) holding everything interactive.
- `SafeFrame` margins: top = max(54, top inset); bottom = max(76, bottom inset); left = right = max(48, the larger of the two side insets). Using the larger side inset on both sides keeps the layout symmetric when an Android cutout is on one side, and the margins are recomputed on `size_changed` and `orientation_changed`.
- Resulting content width: 1824 at 16:9; about 1990 on a 19.5:9 iPhone (2340 wide minus 175 each side); up to about 2200 on a 20:9 Android phone with a small cutout. Width is capped at 2200 and centred on anything wider. So wide phones gain 170 to 380 px, not 420 to 600: most of the extra width is safe-area padding.
- Each screen states what pins to the safe edges, what stays fixed and what stretches. The general rule: edge columns keep their width and pin to the safe edges; the centre region takes the extra width; cards, buttons and text never scale with width.
- Backgrounds are authored 2520x1440 so they cover 21:9 phones and 4:3 tablets without stretching.
- Screens taller than 16:9 (tablets, unfolded foldables) get a content frame capped at 950 high and centred. They are not a prototype target.
- Draggable cards never start within 100 px of the physical left or right edge; the margins already guarantee this.
- The inset conversion snippet (physical pixels to viewport units by the window-to-viewport ratio) is in the ergonomics notes and repeated in section 7. It is derived from documentation and **untested on a device**.

**Sizes used everywhere.**

| Rule | Value |
|---|---|
| Timed controls (answers, helps) | at least 166 px on the short side, 200 or more preferred |
| Any interactive element | at least 116 px on its short side; 150 for buttons; 200 in a corner |
| Gap between buttons | at least 32 px |
| Gap between cards in a row | at least 16 px, with centres at least 132 px apart |
| Text | 32 px smallest, 48 px body, 64 px or more for glanceable values, 96 to 128 px for big numerals |
| Read-only tokens | at least 88 px, never interactive |

**Input model.** One finger. A touch on a card shows its name and text in a caption strip at once. Releasing without moving more than 40 px is a tap; moving further starts a drag. There is no long-press anywhere in the core loop (pattern T4). Every drag has a tap equivalent through the inspect panel, so the game is playable with one hand, with a mouse, and by players who do not discover the drags. `gui/common/drag_threshold` is raised from 10 to 40; whether that value is in viewport units under `canvas_items` stretch is unverified and must be checked on a device. The drag preview is drawn 90 px up and toward the screen centre from the finger so the card face stays visible.

**No rules in the client.** Every button's enabled state, cost and refusal reason should come from the view. Most do not yet (addition V6 in section 2.7). Until they do, controls are always enabled, the host's refusal is shown as a toast, and the control shakes.

**Persistent nodes.** Screens keep their nodes and update them from the view. The current clear-and-rebuild on every snapshot cannot carry animations or drags.

### 2.1 Hero select

Eight heroes, all shown at once; first pick locks a hero; 30 s (`hero_select_seconds`); a player who does not pick gets a random free hero.

```
+---------------+---------------+---------------+---------------+----------------------------------------+
| TOP STRIP 1824x100:  Choose your scholar (64)     6 seat chips (88)     countdown 24 (96)              |
+---------------+---------------+---------------+---------------+----------------------------------------+
| TILE 260x401  | TILE          | TILE          | TILE          | DETAIL PANEL 688x826                   |
| portrait      |               |               |               | portrait 240    Name (64)              |
| 260x260       |               |               |               |                 Title (40)             |
| name (40)     |               |               |               |                 category chip, HP 35   |
| emblem, HP    |               |               |               |                                        |
+---------------+---------------+---------------+---------------+                                        |
| TILE          | TILE          | TILE, taken:  | TILE, yours:  | Ability name (48)                      |
|               |               | hatch and     | laurel and    | Ability text (36), scrolls             |
|               |               | 'Taken' band  | outline       |                                        |
|               |               |               |               |                                        |
|               |               |               |               +----------------------------------------+
|               |               |               |               | LOCK IN 688x200                        |
+---------------+---------------+---------------+---------------+----------------------------------------+
```

Regions:

| Region | Position and size | % of frame |
|---|---|---|
| Top strip | x 0 to 1824, y 0 to 100 | 100% x 10.5% |
| Hero grid | x 0 to 1112, y 124 to 950; 4 columns x 2 rows of tiles 260x401, gaps 24 | 61% x 87% |
| Detail panel | x 1136 to 1824, y 124 to 950 (688 wide) | 37.7% x 87% |

Size check: 4 x 260 + 3 x 24 = 1112. 2 x 401 + 24 = 826 = 950 - 124. Tile 260x401 is far above the 150 px button minimum. The Lock in button is 688x200, in the bottom-right corner, and meets the 200 px corner rule.

| Element | Shows | Interaction | Data |
|---|---|---|---|
| Title | "Choose your scholar" at 64 px | none | static |
| Countdown | Seconds at 96 px, top right of the strip; pulses under 10 s | none | `phase_deadline` |
| Seat chips | 6 chips, 88 px, in the strip: empty ring, or the portrait of the hero that seat picked; own seat marked "You" | none | `players[].hero` |
| Hero tile | Portrait 260x260; band below with short name (40 px), category emblem (56 px) and starting HP (40 px) | Tap selects the hero for the detail panel. Does not pick. | `Store.heroes` |
| Tile, taken | Portrait desaturated, diagonal hatch, band reads "Taken" with the seat number | Tap still previews; Lock in is disabled for it | `players[].hero` |
| Tile, yours | Laurel mark on the corner, thick outline, raised | none | `me.hero` |
| Detail: identity | Portrait 240x240, name 64 px, title 40 px, category chip, "HP 35" | none | hero def |
| Detail: ability | Ability name (48 px) and text (36 px), scrolls inside the panel if longer than 8 lines | Vertical drag scrolls | **needs view addition V7** (hero data has no ability text). Until then: a stand-in text file in the client copied from `HEROES.md`, clearly marked temporary. |
| Lock in | Button 688x200, label "Lock in" plus the hero's short name | Tap sends `PickHero`. On refusal (`HeroUnavailable`) the tile flips to taken and a toast says the hero was just taken. | command result |
| After locking | Button becomes a status plate "Locked in. Waiting for the others." Tiles stay browsable. | none | `me.hero` |

Why a confirm step here when answers have none: the pick is irreversible, lasts the whole match, and needs the ability text to be read first. The cost is one extra tap during a race for a hero; see open question 6.

Wide screens: the grid pins to the left safe edge at fixed size; the detail panel pins to the right safe edge and grows in width up to 900; the gap between them takes the rest. The top strip stretches.

### 2.2 Shop

One fixed screen, nothing scrolls, nothing essential is behind a tap.

```
+--------------+------------------------------------------------------------+------------------+
| RAIL 200x950 | CAPTION STRIP 1344x96: turn, own HP and armor, chips;      | TIMER 240x120    |
| row 1 (You)  | or the touched card's name and text; or a toast            | 23      Turn 7   |
+--------------+------------------------------------------------------------+------------------+
| row 2        | SHOP ROW 1344x220        cards 152x220                     | GOLD 240x96      |
| (next opp.)  | [o1] [o2] [o3] [o4] [o5] [o6]    :    [x1] [x2]            | coin   7         |
+--------------+                                                            +------------------+
| row 3        | up to 6 offers                   :    up to 2 extras       | UPGRADE 240x150  |
|              | ========== TURN TIMER BAR 1344x12 ==========               | III > IV   cost 5|
+--------------+                                                            +------------------+
| row 4        | BOARD ROW 1344x254       7 slots 176x254                   | REROLL 240x150   |
|              | [b1] [b2] [b3] [b4] [b5] [b6] [b7]                         | cost 1           |
+--------------+                                                            +------------------+
| row 5        | Hand 6/10        (labels, 36 px)        Board 5/7          | RESERVE 240x150  |
|              | HAND ROW 1344x220        cards 152x220                     | (freeze toggle)  |
+--------------+                                                            +------------------+
| row 6        | [h1] [h2] [h3] [h4] [h5] [h6]                              | HERO PLATE       |
| each 200x150 | (overlap from the 9th card; 52 px slack below)             | 240x204          |
+--------------+------------------------------------------------------------+------------------+
```

Regions:

| Region | Position and size | % of frame |
|---|---|---|
| Player rail | x 0 to 200, y 0 to 950; 6 rows of 200x150, gaps 10 | 11% x 100% |
| Centre | x 220 to 1564 (1344 wide) | 73.7% x 100% |
| Caption strip | centre, y 0 to 96 | 73.7% x 10% |
| Shop row | centre, y 112 to 332; cards 152x220 | 73.7% x 23% |
| Turn timer bar | centre, y 346 to 358 | 73.7% x 1.3% |
| Board row | centre, y 372 to 626; 7 slots of 176x254 | 73.7% x 26.7% |
| Row labels | centre, y 634 to 670 | 73.7% x 3.8% |
| Hand row | centre, y 678 to 898; cards 152x220 | 73.7% x 23% |
| Controls column | x 1584 to 1824 (240 wide), y 0 to 950 | 13.2% x 100% |

Size checks (the tightest cases):

- **Shop row, 6 offers plus 2 extras:** 6 x 152 + 5 x 16 = 992; divider 24; 2 x 152 + 16 = 320; total 1336 of 1344. Card width 152 px. With fewer offers the row is centred with the same gaps.
- **Board, 7 slots:** 7 x 176 + 6 x 16 = 1328 of 1344. Card width 176 px.
- **Hand, 10 cards:** up to 8 cards sit side by side (8 x 152 + 7 x 16 = 1328). At 9 and 10 the cards overlap with a step of 132 px (9 x 132 + 152 = 1340), leaving a 132 px strip of each card to touch, above the 116 px floor.
- **Player rail, 6 rows:** 6 x 150 + 5 x 10 = 950.
- **Controls column:** 120 + 96 + 150 + 150 + 150 + 204 + 5 x 16 = 950.
- **Centre column height:** 96 + 16 + 220 + 14 + 12 + 14 + 254 + 8 + 36 + 8 + 220 = 898, leaving 52 px of slack at the bottom.

What is always visible: all six players with HP, armor and tier; the whole shop; the whole board; the whole hand; gold; own tier and the upgrade cost; reroll and freeze; the timer; the hero ability. What is behind a tap: card rules text at full size (inspect panel), another player's details, the golden reward options once hidden, and the hero-ability pickers.

Elements and interactions:

| Element | Shows | Interaction | Data |
|---|---|---|---|
| Player rail row (x6) | Portrait 112 px, HP numeral 48 px, armor numeral 32 px with a shield icon, tier numeral on a tab. Sorted by HP, eliminated last. Own row has a thick outline and "You". | Tap opens the player detail panel to the right of the rail (520x600), dimming the rest; tap outside closes. | `players[]` |
| Next-opponent marker | The row shifts 16 px right and gains a crossed-quills icon | none | **needs view addition V1.** Until then: no marker. |
| Streak mark | A tally of up to 5 strokes on the row | none | **needs view addition V2** for other players; own streak is `me.win_streak`. Until then: own row only. |
| Caption strip, idle | "Turn 7", own HP and armor at 48 px, status chips (frozen, reward waiting, endgame effect chosen, overtime) | Tap the reward chip to reopen the reward overlay | `turn`, `me`, `me.rewards`, `me.endgame_choice`; overtime chip **needs V15** |
| Caption strip, card touched | Card name in bold, tier, type, then the rules text, 32 px, two lines | none; it follows the finger's current card | card def text; golden and combined text **needs V8** |
| Toast | A refusal or notice replaces the caption for 2 s ("Not enough gold") | none | error string from `send`; a stable code **needs V10** |
| Shop offer | Small card with a cost coin top left | Drag down onto the board or hand area: `Buy`. Tap: inspect panel with a Buy button. | `me.shop`, card defs |
| Extra offer (potion, skull) | Same card on a separate plinth after a divider, labelled "Extra" | Drag down or tap then Buy: `BuyExtra` | `me.extras` |
| Affordability | Offers the player cannot pay for lie flat and hatched | none | **needs V6.** Until then: no state; refusal toast. |
| Golden hint | A laurel pip on an offer that would complete a golden card | none | **needs V19.** Until then: none. |
| Frozen state | A ribbon band across the shop row and a "Reserved" tag on each offer | none | `me.frozen` |
| Turn timer bar | Drains left to right over the shop phase. Under 10 s it doubles in height and pulses once a second with a tick; under 5 s the timer numeral pulses too. | none | `phase_deadline`, `TurnStarted` timing |
| Board slot (x7) | Board-size card, or an empty outlined slot. Shows armor left on shields and a hatch plus skull stamp when disabled. | Tap: inspect panel. Drag to the Sell zone: `Sell`. | `me.board` |
| Hand card (up to 10) | Small card | Drag up onto the board: `Place`. Drag to the Sell zone: `Sell`. Tap: inspect panel with Place or Use, Sell, and any hero or endgame action. | `me.hand` |
| Row labels | "Hand 6/10" left, "Board 5/7" right, 32 px | none | counts, `tunables` |
| Timer block | Seconds at 96 px, "Turn 7" at 32 px | none | `phase_deadline`, `turn` |
| Gold block | Coin icon and amount at 72 px | none | `me.gold` |
| Upgrade button | 240x150. Tier numeral with an arrow to the next, and the cost. At the top level it reads "Top level" and lies flat. | Tap: `UpgradeTavern` | `upgrade_cost`, `me.tavern` |
| Reroll button | 240x150. Icon and cost | Tap: `Reroll` | `tunables.reroll_cost`; a per-player cost **needs V6** |
| Freeze button | 240x150. Reads "Reserve" or "Release"; shows the toggled state by a pressed-in look and the ribbon icon | Tap: `ToggleFreeze` | `me.frozen` |
| Sell zone | While an owned card is being dragged, the three buttons are replaced by one zone, 240x482, labelled "Sell" with the gold returned; everything that is not a drop target dims | Drop: `Sell` | `tunables.sell_value` |
| Hero plate | 240x204, bottom-right corner. Portrait, ability name, cost or uses left. Heroes with no shop action show their passive. | Tap: see the hero ability table below | hero def; uses, cost and availability **need V6**; text **needs V7** |

**Why the controls are on the right and the list on the left.** The shop is timed, so its controls belong in an outer third (E1). The right third had the lowest error rate for right-handed players, and reroll, upgrade and sell cost gold. The player list is read far more than it is touched, so it takes the left edge. Positions are the same in every state (T5). A mirrored layout for left-handed players is open question 8.

**Buy, sell, place.** Buying is a drag down from the shop into the player's own area; selling is a drag sideways to the right edge; placing is a drag up from the hand to the board. The three gestures go in three different directions, which is the same protection Battlegrounds gets from opposite drop targets (B4). Dropping a shop card on the board sends `Buy` only; the card lands in the hand (open question 12 covers buy-and-place).

**Reordering.** There is no command to reorder the board or to return a board card to the hand, and `Place` takes no slot. A placed card goes where the host puts it. Drop position on the board is therefore ignored. See open question 3.

**Inspect panel.** A tap on any card opens it enlarged (420x608) in the centre with a column of action buttons (280x150 each) to its right and a dimmed background. Tap outside or on the close button (116x116) to dismiss. The buttons come from the view's action list (**needs V6**). Until then the client keeps today's effect-name checks in one clearly marked adapter script (open question 1).

**Golden card and reward.** When `GoldenMade` arrives the merge animation plays (section 6). `RewardOffered` then adds a pulsing "Reward" chip to the caption strip and, if no drag is in progress, opens the reward overlay: a banner "Choose one" with the shop countdown, three enlarged cards (3 x 420 + 2 x 40 = 1340 of 1344 wide; banner 110 + card 608 + Hide button 150 with gaps fits in 950), and a Hide button bottom left of the centre region that collapses the overlay back to the chip so the board can be read (B9). Tap a card, then tap the "Take" button under it: `PickReward`. Two taps because the cards are large and the choice is permanent, and the shop timer is the only pressure. If the shop closes first the pick becomes a reward card in the hand, so the overlay says "If time runs out, the pick waits in your hand as a reward card".

**Hero abilities in the shop.**

| Hero | What the plate shows | Flow | Command |
|---|---|---|---|
| Herodotus | "Rewrite", locked with a padlock and "Needs top level" until available; then "1 use" | Hold the plate for 0.6 s with a fill ring (the action is once per game) | `UseAbility Rewrite` |
| Darwin | "Evolve", uses left | Tap the plate: targeting mode with a banner "Pick the card to evolve" and a Cancel button; cards that qualify lift (**needs V6**, else all cards stay selectable); tap a card | `UseAbility DarwinMerge {def}` |
| Benz | "Combine", uses left | Tap the plate: targeting mode "Pick two cards"; first pick is marked "1"; second pick sends; Cancel is always visible | `UseAbility Combine {a, b}` |
| Lumiere | "Record", cost | Tap: a picker listing the film questions of the last battle (question text, 40 px, rows 150 px tall); tap one | `UseAbility Record {question}`; the list text **needs V11**. Until then: rows read "Question 2 (Film)". |
| Nietzsche | "Ubermensch", cost | Tap pays and the host answers with `HouseOptionsShown`. A picker shows the 4 categories as tiles 300x300; tap two different ones; the second tap sends. The picker cannot be dismissed while `me.hero_state.house_options` is non-empty, but has a Hide button like the reward. | `UseAbility Ubermensch`, then `UseAbility ChooseHouse {first, second}` |
| Shakespeare | "To be or not to be", "Costs all your gold" | Hold 0.6 s with a fill ring. Must be the first action of the round; the host enforces it. If the round is skipped the screen shows the standings in its "sitting out" form. | `UseAbility ToBeOrNotToBe` |
| Newton, Herophilus | Passive text only | Tap opens the ability text | none |

The plate is always enabled until V6 exists; a refusal ("ability requirements are not met", "no charges left") shows as a toast.

**Endgame effect choice.** A card with an endgame effect has an "Endgame effect" button in its inspect panel (`ChooseEndgameEffect`). The chosen card carries a laurel-and-hourglass mark and the caption strip shows a chip with its name (`me.endgame_choice`). When the host says the next phase is the endgame and the player holds more than one such card with none chosen, a prompt lists them as enlarged cards with a "Use in the endgame" button each. Knowing that the endgame is next **needs V13**; until then there is no prompt, only the button and the chip. What happens when none is chosen is open question 10.

**Player detail panel.** 520x600 beside the rail: portrait 200 px, hero name and title, HP, armor, tier, ability text (V7), and the last two battle results (V2). It must stay inside the content frame (the Battlegrounds overflow report in B12 is the cautionary case).

Wide screens: the rail pins to the left safe edge and the controls column to the right safe edge, both at fixed width. The centre region stretches; cards keep their size and the gaps between them grow up to 32 px, after which the rows are centred.

### 2.3 Battle question

This screen is original design. Section 1.4 found no landscape phone quiz to learn from, so the layout below is reasoned from the thumb-reach study (E1), the target sizes (E3), the occlusion rule that feedback goes above and inward of the thumb, and the genre rule that persistent elements do not move (B14, T5). It has not been tested on a device and should be the first screen put on a real phone.

**Alternatives considered.**

| Layout | For | Against | Verdict |
|---|---|---|---|
| A. Question band on top, 2x2 answer grid below, full width | The familiar quiz shape (Q3). Shortest eye travel between options. Each tile reaches a screen edge, so a thumb can hit its outer end. | Height. A status strip, a three-line question and two rows of 200 px answers use about 860 of 950 px. Nothing is left for the opponent's board, the player's board or the helps, which Owl must show beside the question. | Rejected for Owl; kept as the fallback to test (open question 9). |
| B. Question on the left, four answers stacked on the right | A vertical list is the easiest shape to scan; one thumb does everything. | 4 x 200 + 3 x 32 = 896 of 950 px: the column is full, with no room for helps or the opponent on that side. All input goes to one hand. | Rejected; a candidate for a one-handed option. |
| C. "Wings": two answers under each thumb, question and boards in the centre | Every timed control is in an outer third, where tapping is fastest. The centre, slowest to tap and never covered by thumbs, holds everything that is only read. Answers can be 250 px tall. Leaves the whole centre column for question, timer and both boards. | The eye crosses the centre between option pairs (about 66 mm on a 6.1-inch phone, roughly 12 degrees at 30 cm). No shipped example was found. A one-handed player must reach across. | **Chosen.** |

Layout C is chosen because Owl's question screen has to carry more than a quiz app's (two boards, helps, score), and only C has the room while keeping answers large and at the edges. The eye-travel cost is reduced by putting each answer's text at the inner end of its button, next to the question, and the letter badge at the outer end under the thumb.

```
+-----------------------------+------------------------------------+-----------------------------+
| OWN PLATE 552x146           | CATEGORY CHIP           TIMER      | OPPONENT PLATE 552x146      |
| portrait HP armor  score 3  | emblem, name               9       | score 2  armor HP portrait  |
| (tap: own board)            | Question 3 of 6                    | (tap: their board)          |
+-----------------------------+------------------------------------+-----------------------------+
| LEDGER 552x200              | ===== TIMER BAR 672x24 =====       | HELPS 3 x 168x200           |
| you   v  x  v  .  .  .      | QUESTION PANEL 672x440             | [Second ] [Flaming] [The   ]|
| them  x  x  v  .  .  .      |                                    | [Opinion] [Sword  ] [Oracle]|
+-----------------------------+                                    +-----------------------------+
| ANSWER A 552x250            | question text, 60 px,              | ANSWER B 552x250            |
| (A)   answer text           | up to 4 lines                      | answer text   (B)           |
|                             |                                    |                             |
+-----------------------------+------------------------------------+-----------------------------+
| ANSWER C 552x250            | OPPONENT'S BOARD  7 tokens x 88    | ANSWER D 552x250            |
| (C)   answer text           | [t] [t] [t] [t] [t] [t] [t]        | answer text   (D)           |
|                             | YOUR BOARD        7 tokens x 88    |                             |
|                             | [t] [t] [t] [t] [t] [t] [t]        |                             |
+-----------------------------+------------------------------------+-----------------------------+
```

In the wireframes, `v` is a check (right), `x` a cross (wrong), `.` a question not yet asked, and a number in brackets is a text size in px.

Regions:

| Region | Position and size | % of frame |
|---|---|---|
| Left wing | x 0 to 552 | 30.3% wide |
| Centre column | x 576 to 1248 (672 wide) | 36.8% wide |
| Right wing | x 1272 to 1824 (552 wide) | 30.3% wide |
| Player plates | each wing, y 0 to 146 | 30.3% x 15.4% |
| Ledger (left) and helps (right) | each wing, y 174 to 374 (200 tall) | 30.3% x 21% |
| Answers A and B | each wing, y 414 to 664 (552x250) | 30.3% x 26.3% |
| Answers C and D | each wing, y 700 to 950 (552x250) | 30.3% x 26.3% |
| Category and timer row | centre, y 0 to 100 | 36.8% x 10.5% |
| Timer bar | centre, y 112 to 136 | 36.8% x 2.5% |
| Question panel | centre, y 152 to 592 (672x440) | 36.8% x 46.3% |
| Opponent's board | centre, label y 612 to 648, tokens y 652 to 740 | 36.8% x 13.5% |
| Own board | centre, label y 760 to 796, tokens y 800 to 888 | 36.8% x 13.5% |

Size checks:

- **Width:** 552 + 24 + 672 + 24 + 552 = 1824.
- **Wing height:** 146 + 28 + 200 + 40 + 250 + 36 + 250 = 950.
- **Answer button:** 552x250, about 33 x 15 mm on a 6.1-inch phone, above the 200 px preferred size; gaps of 36 px between the two on a side and 40 px to the helps.
- **Help buttons:** three of 168x200 with 24 px gaps = 552. Each is above the 166 px timed minimum.
- **Board tokens:** 7 x 88 + 6 x 8 = 664 of 672. Tokens are read-only, so the 116 px touch floor does not apply; the glyph inside is 48 px and the value 32 px.
- **Question text:** 672 px holds about 23 characters per line at 60 px, so four lines hold about 90 characters (12 to 14 words). Longer questions drop to 48 px and five lines (about 140 characters). That keeps lines far under the 80-character cap.
- **Answer text:** the text zone is the inner 388 px of the button; at 48 px that is about 16 characters per line and three lines; longer answers drop to 40 px.

Elements and interactions:

| Element | Shows | Interaction | Data |
|---|---|---|---|
| Own plate (left) | Portrait 120 px, name, HP bar with numeral (48 px), armor numeral, running score (64 px) at the inner end | Tap opens own board at card size (overlay) | `me`, `battle.my_correct` |
| Opponent plate (right) | Mirror of the own plate. A ghost is drawn translucent and labelled "Ghost of" plus the name. A small quill moves while they have not answered; a padlock replaces it when they lock in. | Tap opens the opponent's board overlay: 7 board-size cards across the frame with the caption strip. It closes on a tap outside and by itself when the next question opens. | `battle.opponent`, `ghost_of`, `players[]`, `opponent_locked`, `opponent_correct` |
| Ledger (left wing) | Two rows of 6 cells, 72 px each: own answers and the opponent's, a check or a cross per finished question, the current one outlined | none | **needs view addition V5.** Until then the client fills cells from `AnswerRevealed` events and loses them on reconnect. |
| Helps (right wing) | Up to three buttons, 168x200: Second Opinion, Flaming Sword, The Oracle. Icon, name at 32 px, uses left at 48 px. Only helps the player had at battle start are shown; a spent help stays in place, flat and hatched, with "0". | One tap, no confirmation: `UseHelp`. Disabled before the question opens and after locking in. A second help may be used on the same question. | `second_opinion_left`, `sword_left`, `oracle_left` |
| Category chip | Emblem 72 px and name at 48 px; "Question 3 of 6" at 32 px under it | none | `battle.category`, `question_number`, `question_count` |
| Slot tag | "House", "Yours" or "Theirs" beside the chip | none | **needs view addition V17.** Until then: none. |
| Timer numeral | Seconds left at 96 px, right end of the row | none | `battle.deadline` |
| Timer bar | Full and still during the category, reading and reveal stages; drains during the question. While reading, a slim calm bar fills toward `answers_at` with a "Read…" label. Bonus time shows as a longer drain, not a tap. Under 3 s the bar doubles in height, the numeral pulses each second and a tick plays. | none | `opens_at`, `deadline` |
| Question panel | The question at 60 px, centred, on a paper panel | none | `question_text` (offline today; online delivery is part of V11) |
| Board tokens | One 88 px token per card: type glyph, one value, golden mark, armor left on shields, hatch and skull stamp on a disabled card | none (use the plates) | `opponent_board`, `me.board` |
| Answer buttons A to D | Letter badge (96 px disc, 64 px letter) at the outer end, answer text at the inner end. A and C on the left, B and D on the right, read in Z order. | One tap sends `SubmitAnswer` and locks. No confirmation, no change. | `question_text.options`, `battle.removed`, `battle.locked` |

Why the helps are on the right, above the answers: a wasted help cannot be taken back, the right third has the lowest error rate, and the bottom-left quadrant has the highest (E1). Putting them above the answers keeps them out of the path of a thumb travelling to an answer, with a 40 px gap. They are in reach but not underfoot.

**Stages of one question.** The host fixes the pauses (the reading time, `answer_reveal_ms` 1500); the client fills them and never extends them. The category shows with the question, not before it.

| Stage | Host-clock window | Centre column | Wings |
|---|---|---|---|
| Battle intro (first question only) | first 700 ms after `BattleStarted` | Versus card: both portraits, names, "Ghost of" if a ghost | Plates slide in; answers blank |
| Category | from `opens_at` minus 2000 ms (1300 ms on the first question, after the intro) | Category emblem at 240 px, name at 96 px, "Question 3 of 6". Timer bar full. | Answer buttons present but blank and hatched, so nothing moves when the question opens. Helps flat. |
| Reading | from `opens_at` to `answers_at` (6 s plus 150 ms a word, at most 10 s) | Question text; timer bar full and still, a slim reading bar fills under it | Answers shown but inactive; helps flat |
| Question | from `answers_at` to lock-in or `deadline` | Question text; timer draining | Answers and helps live, with a short soft cue as they open |
| Locked, waiting | from own lock-in until `AnswerRevealed` | Unchanged | Own pick outlined with the own-portrait marker; the other three dim. Helps flat. |
| Reveal | 2500 ms from `AnswerRevealed` | Question stays; timer bar still | Correct answer: thick outline, check icon, raised. Own wrong pick: cross icon and hatch. Each plate shows a check or a cross; the ledger cell fills; the score ticks. |

- A removed option stays in place, faded to 30% with a strike line through the text and no response to touch, so the layout does not shift.
- The opponent's pick is shown at the reveal as their portrait marker on the button's opponent-side end. This **needs view addition V3** (`AnswerRevealed` carries only who was right). Until then only the check or cross on their plate is shown. The player's own pick is remembered locally.
- When the Oracle answers, the view only reports "locked", so the plate reads "Answered by the Oracle" and no button is marked until the reveal.
- The question text and its options appear together at `opens_at` and are read until `answers_at`; the host refuses an answer before then (decided after a tester could not read a question in 9 seconds).

Wide screens: both wings keep 552 px and pin to the safe edges. The centre column takes all the extra width (about 838 on an iPhone, up to 1048). The question panel grows to at most 900 px wide and stays centred; tokens grow to at most 116 px (7 x 116 + 6 x 12 = 884).

### 2.4 Battle result

Shown from `BattleScored` until the turn resolves. Damage is dealt only when every battle of the turn is done, so the screen has two beats: the verdict straight away, and the damage when `DamageDealt` arrives.

```
+-----------------------------+------------------------------------+-----------------------------+
| OWN SIDE 552x950            | VERDICT   Victory (128)            | OPPONENT SIDE 552x950       |
| splash 552x736              | SCORE     5 - 3 (128)              | splash 552x736              |
|                             +------------------------------------+                             |
|                             | QUESTION LEDGER 672x256            |                             |
|                             | cat   Hi  Fl  Ph  Hi  Sp  Ge       |                             |
|                             | you   v   x   v   v   x   v        |                             |
|                             | them  x   x   v   v   v   x        |                             |
|                             +------------------------------------+                             |
|                             | DAMAGE LINES 672x264               |                             |
|                             | You deal 7                         |                             |
|                             | 3 blocked by armor                 |                             |
+-----------------------------+                                    +-----------------------------+
| name                        | They lose 4 HP                     | name                        |
| HP bar and numeral, armor   | 2 reflected back at you            | HP bar and numeral, armor   |
|                             +------------------------------------+                             |
|                             | STATUS: Waiting / Tap to continue  |                             |
+-----------------------------+------------------------------------+-----------------------------+
```

| Region | Position and size | % of frame |
|---|---|---|
| Own side | x 0 to 552, full height | 30.3% x 100% |
| Verdict and score | centre, y 0 to 300 | 36.8% x 31.6% |
| Question ledger | centre, y 320 to 576 | 36.8% x 27% |
| Damage lines | centre, y 596 to 860 | 36.8% x 27.8% |
| Status line | centre, y 880 to 950 | 36.8% x 7.4% |
| Opponent side | x 1272 to 1824, full height | 30.3% x 100% |

Size check: ledger cells 6 x 96 + 5 x 12 = 636 of 672; a header row of category emblems (64 px) plus two rows of 96 px cells and gaps is 256 px. Four damage lines of 64 px fit in 264 px.

| Element | Shows | Interaction | Data |
|---|---|---|---|
| Verdict | "Victory", "Defeat" or "Tie" at 128 px, with a laurel, a broken quill or a balance icon | none | `BattleScored` compared for own side; in the view **needs V18** |
| Score | "5 - 3" at 128 px | none | `BattleScored` |
| Question ledger | A column per question: category emblem, own check or cross, opponent's check or cross | none | **needs V5**; until then from accumulated events |
| Side panels | Splash portrait 552x736, name, HP bar and armor. HP and armor animate when damage lands. | none | `players[]`, `DamageDealt` |
| Damage lines | Before damage: "Waiting for the other battles". After: one line per `DamageDealt` that involves either fighter, for example "You deal 7", "3 blocked by armor", "They lose 4 HP", "2 reflected back at you". The source has an icon and a word: battle, tie-breaker, 0-0, overtime, reflect, arrow, ability. | none | `DamageDealt {to, source, amount, armor_absorbed, hp_lost}` |
| Damage sum | The reason for the number: margin + base + tier + each card bonus, then multipliers and the cap | none | **needs view addition V4.** Until then only the lines above. |
| Status line | "Tap to continue" once damage has been shown | Tap anywhere skips to the shop | none |

**Pacing.** Today the next `TurnStarted` arrives in the same step as the damage, and the shop timer starts with it. Any time this screen holds after that is taken from the 30 s shop (the current client holds 2.5 s). This design holds for at most 1500 ms after the damage events, lets a tap skip it, and continues the HP changes on the player rail if the screen is left early. The better fix is a host-side resolve pause, like the reveal pauses; that is open question 5.

Wide screens: side panels pin to the safe edges at 552 px; the centre column stretches and its contents stay centred at 672 px.

### 2.5 Endgame

The endgame keeps the battle question skeleton so that the answer buttons are exactly where the player's thumbs have learned them. What changes is what the centre says the question is worth.

```
+-----------------------------+------------------------------------+-----------------------------+
| OWN PLATE 552x146           | STAKE                   TIMER      | OPPONENT PLATE 552x146      |
| portrait, name              | Damage  4 (128)           10       | name, portrait              |
|                             | (steps up after each question)     |                             |
+-----------------------------+------------------------------------+-----------------------------+
| OWN HP 552x200              | ===== TIMER BAR 672x24 =====       | THEIR HP 360   ORACLE       |
| HP 21 (128)   armor 3       | QUESTION PANEL 672x440             | HP 17 (128)    168x200      |
| chip: endgame effect        | category chip on top               | armor 0        uses 1       |
+-----------------------------+                                    +-----------------------------+
| ANSWER A 552x250            | question text, 60 px,              | ANSWER B 552x250            |
| (A)   answer text           | up to 4 lines                      | answer text   (B)           |
|                             |                                    |                             |
+-----------------------------+------------------------------------+-----------------------------+
| ANSWER C 552x250            | OPPONENT'S BOARD  7 tokens x 88    | ANSWER D 552x250            |
| (C)   answer text           | [t] [t] [t] [t] [t] [t] [t]        | answer text   (D)           |
|                             | YOUR BOARD        7 tokens x 88    |                             |
|                             | [t] [t] [t] [t] [t] [t] [t]        |                             |
+-----------------------------+------------------------------------+-----------------------------+
```

Regions are those of 2.3 with these differences:

| Region | Regular battle | Endgame |
|---|---|---|
| Left wing, y 174 to 374 | Ledger | Own HP at 128 px, armor at 64 px, and the chosen endgame effect as a chip |
| Right wing, y 174 to 374 | Three helps | Opponent's HP at 128 px and armor on the inner 360 px; the Oracle button (168x200) at the outer end if the player has it |
| Centre, y 0 to 100 | Category chip and timer numeral | The stake: "Damage" at 32 px over the current damage at 128 px, with the timer numeral beside it |
| Category chip | In the top row | Moves to the top of the question panel |
| Score and "Question n of 6" | Shown | Not shown; there is no score in the endgame |

Size check: on the right wing 360 + 24 + 168 = 552. Everything else is unchanged from 2.3.

| Element | Shows | Interaction | Data |
|---|---|---|---|
| Stake numeral | The damage this question is worth. Between questions the old value is struck and the new one stamps in, one step higher. | none | `battle.question_number` (the endgame step) |
| HP blocks | Both players' HP and armor, large, because HP is the only score | none | `players[]` |
| Endgame effect chip | Own chosen effect, with its name | Tap shows the card | `me.endgame_choice`; the opponent's choice **needs V13** |
| Oracle button | As in 2.3, with the total uses left | One tap: `UseHelp Oracle` | `battle.oracle_left` |
| Answers | As in 2.3. With Narrowed Choice the host removes an option; it shows in the removed state. | One tap | `battle.removed` |
| Reveal | Two beats inside the 2500 ms pause. First, which answer was right and a check or cross on each plate. Second, the stake numeral strikes the HP of whoever was wrong; armor cracks first. Both right: the stake flashes "No damage" and still steps up. Both wrong: both are hit together. | none | `AnswerRevealed`, `DamageDealt` with source `EndgameStep`, `ShieldBroken` |
| Finish | "Final duel won" or "Shared victory" with both portraits | none | `SharedVictory`, `MatchFinished` |

Eliminated players receive the endgame's question and answer events but their view has no `battle`, so they cannot be shown this screen from the view alone. A watch mode **needs view addition V14**. Until then they see the standings screen, with both finalists' HP updating live from `players[]` and `DamageDealt`.

Wide screens: as 2.3.

### 2.6 Standings and results

One screen, four forms: sitting out a round, eliminated, match over, and winner.

```
+---------------------------------------------------------+------------------------------------+
| STANDINGS header 1100x80                      Turn 7    | RESULT PANEL 700x726               |
+---------------------------------------------------------+                                    |
| row 1100x130: place, portrait, name, HP bar, armor, tier| You placed 3rd of 6                |
+---------------------------------------------------------+                                    |
| row 2                                                   | (numeral at 200 px)                |
+---------------------------------------------------------+                                    |
| row 3 (You): thick outline                              | own splash                         |
+---------------------------------------------------------+                                    |
| row 4                                                   | 41 of 60 answered correctly        |
+---------------------------------------------------------+------------------------------------+
| row 5: eliminated, desaturated, placement numeral       | ACTION BUTTON 700x200              |
+---------------------------------------------------------+                                    |
| row 6                                                   | Play again / Leave match           |
+---------------------------------------------------------+------------------------------------+
```

| Region | Position and size | % of frame |
|---|---|---|
| Player list | x 0 to 1100; header y 0 to 80; 6 rows of 1100x130, gaps 12, from y 96 | 60.3% x 100% |
| Own result panel | x 1124 to 1824 (700 wide), y 0 to 726 | 38.4% x 76.4% |
| Action button | x 1124 to 1824, y 750 to 950 (700x200) | 38.4% x 21% |

Size check: 80 + 16 + 6 x 130 + 5 x 12 = 936 of 950. The button is 700x200 in the bottom-right corner.

| Element | Shows | Interaction | Data |
|---|---|---|---|
| Header | "Standings", and "Turn 7" while the match runs | none | `turn`, `phase` |
| Player row | Placement numeral (64 px) once placed, portrait 112 px, hero name (48 px), HP bar and numeral, armor, tier tab. Living players first by HP, then the eliminated by placement. Own row outlined and marked "You". | Tap opens the player detail panel | `players[]` |
| Result panel, sitting out | "Sitting out this round" and the reason line | none | `me.skip_battle`, phase |
| Result panel, eliminated | "You placed 3rd of 6" with the numeral at 200 px, own splash, and "41 of 60 answered correctly" | none | `me.placement`, `me.correct`, `me.asked` |
| Result panel, match over | The same, plus the winner's name; first place gets a laurel; a shared victory shows both finalists with laurels | none | `MatchFinished`, `SharedVictory` |
| Action button | "Play again" when the match is finished. While it runs and the player is out: "Leave match". | Tap | client |

Wide screens: the list pins left and the panel and button pin right at fixed widths; the gap grows.

### 2.7 Data the view does not carry yet

Every element marked "needs view addition" above, in one place. "Field needed" is a suggestion for `crates/lobby/src/view.rs`, `crates/core/src/api.rs` or the content files; each is a host change with its own test, not client work. None of these changes a game rule except where a row says so.

| # | Field needed | Used by | Until it exists |
|---|---|---|---|
| V1 | `next_opponent` (seat, or the seat a ghost copies) during the shop | Shop rail marker; player detail | No marker. Also needs a rule decision on whether the pairing is known before the shop ends: open question 14. |
| V2 | Per player: win or loss streak and the last two results | Rail streak mark; player detail | Own streak only (`me.win_streak`); no history. |
| V3 | In `AnswerRevealed`: the option each side chose (or none) | Battle and endgame reveal markers | Own pick remembered locally; opponent shown only as right or wrong. |
| V4 | In `DamageDealt`: the breakdown that `ARCHITECTURE.md` section 6 describes (margin, base, tier, each bonus with its card, multipliers, cap, reflected amount) | Battle result damage sum | Amount, armor absorbed and HP lost only. |
| V5 | In `BattleView`: per finished question, its category and whether each side was right | Battle ledger; result ledger | Built from `AnswerRevealed` events; blank after a reconnect. |
| V6 | For every shop offer, extra offer, hand card, board card, shop button, hero ability and help: the list of actions with `usable`, cost, charges left and a reason code, as `ARCHITECTURE.md` section 7 describes | Shop (affordability, action buttons, hero plate, sell value), battle helps | Everything enabled; refusals shown as a toast; the action buttons for a card come from a temporary adapter that reads effect names (open question 1). |
| V7 | Hero ability name, short text and full text, generated from hero data | Hero select detail, hero plate, player detail | A temporary text file in the client copied from `HEROES.md`. |
| V8 | Generated text for golden and combined card instances | Caption strip, inspect panel, enlarged card | Base text per part plus a "Golden" tag; the author-written "Golden: ..." sentences where they exist. |
| V9 | `EffectTriggered { source, ... }` event, as `ARCHITECTURE.md` section 7 describes | Per-card trigger animations (Consolation, Coin Purse, Healer, Hunter, Aegis, tie cards) | No source card highlight; only the resulting event (`CardGained`, `Healed`, `DamageDealt`) animates. |
| V10 | A stable reason code on every refusal, not only the English message | Toasts, shake on the right control, localisation later | The message string is shown as is. |
| V11 | Question text and options for past questions of the own last battle; and the online delivery of the current question's text (today the offline extension adds it) | Lumiere's Record picker; battle screen online | Picker rows read "Question 2 (Film)". |
| V12 | Player display names | Rail, plates, standings, seat chips | Hero names; "Player 3" before a pick. |
| V13 | A flag that the next phase is the endgame; the opponent's chosen endgame effect | Endgame-effect prompt in the shop; endgame plates | No prompt, only the button and chip; opponent's effect not shown. |
| V14 | A watch-only battle view of the endgame for eliminated players | Endgame watch mode | Standings with live HP. |
| V15 | An overtime-active flag | Overtime chip in the shop and on the result screen | No chip; `DamageDealt` with source `Overtime` still explains itself. |
| V16 | Computed stats for the own player: total weight, reflect percentage, answer time, progress toward each category bonus | A stats line in the own-board overlay; category bonus pips | Not shown. |
| V17 | Whose slot each question is (house, own, opponent's) | Slot tag on the category chip | Not shown. |
| V18 | Score and outcome of the own last battle | Result screen after a reconnect | From the `BattleScored` event of this session. |
| V19 | Per offer: buying it would complete a golden card | Laurel pip on shop offers | Not shown. |
| V20 | Display names for categories and for per-category cards (today the slug shows, as in "Apprentice (flora_and_fauna)") | Every card name and category label | The client title-cases the slug. |

## 3. Components

Each component is its own scene with exported parameters and a preview scene under `client/scenes/preview/`. State names below are the names to use in code.

### 3.1 Card

One scene, four sizes, rendered from a card instance (one or more parts).

| Size | Pixels | Used in | Shows |
|---|---|---|---|
| Token | 88 to 116 square | Battle boards | Type glyph, one value, golden mark, armor left, disabled stamp. Never interactive. |
| Small | 152x220 | Shop, hand | Art window 136x96; tier tab top right; cost coin top left (shop only); category emblem (category cards) on the art's lower left; bottom band with type seal (56 px) and the card's one value at 40 px. No name, no rules text. |
| Board | 176x254 | Board row, board overlays | The same at larger size (art 160x112), plus armor left on shields. |
| Enlarged | 420x608 | Inspect panel, reward, prompts | Top band with tier tab, cost coin and category emblem; art window 388x272; name at 48 px; rules text at 36 px in a 388x168 box; type line at 32 px. A combined card shows both names and both texts. |

"The card's one value" is the first number of its first effect as written in its text ("+1", "15%", "3", "+2 s"); cards with no number show only the seal. It is copied from the card's text, never computed.

| State | How it looks | When |
|---|---|---|
| `idle` | As above | default |
| `touched` | Lifts 6 px; caption strip shows its text | finger down |
| `selected` | Thick outline; inspect panel open | after a tap |
| `dragging` | Preview follows the finger, offset up and inward, tilted 4 degrees; the origin slot keeps a dotted outline | during a drag |
| `drop_target` | Board row, hand area or Sell zone shows a dashed outline and its label; all else dims to 50% | while a drag could land there |
| `offer` | Cost coin visible | in the shop |
| `unaffordable` | Lies flat, diagonal hatch, coin struck through | V6 says not usable |
| `reserved` | "Reserved" tag on the top edge | shop frozen |
| `extra` | Sits on a plinth after a divider, label "Extra" | potion, skull |
| `golden` | Night-edition face, gilt lines, laurel mark at the top centre | any part is golden |
| `combined` | Art window split on a diagonal, two type seals | two or more parts |
| `disabled` | Diagonal hatch over the whole card and a skull stamp | `disabled` is true |
| `armor` | Shield numeral in the bottom band; flashes when it drops | `armor_left` above 0 |
| `endgame_choice` | Laurel-and-hourglass mark on the top edge | chosen endgame effect |
| `golden_hint` | Laurel pip beside the cost coin | V19 |
| `reward_option` | Enlarged, with a "Take" button under it | reward overlay |
| `placeholder` | Flat tint by type family, type glyph large, card name printed in the art window | no art file yet |

### 3.2 Hero portrait

| Size | Used in |
|---|---|
| 88 px disc | Seat chips, answer-pick markers (48 px variant) |
| 112 to 120 px disc | Rail rows, battle plates, standings rows |
| 240 to 260 px square | Hero select tiles and detail |
| 552x736 splash | Battle intro, battle result, standings result panel |

| State | How it looks |
|---|---|
| `idle` | Portrait in a ring |
| `own` | Thick ring and a "You" tag |
| `taken` | Desaturated, hatched, "Taken" band (hero select) |
| `thinking` | Small quill moving on the ring |
| `locked_in` | Padlock on the ring |
| `correct` / `wrong` | Check or cross stamp on the ring for the reveal |
| `ghost` | 60% opacity, dashed ring, "Ghost of" label |
| `hit` | Shake and a damage numeral |
| `healed` | Plus icon and numeral rising |
| `eliminated` | Desaturated, placement numeral over it |
| `revived` | Colour returns from the centre outward |
| `winner` | Laurel over the ring |

### 3.3 Player-list row

200x150 in the shop rail; 1100x130 on the standings screen. Content: portrait, HP numeral, armor numeral with shield icon, tier tab.

| State | How it looks | Data |
|---|---|---|
| `idle` | As above | `players[]` |
| `own` | Thick outline, "You" tag | `you` |
| `next_opponent` | Shifted 16 px right, crossed-quills icon | V1 |
| `current_opponent` | Same mark, during battle and result | `battle.opponent` |
| `damaged` | HP numeral ticks down, row flashes, numeral with a minus sign | `DamageDealt` |
| `healed` | HP ticks up, plus icon | `Healed` |
| `tier_up` | Tier tab flips to the new numeral | `TavernUpgraded` |
| `streak` | Tally strokes, up to 5 | V2 |
| `eliminated` | Desaturated, placement numeral replaces HP, moves to the bottom | `PlayerEliminated` |
| `revived` | Returns to its sorted place | `PlayerRevived` |
| `pressed` | Detail panel open beside it | tap |

### 3.4 Timers

| Timer | Form | States |
|---|---|---|
| Phase timer (hero select, shop) | Numeral 96 px plus, in the shop, a 12 px bar between shop and board | `running`; `low` under 10 s (bar 24 px, pulse and tick each second); `critical` under 5 s (numeral pulses); `ended` |
| Question timer | 24 px bar across the centre column plus a 96 px numeral | `held` (full, still, during category and reveal); `running`; `low` under 3 s (bar 48 px, pulse, tick, optional haptic); `expired`; `locked` (keeps draining, greyed, after own lock-in) |

Low-time states never rely on a colour change alone: size, pulse and sound carry them. All timers count toward the host's deadline on the host clock (`Transport.now()`), and stop at zero without ever deciding anything.

### 3.5 Gold and tier indicators

| Component | Content | States |
|---|---|---|
| Gold counter | Coin icon, amount at 72 px | `idle`; `spend` (counts down, coin drops); `gain` (counts up, coin rises); `refused` (shakes when a command fails for gold; needs V10 to know the reason reliably) |
| Tier tab | Roman numeral I to VI on a bookmark shape | `idle`; `flip` on upgrade |
| Upgrade button | Current tier, arrow, next tier, cost | `available`; `unaffordable` (V6); `cheaper` (a down mark for one turn when the cost dropped since the last turn; a display comparison of two view values); `top_level` |
| Reroll button | Icon, cost | `available`; `unaffordable` (V6); `pressed` |
| Freeze button | Ribbon icon, "Reserve" or "Release" | `off`; `on` (pressed-in, ribbon tied) |
| Sell zone | "Sell" and the gold returned | `hidden`; `shown` during an owned-card drag; `hover` when the card is over it |

### 3.6 Answer button

552x250. Letter badge at the outer end, text at the inner end.

| State | How it looks | When |
|---|---|---|
| `blank` | Letter only, hatched, no response | category stage |
| `live` | Paper face, letter, text | question open |
| `pressed` | Sinks 4 px | finger down |
| `locked_mine` | Thick outline, own portrait marker at the outer end | own lock-in |
| `dimmed` | 50% opacity, no response | the other three after lock-in |
| `removed` | 30% opacity, strike line through the text, no response | help removed it |
| `correct` | Thick double outline, check icon over the badge, raised | reveal |
| `wrong_mine` | Cross icon over the badge, hatch | reveal, own wrong pick |
| `their_pick` | Opponent's portrait marker at the opponent-side end | reveal, V3 |
| `unanswered` | All four return to paper; the plate reads "No answer" | timeout without a pick |

Correct, own pick and opponent's pick are three separate marks (outline and check, own marker, their marker), so all three can be read without colour.

### 3.7 Help button

168x200. Icon, name, uses left.

| State | How it looks |
|---|---|
| `ready` | Raised, uses numeral |
| `waiting` | Flat (question not open, or already locked in) |
| `used_now` | Brief flash; numeral ticks down; stays `ready` if uses remain |
| `spent` | Flat, hatched, "0" |
| `refused` | Shake and a toast |

### 3.8 Smaller parts

| Component | Size | States |
|---|---|---|
| Category chip | Emblem 72 px plus name; 240 px emblem in the category stage | one per category; `house`, `yours`, `theirs` tag (V17) |
| Status chip | 64 px tall pill in the caption strip | `frozen`, `reward` (pulsing), `endgame_effect`, `overtime` (V15) |
| Toast | Replaces the caption for 2 s | `notice`, `refusal` (with a cross icon) |
| Ledger cell | 72 px (battle), 96 px (result) | `pending`, `current`, `right` (check), `wrong` (cross), `unknown` (after reconnect, until V5) |
| Stake numeral | 128 px | `idle`, `step_up`, `strike`, `no_damage` |
| Hero plate | 240x204 | `passive`, `ready`, `locked` (padlock and reason), `targeting` (banner and Cancel), `holding` (fill ring), `spent` |
| Modal frame | Dim layer plus panel; close button 116x116 | `open`, `hidden` (collapsed to a chip, for reward and house picker) |

## 4. Visual direction

### 4.1 The idea: an engraved reference book read by lamplight

Owl's heroes are people known from portraits in old books: Herodotus from a bust, Darwin and Newton from engravings, Shakespeare from a frontispiece. The direction takes its look from that source. Screens are dark ink, as a desk at night. Cards and panels are paper plates, as pages cut from an encyclopedia: an illustration window drawn in line and hatch, a ruled border, a caption. Accents are the three things found on a scholar's desk: gilt (the edge of a book), verdigris (aged brass and copper instruments) and vermilion (the red ink of a correction).

Reasons for this choice:

- It fits the theme without props from fantasy games. There is no tavern, no wood and stone, no bartender, no magical glow.
- Line engraving with flat tints is cheap to stand in for. A placeholder is a flat tint with a glyph, and it already looks like part of the same world.
- Hatching, the engraver's way of shading, doubles as the non-colour signal for "disabled", "unavailable" and "wrong", which the accessibility rule requires anyway.
- Dark screens with light cards put the highest contrast on the objects the player acts on.

### 4.2 Palette

Contrast ratios are approximate, worked by hand from the sRGB luminance formula. Check them again with a contrast tool before the theme is frozen.

| Name | Hex | Role | Contrast |
|---|---|---|---|
| Ink | `#141B2D` | Screen background | base |
| Ink raised | `#1F2A44` | Panels, plates, rail rows on the background | decorative step from Ink |
| Paper | `#F2E9D8` | Card faces, answer buttons, question panel; all text on dark | about 14:1 on Ink |
| Paper shade | `#D9CDB4` | Card bands, pressed buttons, inner rules | text on it stays Ink |
| Slate | `#9AA3B8` | Secondary text on dark, idle outlines | about 6.8:1 on Ink |
| Gilt | `#E0A526` | Gold amounts, golden cards, own-row outline, laurels | about 7.8:1 on Ink; about 1.8:1 on Paper, so never text or thin lines on Paper |
| Verdigris | `#2FA58C` | Correct, heal, available, timer bar | about 5.6:1 on Ink |
| Verdigris deep | `#17695A` | The same meanings on Paper | about 5.4:1 on Paper |
| Vermilion | `#E4572E` | Wrong, damage, low time, refusal | about 4.7:1 on Ink |
| Vermilion deep | `#B23A16` | The same meanings on Paper | about 5:1 on Paper |

Rules of use:

- Body text is Paper on Ink or Ink on Paper only. Both give about 14:1.
- Verdigris and Vermilion are close in lightness, so a player who cannot tell red from green is never asked to: correct always carries a check icon and a thick outline, wrong always carries a cross icon and a hatch, low time always carries size, pulse and sound. The pair was picked as a blue-green against an orange-red, which most colour-vision deficiencies still separate, but that has not been checked in a simulator and the design does not depend on it.
- The two players in a battle are told apart by side (own left, opponent right) and portrait, never by colour.
- Gilt means money and golden cards and nothing else.

### 4.3 Typography

Two families, both to be chosen from open-licence fonts and bundled; the licences must be confirmed by the owner before shipping.

| Use | Style | Why | Candidates (licence unverified, from general knowledge) |
|---|---|---|---|
| Names, titles, verdicts, big numerals | An old-style book serif with sturdy strokes and lining figures, set in sentence case | Carries the reference-book voice; numerals at 96 to 128 px look like page numbers and plate numbers | EB Garamond, Fraunces, Cormorant |
| Questions, answers, rules text, labels | A humanist sans with open shapes and clearly different l, I and 1 | Read under a 9-second timer at 48 to 60 px; the accessibility guideline in the notes asks for a sans option and sentence case | Atkinson Hyperlegible, Source Sans 3 |

- Timer and HP numerals use tabular figures so digits do not jitter as they change.
- No all-caps lines of text; small caps are allowed for one-word labels ("EXTRA", "YOU").
- Sizes follow section 2.0: 32 smallest, 48 body, 60 question, 64 or more glanceable, 96 to 128 numerals.
- Both reference games use distinctive display faces of their own (unverified, from general knowledge). A book serif paired with a plain sans is far from either.

### 4.4 Shape language and how things are signalled

- **Cards are plates.** Upright rectangles with a small corner radius (12 px at small size), a double ruled border, a rectangular art window. No ovals, gems or metal frames.
- **Buttons are stamps and tabs.** Rectangles with one clipped corner. A raised button casts a short hard shadow; a flat one has none. "Raised" means available and is the genre's "can act" outline done by depth, not colour.
- **Panels are sheets.** Paper for things read (question, enlarged card), Ink raised for things that frame (plates, rail).
- **Hatch means no.** A diagonal engraved hatch covers anything disabled, unavailable, removed or wrong.

| What | Signal | Colour-independent because |
|---|---|---|
| Tier (1 to 6) | A Roman numeral, I to VI, on a bookmark tab at the card's top right; the same tab shows a player's own tier | It is a numeral |
| Golden | The card face inverts to Ink with Gilt line work (a "night edition"), and a laurel mark sits on its top edge. A golden category card's laurel carries "x3". | Light card versus dark card is a lightness change, and the laurel is a shape |
| Card type (16 kinds) | A seal in the bottom band: its outline shape gives the family, the glyph inside gives the kind (table below) | Shape and glyph |
| Category (10) | An emblem, with the name beside it wherever there is room | Each emblem is a distinct silhouette |
| Extra offer | Stands on a plinth after a divider, labelled "Extra" | Position and label |
| Reserved (frozen) shop | A ribbon across the row and a tag per card | Shape and word |
| Disabled card | Hatch and a skull stamp | Pattern and shape |
| Chosen endgame effect | Laurel-and-hourglass mark | Shape |

Card type families:

| Family | Seal shape | Tint | Kinds and glyphs |
|---|---|---|---|
| Knowledge | Circle | Verdigris | `category` (open book), `weights` (balance weights) |
| Attack | Pennant (pointed down) | Vermilion | `damage` (quill as a blade), `bet` (two dice), `counter` (broken ring) |
| Guard | Shield | Slate | `armor` (plain shield), `defense` (wall), `reflect` (hand mirror) |
| Tempo | Diamond | Paper shade | `time` (hourglass), `tie` (scales) |
| Fortune | Rounded square | Gilt | `economy` (coin purse), `healing` (mortar and pestle) |
| Rare | Eight-point star | Ink with Gilt line | `help` (raised hand), `endgame` (laurel and hourglass), `legendary` (lamp), `token` (tag) |

Category emblems and tints. The tints fill the emblem and a thin stripe on category cards; they are never used for text. Several pairs (green, ochre and coral; indigo and plum) will look alike to some players, which is why the silhouette, not the tint, is the identifier.

| Category | Emblem | Tint |
|---|---|---|
| History | Column | Ochre `#C9973B` |
| Flora and Fauna | Leaf | Leaf green `#6FA85A` |
| Cars and Mechanics | Gear | Steel `#8C9BB0` |
| Film | Film reel | Plum `#9A6BB5` |
| Physics | Pendulum | Sky `#5AA9D6` |
| Philosophy | Oil lamp | Indigo `#6C78D8` |
| Theater | Mask | Rose `#D86C7E` |
| Anatomy | Heart, anatomical | Coral `#E08A6A` |
| Geography | Compass rose | Sea `#3FA7A0` |
| Sport | Discus | Lime `#C9C94A` |

Vocabulary is part of identity too. The rules call the shop level the "tavern level"; on screen that word would bring the reference game's theme with it. Open question 7 proposes "library" terms.

## 5. Asset list

`client/assets/` does not exist yet and no naming scheme has been agreed. This section proposes one.

**Convention.**

- Folders by type under `client/assets/`: `cards/art/`, `cards/frames/`, `heroes/portrait/`, `heroes/splash/`, `icons/category/`, `icons/kind/`, `icons/ui/`, `ui/`, `backgrounds/`, `fonts/`, `sfx/`, `music/`.
- File names in `lower_snake_case`. Where a data id exists the file name is that id: card ids (`whetstone`), hero ids (`benz`), category slugs (`flora_and_fauna`), kind values (`reflect`).
- Images are PNG with alpha. Icons are a single white shape on transparent, tinted by the engine.
- **Scale: art is authored at twice its largest displayed size in ref px.** The reference is 1080 high and large phones have more physical pixels than that on the short side, and enlarged cards are zoomed in animations. Placeholders are generated at the authored size so nothing moves when art replaces them.
- Sound effects are WAV, 44.1 kHz, 16-bit, mono unless noted; music is Ogg Vorbis, stereo, loopable. (Common Godot practice; check the import documentation.)

### 5.1 Illustrations

| Asset | Count | File | Authored size | Displayed at |
|---|---|---|---|---|
| Card illustration | 36 | `cards/art/<card_id>.png` | 776x544 | 388x272 enlarged, 160x112 board, 136x96 small |
| Hero portrait | 8 | `heroes/portrait/<hero_id>.png` | 1024x1024 | up to 260x260; cropped to a disc at 88 to 120 |
| Hero splash | 8 | `heroes/splash/<hero_id>.png` | 1104x1472 | 552x736 on the battle intro, result and standings |
| Screen background | 5 | `backgrounds/hero_select.png`, `shop.png`, `battle.png`, `endgame.png`, `results.png` | 2520x1440, with nothing important outside the central 1824x950 | full screen, cropped per aspect |
| Logo | 1 | `ui/logo_owl.png` | 1024x512 | title and loading |

Card ids (36 illustrations): `apprentice`, `scholar`, `master` (one each; the category emblem is laid over by the engine, see open question 16), `hourglass`, `whetstone`, `wooden_shield`, `coin_purse`, `consolation`, `mirror_shard`, `scales_of_judgment`, `iron_shield`, `last_laugh`, `long_hourglass`, `second_opinion`, `thorned_mirror`, `double_or_nothing`, `piercing_quill`, `rivals_notes`, `siege_ram`, `streak_banner`, `deep_breath`, `hall_of_mirrors`, `narrowed_choice`, `perfect_mind`, `steel_bulwark`, `aegis`, `encyclopedia`, `healer`, `hunter`, `oracle`, `second_wind`, `arrow`, `buckler`, `tower_shield`, `potion`, `skull`.

Hero ids: `benz`, `darwin`, `herodotus`, `herophilus`, `lumiere`, `newton`, `nietzsche`, `shakespeare`.

Golden cards need no second illustration: the night-edition look is a shader over the same art.

### 5.2 Card and panel surfaces

These can be drawn with `StyleBoxFlat` and shaders until art exists.

| Asset | File | Authored size | Note |
|---|---|---|---|
| Small and board card frame | `cards/frames/card_small.png` | 352x508 | Used at 176x254 and 152x220 |
| Enlarged card frame | `cards/frames/card_large.png` | 840x1216 | Used at 420x608 |
| Golden variants | `cards/frames/card_small_golden.png`, `card_large_golden.png` | same | Night edition |
| Card back | `cards/frames/card_back.png` | 352x508 | Shop reroll flip, reward deal |
| Token frame | `cards/frames/token.png`, `token_golden.png` | 232x232 | Used at 88 to 116 |
| Tier tab | `cards/frames/tab_tier.png` | 96x128 | Numeral drawn by the engine |
| Cost coin | `cards/frames/coin_cost.png` | 128x128 | Numeral drawn by the engine |
| Hatch pattern | `ui/hatch.png` | 128x128, tileable | Disabled, removed, wrong |
| Paper panel | `ui/panel_paper.png` | 192x192 nine-patch | Question panel, enlarged text box |
| Ink panel | `ui/panel_ink.png` | 192x192 nine-patch | Plates, rail rows, controls |
| Answer button face | `ui/button_answer.png` | 256x256 nine-patch | Stretched to 552x250 |
| Control button face | `ui/button_control.png` | 256x256 nine-patch | Raised and flat variants by shader |
| Reserve ribbon | `ui/ribbon_reserved.png` | 512x96, tileable in x | Across the shop row |
| Plinth | `ui/plinth_extra.png` | 352x96 | Under extra offers |

### 5.3 Icons

All `256x256` white on transparent unless noted; category emblems are `512x512` because they are shown at 240 px.

| Group | Files |
|---|---|
| Categories (10), `icons/category/` | `history`, `flora_and_fauna`, `cars_and_mechanics`, `film`, `physics`, `philosophy`, `theater`, `anatomy`, `geography`, `sport` |
| Card kinds (16), `icons/kind/` | `category`, `armor`, `time`, `damage`, `reflect`, `tie`, `economy`, `defense`, `help`, `bet`, `counter`, `weights`, `endgame`, `legendary`, `healing`, `token` |
| Seal shapes (6), `icons/kind/` | `seal_knowledge`, `seal_attack`, `seal_guard`, `seal_tempo`, `seal_fortune`, `seal_rare` |
| Economy and stats, `icons/ui/` | `gold`, `hp`, `armor`, `tier`, `timer`, `streak`, `turn` |
| Shop actions, `icons/ui/` | `upgrade`, `reroll`, `reserve`, `sell`, `buy`, `place`, `use` |
| Battle, `icons/ui/` | `check`, `cross`, `padlock`, `thinking`, `ghost`, `next_opponent`, `no_answer`, `house`, `overtime` |
| Helps, `icons/ui/` | `help_second_opinion`, `help_flaming_sword`, `help_oracle` |
| Hero abilities, `icons/ui/` | `ability_rewrite`, `ability_evolve`, `ability_combine`, `ability_record`, `ability_ubermensch`, `ability_to_be`, `ability_passive` |
| Damage sources, `icons/ui/` | `source_battle`, `source_tie_breaker`, `source_zero_zero`, `source_overtime`, `source_reflect`, `source_arrow`, `source_ability`, `source_endgame_step` |
| Marks, `icons/ui/` | `laurel`, `laurel_hourglass`, `skull_stamp`, `eliminated`, `revived`, `victory`, `defeat`, `tie` |
| System, `icons/ui/` | `close`, `hide`, `show`, `info`, `settings`, `cancel` |

### 5.4 Fonts

| File | Use |
|---|---|
| `fonts/display_regular.ttf`, `fonts/display_bold.ttf` | Book serif for names, titles, numerals |
| `fonts/text_regular.ttf`, `fonts/text_bold.ttf` | Humanist sans for questions, answers, rules text |

Until fonts are chosen the engine's default font stands in at the sizes of section 2.0.

### 5.5 Sound

| Group | Files in `sfx/` |
|---|---|
| Touch | `ui_tap`, `ui_refuse`, `card_pick_up`, `card_drop`, `modal_open`, `modal_close` |
| Shop | `card_buy`, `card_sell`, `card_place`, `card_use_potion`, `card_gain`, `shop_reroll`, `shop_reserve_on`, `shop_reserve_off`, `tier_up`, `gold_gain`, `gold_spend`, `golden_merge`, `golden_land`, `reward_open`, `reward_take`, `cards_combine`, `ability_use`, `heal` |
| Timers | `timer_tick`, `timer_tick_urgent`, `phase_shop_start`, `phase_shop_end` |
| Battle | `battle_intro`, `card_disabled`, `category_reveal`, `question_open`, `answer_lock`, `opponent_lock`, `help_second_opinion`, `help_flaming_sword`, `help_oracle`, `option_removed`, `reveal_correct`, `reveal_wrong` |
| Result | `verdict_win`, `verdict_lose`, `verdict_tie`, `damage_hit`, `armor_absorb`, `shield_break`, `damage_reflect`, `damage_arrow`, `player_eliminated`, `player_revived` |
| Endgame and match end | `endgame_intro`, `stake_step`, `stake_no_damage`, `shared_victory`, `match_win`, `match_end` |

| Music file in `music/` | Use | Length |
|---|---|---|
| `hero_select.ogg` | Hero select | loop, 60 to 90 s |
| `shop.ogg` | Shop phase | loop, 60 to 90 s |
| `battle.ogg` | Regular battle | loop, 60 to 90 s |
| `endgame.ogg` | Endgame | loop, 60 to 90 s |
| `results.ogg` | Standings at match end | 15 to 30 s |

## 6. Animation list

The owner authors these. Each is a separate scene under `client/scenes/preview/` that runs on its own with test data, with durations, curves and colours as `@export` parameters, checked by capturing frames with `--write-movie`.

**How animations relate to game time.** The state in `Store` is already final when an event arrives. The Director decides only what the player sees and when. It has two lanes:

- **Stage lane.** Events that change what the whole screen is doing, played in order. They are scheduled against host-clock times (`opens_at`, `deadline`), not against the end of the previous animation. Every stage animation takes a time budget and compresses to fit it; if its start time has already passed, it jumps to its end state.
- **Overlay lane.** Small effects on one card, row or counter. They start at once, never delay anything, never block input, and are dropped if the node they belong to has gone.

**The fixed pauses the stage lane must fit inside** (`data/tunables.toml`):

| Moment | Host pause | How it is split |
|---|---|---|
| `BattleStarted` to the first question opening | 2000 ms | Intro 700, category 1300 |
| `AnswerRevealed` to the next question opening | 4500 ms | Reveal 2500, category 2000 |
| `EndgameStarted` to the first endgame question | 2000 ms | Intro 1200, category 800 |
| Endgame, between questions | 4500 ms | Answer 900, damage 1200, hold 400; then stake step-up 500 inside the 2000 ms category stage |
| After the last question of a battle | none | The reveal is client time: 2500 ms, compressed to no less than 1200 ms if the turn has already resolved |
| Shop phase | 30 s, shared | Nothing blocks; every shop animation is an overlay |

The two intros are tight. Open question 5 proposes host pauses for them.

### 6.1 One row per event in `crates/core/src/api.rs`

All 29 variants of `Event`. "Seen by" is the audience the lobby filter gives the event.

| Event | Seen by | Animation | Duration | Lane | Preview scene |
|---|---|---|---|---|---|
| `HeroPicked` | everyone | The hero's tile is stamped "Taken" and its seat chip fills with the portrait. Own pick: tile lifts and gets a laurel. | 350 ms | overlay | `preview_hero_picked` |
| `TurnStarted` | everyone | "Turn N" title sweeps across the caption strip; shop cards deal in left to right, 60 ms apart; timer bar fills. The shop accepts input from the first frame. | 600 ms | stage, non-blocking | `preview_turn_started` |
| `CardBought` | the buyer | The card flies from its shop slot to the hand; a coin drops from the gold counter, which counts down. | 250 ms | overlay | `preview_card_bought` |
| `CardSold` | the seller | The card slides into the Sell zone and dissolves to ink; a coin rises to the gold counter. | 250 ms | overlay | `preview_card_sold` |
| `CardPlaced` | the owner | The card drops into its board slot and settles with a small bounce. | 200 ms | overlay | `preview_card_placed` |
| `CardUsed` | the owner | The card lifts from the hand and is consumed (a potion is poured). `Healed` follows. | 400 ms | overlay | `preview_card_used` |
| `CardGained` | the owner | A card inks itself in at the end of the hand. Used for reward picks, Consolation's Buckler, skulls and the Arrow. | 300 ms | overlay | `preview_card_gained` |
| `ShopRolled` | the roller | Shop cards turn over to their backs and new ones turn up, 40 ms apart. | 300 ms | overlay | `preview_shop_rolled` |
| `TavernUpgraded` | everyone | Own: the tier tab flips to the new numeral and the shop row widens for the extra slot. Others: the tier tab on their rail row flips. | 500 ms own, 300 ms others | overlay | `preview_tier_up` |
| `GoldenMade` | the owner | The copies slide together from hand and board, flash, and one night-edition card lands with a laurel. The reward overlay waits for this to finish. | 900 ms (400 gather, 150 flash, 350 land) | overlay; holds back `RewardOffered` only | `preview_golden_made` |
| `CardsCombined` | the owner | Two cards slide together and are bound into one with a split art window. | 500 ms | overlay | `preview_cards_combined` |
| `RewardOffered` | the owner | The reward chip appears and the overlay opens: three enlarged cards deal in from their backs. | 350 ms | overlay | `preview_reward_offered` |
| `Healed` | everyone | The HP numeral counts up and a plus sign with the amount rises from the portrait. | 500 ms | overlay | `preview_healed` |
| `AbilityUsed` | the user | The hero plate flashes and plays that hero's flourish (a quill stroke for Rewrite, a film frame for Record, and so on). The flourish is chosen by hero id; it is decoration, not a rule. | 400 ms | overlay | `preview_ability_used` |
| `HouseOptionsShown` | Nietzsche's player | The house picker opens and four category tiles deal in. | 300 ms | overlay | `preview_house_options` |
| `BattleStarted` | the two fighters | Shop slides away; the versus card brings both splashes in from the sides with names; a ghost is translucent and labelled. Ends on the battle screen with plates in place. | 700 ms | stage | `preview_battle_started` |
| `CardDisabled` | the two fighters (everyone in the endgame) | A skull stamp lands on the token and the hatch draws across it. Plays during the intro. | 300 ms | overlay | `preview_card_disabled` |
| `CategoryRevealed` | the two fighters (everyone in the endgame) | Arrives with `QuestionShown`: the category and its difficulty show above the question as it appears, with the reveal sweep. No screen of its own. | with the question | stage | `preview_category_revealed` |
| `QuestionShown` | the two fighters (everyone in the endgame) | At `opens_at`: question text and answers appear, inactive, to be read. At `answers_at` they light up and the timer bar starts. | 150 ms | stage, non-blocking | `preview_question_shown` |
| `OptionsRemoved` | the help's user | The removed answers are struck through and fade to 30%; the help button's count ticks down. Flaming Sword cuts two at once. | 200 ms | overlay | `preview_options_removed` |
| `AnswerRevealed` | the two fighters (everyone in the endgame) | Correct answer gets its outline and check and rises; a wrong own pick gets its cross and a short shake; both plates are stamped; the ledger cell fills; the score ticks. Holds for the rest of the reveal. | 500 ms, then hold to 1500 | stage | `preview_answer_revealed` |
| `BattleScored` | the two fighters | The battle screen gives way to the result screen; the verdict word and score land. | 600 ms | stage | `preview_battle_scored` |
| `DamageDealt` | everyone | On the result or endgame screen, for a fighter: a numeral travels to the target, armor cracks for the absorbed part, then the HP numeral drops. On the rail, for anyone else: the row flashes and HP ticks down. One variation per source, below. | 500 ms each, 300 ms on the rail | stage on the result and endgame screens; overlay on the rail | `preview_damage_dealt` |
| `ShieldBroken` | the two fighters (everyone in the endgame) | The shield card cracks and falls from its slot; the slot empties. | 400 ms | overlay | `preview_shield_broken` |
| `PlayerRevived` | everyone | Colour returns to the portrait from the centre, HP counts up to its new value, the row returns to its place. | 800 ms | overlay | `preview_player_revived` |
| `PlayerEliminated` | everyone | The row desaturates, the placement numeral is stamped on it and it sinks to the bottom. For the player themself: a full-screen "You placed Nth" card, then the standings. | 500 ms row; 1200 ms own card | overlay for others; stage for own | `preview_player_eliminated` |
| `EndgameStarted` | everyone | "Final duel" title card with both splashes and the HP and armor they carry in; ends on the endgame screen. | 1200 ms | stage | `preview_endgame_started` |
| `SharedVictory` | everyone | Both finalists' portraits receive laurels side by side. | 1500 ms | stage | `preview_shared_victory` |
| `MatchFinished` | everyone | The winner's splash with a laurel and name; the results screen is already beneath and can be used at once. | 1500 ms | stage, non-blocking | `preview_match_finished` |

`DamageDealt` variations by `DamageSource`:

| Source | Variation |
|---|---|
| `Battle` | The numeral leaves the winner's plate and strikes the loser's |
| `TieBreaker` | A pair of scales tips toward the target before the hit |
| `ZeroZero` | A single small hit with a mask icon |
| `Overtime` | Both plates are struck at the same moment from the centre |
| `Reflect` | The hit rebounds from the target's plate back along its path |
| `Arrow` | An arrow crosses the screen ahead of any other damage |
| `Ability` | A quill stroke rewrites the score, then the hit lands |
| `EndgameStep` | The stake numeral itself drops onto the target's HP |

### 6.2 Animations with no event

These are driven by input or by a change in the snapshot.

| Trigger | Animation | Duration | Preview scene |
|---|---|---|---|
| Finger down on a card | Lift 6 px; caption appears | 80 ms | `preview_card_touch` |
| Drag start and end | Lift, tilt, drop targets appear and others dim; on a miss the card returns to its slot | 120 ms start, 180 ms return | `preview_card_drag` |
| A command is refused | The control shakes; toast in the caption strip | 250 ms shake, 2 s toast | `preview_refusal` |
| `me.frozen` changes | Ribbon ties across the shop row, or unties | 300 ms | `preview_reserve` |
| Own answer tapped | Button sinks and takes its outline; the others dim | 120 ms | `preview_answer_lock` |
| `battle.opponent_locked` becomes true | Padlock closes on the opponent's plate | 200 ms | `preview_opponent_lock` |
| Timer enters `low` | Bar thickens; pulse each second | 150 ms per pulse | `preview_timer_low` |
| Endgame step rises | Old stake struck out, new one stamps in | 500 ms | `preview_stake_step` |
| Modal opens or hides | Scale and fade from its chip or card | 200 ms | `preview_modal` |
| Screen change with no event (standings) | Cross-fade | 250 ms | `preview_screen_fade` |
| Hold to confirm (Rewrite, To be or not to be) | Ring fills around the plate; releasing early empties it | 600 ms | `preview_hold_confirm` |
| Golden or legendary card idle | Slow gilt sheen across the line work (shader) | 3 s loop | `preview_card_sheen` |

## 7. Build order

The existing portrait screens (`hero_select.gd`, `shop.gd`, `battle.gd`, `standings.gd`, the helpers in `ui.gd`, and the screen swapping in `main.gd`) are **replaced, not adapted**. Their sizes, single-column layout and clear-and-rebuild refresh do not carry over. `Transport`, `Store` and the command and event plumbing are kept.

Everything in steps 1 to 7 can be built before any art arrives. Placeholders are generated at the authored sizes of section 5, so art drops in by file name with no layout change.

| Step | Work | Notes |
|---|---|---|
| 1. Landscape shell | Project settings: viewport 1920x1080; `stretch/mode = canvas_items`; `stretch/aspect = expand`; `window/handheld/orientation = 4` (sensor landscape); desktop window override 960x540; `gui/common/drag_threshold = 40`. Update `play.bat`'s `--resolution`. Root scene with a full-rect background and the `SafeFrame` margin container of section 2.0. A debug flag that fakes insets (175 left and right, 75 bottom, 54 top) and draws the unsafe regions. | Test shapes by window size: 960x540 (16:9), 1170x540 (19.5:9), 1200x540 (20:9), 1260x540 (21:9). Settings and the inset conversion are from documentation, untested on a device. Leave `emulate_mouse_from_touch` on or drags will not start from touch. |
| 2. Theme and placeholders | One `Theme` resource with the palette, the two font roles and the sizes of section 2.0; style boxes for paper, ink, raised, flat, hatched. A placeholder generator that writes tinted PNGs with a glyph and the asset's name at each authored size. | No art needed. |
| 3. Components | Card (four sizes, all states), hero portrait, player-list row, timers, gold and tier indicators, answer button, help button, chips, toast, modal frame. Each with a preview scene and exported parameters. | Build the card first; five screens use it. |
| 4. Director | The two-lane event queue of section 6, with host-clock scheduling and compression. Screens become persistent node trees updated from `Store`. | Start with instant stand-ins for every animation so the timing can be tested before any animation exists. |
| 5. Battle question, then result, then endgame | In that order. The question screen is the most original and the least certain, so it goes on a real phone first (the device spike is already step 3 of the project's build order). Result and endgame reuse its plates, tokens and ledger. | Extend the existing `--shots` switch with labels for the new screens. |
| 6. Shop | First with taps only (inspect panel and its buttons), which reaches parity with today's client. Then drags: buy, place, sell. Then the overlays: reward, hero ability pickers, player detail, endgame-effect button. | The temporary action adapter lives in one script (open question 1). |
| 7. Hero select and standings | The two simplest screens. | Hero select needs the temporary ability text (V7). |
| 8. View additions | In `core` and `lobby`, each with a test, in this order of value to the interface: V6 (usable, costs, actions), V7 and V8 (text), V3 and V5 (reveal and ledger), V4 (damage breakdown), V1 and V2 (rail), then the rest. | Can run in parallel with steps 3 to 7. The private-view change of open question 2 belongs here. `docs/protocol.md` is written when the server step begins. |
| 9. Animations and sound | The owner's scenes replace the stand-ins one by one, in the order of section 6. Sound hooks sit on the same Director entries. | Each is checked from captured frames; the final call on looks is the owner's. |
| 10. Art | Illustrations, portraits, splashes, icons and fonts replace placeholders by file name. | No code change expected. |

Safe-area conversion for step 1 (derived from the documented behaviour of `get_display_safe_area()`; untested on a device; skip on desktop, where the function returns the desktop's usable area):

```gdscript
func _apply_safe_area() -> void:
    var safe: Rect2i = DisplayServer.get_display_safe_area()
    var win: Vector2i = DisplayServer.window_get_size()
    var vp: Vector2 = get_viewport().get_visible_rect().size
    var sx := vp.x / win.x
    var sy := vp.y / win.y
    var left := ceili(safe.position.x * sx)
    var right := ceili((win.x - safe.end.x) * sx)
    var side := maxi(48, maxi(left, right))
    frame.add_theme_constant_override("margin_left", side)
    frame.add_theme_constant_override("margin_right", side)
    frame.add_theme_constant_override("margin_top", maxi(54, ceili(safe.position.y * sy)))
    frame.add_theme_constant_override("margin_bottom", maxi(76, ceili((win.y - safe.end.y) * sy)))
```

Call it on ready, on the root window's `size_changed`, and on `DisplayServer.orientation_changed`.

## 8. Open questions for the owner

| # | Question | Recommendation |
|---|---|---|
| 1 | The shop screen today decides which buttons to show from effect names (`HealOnce`, `EndgameFewerOptions`, `EndgameTime`, `EndgameShield`) and from `hero.kind`. That is rule knowledge in the client, against the project's working rules. | Add V6 to the view: for each card and ability, the list of actions with `usable`, cost, charges and a reason code. Until then keep the checks in one adapter script with a comment that it must be deleted, and nowhere else. |
| 2 | The snapshot's `me` is the raw core `Player` struct. It carries correct-answer indices: the last battle's questions, and Lumiere's recorded question, which will be asked again. Harmless offline, a leak once a real opponent exists. | Before the server step, replace `me` with a purpose-built private view struct that lists only what the interface uses. Strip `correct` from the recorded question at the least. Add a test that no view contains a correct index for a question that can still be asked. |
| 3 | There is no command to reorder the board or to move a board card back to the hand, and `Place` takes no slot. Effects resolve left to right across the board, so order can matter. | Leave it out until a card makes order matter to the player. If it does, add a slot to `Place` and a `Move` command; the board row already has room for drag-to-reorder. This is a rule decision. |
| 4 | Should the question text be visible before the timer starts? The rules say category, then the question, then the timer. Today text and options appear when the timer starts. | Show the question text, without the options, for the last 800 ms of the category pause. It rewards reading and costs nobody anything, since both players get the same. It changes what the host sends when, so decide it as a rule. |
| 5 | Three moments have no host pause and take time from the player: the battle intro (shares the first 2000 ms category pause), the endgame intro (the same), and the result and damage after the last battle of a turn (the shop timer is already running, and the final reveal has no pause at all). | Add three tunables: a battle-intro pause (about 1500 ms), an endgame-intro pause (about 2500 ms), and a resolve pause after the last battle (the 2500 ms reveal plus about 3000 ms for verdict and damage) before `TurnStarted`. Report the effect on match length from `sim`. |
| 6 | Hero select: one tap to pick (today) or select then "Lock in"? And is 30 s enough to read eight abilities? | Select then lock in. The pick is permanent and the abilities are long. Raise `hero_select_seconds` to 45 and watch whether players use it. |
| 7 | On-screen vocabulary. "Tavern level", and the bartender it implies, belong to the reference game. | Call the shop the Library and the level the "library level"; Reroll becomes "Restock", Freeze becomes "Reserve". Keep the data names as they are. |
| 8 | Left-handed players. The study behind the layout tested right-handed people only. | A setting that mirrors the shop (rail right, controls left) and moves the helps to the left wing. The answer buttons are already symmetric. |
| 9 | The wings answer layout is untested, and one-handed play has to reach across the screen. | Put it on a phone in step 5 and time real answers. Keep the 2x2 layout (alternative A, without the boards) buildable behind a debug switch for comparison. Consider a "stacked on one side" option later for one-handed play. |
| 10 | Endgame effect: what happens if a player holds several and chooses none, and when is the player asked? | Have the host pick the first one held, and expose V13 so the client can prompt in the last shop before the endgame. A rule decision. |
| 11 | Should eliminated players watch the endgame? They already receive its events but have no view of it. | Yes, as a watch-only form of the endgame screen (V14). Until then the standings show the finalists' HP live. |
| 12 | Dropping a shop card straight onto the board: buy only, or buy and place? | Buy only for now. Buy-and-place needs two commands and must handle the bought card merging into a golden one on arrival. Revisit once V6 exists. |
| 13 | Length limits for the question bank. The question panel holds about 90 characters at full size and an answer button about 48. | Give writers a guideline: questions up to 90 characters (140 at most), answers up to 30 (48 at most). The reading-speed figure supports about 20 words in total for a 9 s window. |
| 14 | Is the next opponent known during the shop? Pairing happens after the shop closes. | If the rotation can be fixed at the start of the turn, expose it (V1); the marker is one of the most used pieces of the genre's player list. If elimination or a ghost can change it, show it as "likely". A rule decision. |
| 15 | Should the golden reward open by itself or wait as a chip? | Open by itself when no drag is in progress, with the Hide button. A reward that waits unseen will often be lost to the random pick. |
| 16 | Category cards: 3 illustrations with the category emblem laid over (as assumed in section 5), or 30 illustrations? | Three. The emblem is the part players need to read. |
| 17 | Text size and haptics as settings. | Offer a text-size setting through the stretch scale later; add a short haptic on low time and on lock-in once on devices. Neither blocks the prototype. |
| 18 | Is a 30 s shop enough on a touch screen with drags? The reference game gives 60 to 120 s for a larger board. | Keep 30 s, measure how often players run out, and tune `shop_seconds` from that. |
| 19 | Tablets and unfolded foldables get extra height under `expand`. | Cap the content frame at 950 high and centre it. Do not design for tablets in the prototype. |
| 20 | Player names. The view has none; the client shows hero names. | Hero names are enough offline. Add display names with accounts (V12). |
| 21 | `SubmitAnswer` accepts an explicit pass, and the client never sends it. | Leave it without a button. A pass and a timeout score the same, and a pass button near the answers invites mis-taps. |

## Sources

Reference games

- Rdu Hearthstone, "Playing BG on Mobile is SO Weird! | F2P To 10k EP 14", 27 May 2026 (six phone frames viewed): https://www.youtube.com/watch?v=oEKyHSBcmzk
- Hearthstone wiki, Battlegrounds (describes about patch 29.2): https://hearthstone.wiki.gg/wiki/Battlegrounds
- Pocket Tactics, Battlegrounds on mobile (May 2022): https://www.pockettactics.com/hearthstone/battlegrounds-mobile
- Icy Veins, Battlegrounds mechanics guide (search summary only): https://www.icy-veins.com/hearthstone/hearthstone-battlegrounds-mechanics-guide
- Hearthstone Fandom wiki, Battlegrounds patch changes (search summary only): https://hearthstone.fandom.com/wiki/Battlegrounds/Patch_changes
- Blizzard forum threads (search summaries only): https://us.forums.blizzard.com/en/hearthstone/t/104774/1 and https://us.forums.blizzard.com/en/hearthstone/t/fix-the-timer-to-start-when-you-enter-the-tavern/83164
- Blizzard, patch 35.6 notes (search summary only): https://playhearthstone.com/en-us/blog/24276665/
- Riot Games, "Teamfight Tactics Mobile Update" (2020, read through the VNG mirror): https://dtcl.vnggames.com/en-us/news/riot-games/teamfight-tactics-mobile-update
- Riot Games, "TFT Interface Update" (10 December 2019): https://teamfighttactics.leagueoflegends.com/en-gb/news/game-updates/tft-interface-update/
- Riot Games, TFT patch 12.4 notes: https://teamfighttactics.leagueoflegends.com/en-au/news/game-updates/teamfight-tactics-patch-12-4-notes
- Upcomer, TFT Mobile first impressions (20 March 2020): https://upcomer.com/teamfight-tactics-mobile-first-impressions/
- devtrackers.gg, TFT Mobile early-access feedback thread (2020): https://devtrackers.gg/teamfight-tactics/p/15e47ea8-tft-mobile-early-access-feedback
- turboboost.gg, TFT mobile versus PC (20 July 2026): https://turboboost.gg/article/tft-mobile-vs-pc-differences-and-crossplay
- Mobalytics, TFT Mobile guide (search summary only): https://mobalytics.gg/?p=17475
- progameguides, TFT augments (search summary only): https://progameguides.com/teamfight-tactics/what-stages-do-you-get-augments-in-tft
- Google Play, Teamfight Tactics listing (screenshots inspected): https://play.google.com/store/apps/details?id=com.riotgames.league.teamfighttactics&hl=en_US
- Super Auto Pets wiki, The Basics: https://superautopets.wiki.gg/wiki/The_Basics
- InvenGlobal, Dota Underlords UI changes (15 June 2019): https://www.invenglobal.com/articles/8439/valve-has-changed-dota-underlords-ui-to-look-less-like-a-mobile-port

Quiz screens and readability

- Wikipedia, QuizUp: https://en.wikipedia.org/wiki/QuizUp
- Android Authority, QuizUp review (10 March 2014): https://www.androidauthority.com/quizup-review-355780/
- Wikipedia, HQ (video game): https://en.wikipedia.org/wiki/HQ_(video_game)
- screensdesign.com, Kahoot screens (captions): https://screensdesign.com/apps/kahoot-play-create-quizzes/?vs=97291
- Kahoot, accessibility: https://kahoot.com/accessibility/
- etermax help centre, power-ups (search summary only): https://triviacrack.help.etermax.com/hc/en-us/articles/360024775733-Power-ups
- etermax Technology, Tower Duel (search summary only): https://medium.com/etermax-technology/tower-duel-developing-a-real-time-game-mode-in-trivia-crack-2-b139d930f278
- Engadget, Trivia Royale (search summary only): https://www.engadget.com/battle-royale-quiz-game-trivia-royale-quizup-130038696.html
- Baymard, accidental taps on touch devices (28 March 2017): https://baymard.com/blog/handling-accidental-taps-on-touch-devices
- Brysbaert 2019, reading rate meta-analysis: https://reader.ku.edu/sites/reader/files/2024-01/How%20many%20words%20do%20we%20read%20per%20minute%20(1).pdf
- W3C, Understanding SC 1.4.1 Use of Color: https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html
- W3C, Understanding SC 2.5.8 Target Size: https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
- Microsoft, Xbox Accessibility Guideline 101: https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101

Ergonomics and platform

- Lehmann and Kipp 2018, thumb tapping by grip and screen region: http://michaelkipp.de/publication/LehmannKipp18.pdf
- Hoober 2017, Design for Fingers, Touch, and People, parts 2 and 3: https://www.uxmatters.com/mt/archives/2017/05/design-for-fingers-touch-and-people-part-2.php and https://www.uxmatters.com/mt/archives/2017/07/design-for-fingers-touch-and-people-part-3.php
- Google Accessibility Help, touch target size: https://support.google.com/accessibility/android/answer/7101858
- Apple Human Interface Guidelines, Accessibility and Layout (page data): https://developer.apple.com/tutorials/data/design/human-interface-guidelines/accessibility.json and https://developer.apple.com/tutorials/data/design/human-interface-guidelines/layout.json
- Use Your Loaf, iPhone 17 and iPhone 16 screen sizes: https://useyourloaf.com/blog/iphone-17-screen-sizes/ and https://useyourloaf.com/blog/iphone-16-screen-sizes/
- Android Developers, display cutouts and gesture navigation: https://developer.android.com/develop/ui/views/layout/display-cutout and https://developer.android.com/develop/ui/views/touch-and-input/gestures/gesturenav
- Godot documentation, Multiple resolutions (labelled 4.7): https://docs.godotengine.org/en/latest/tutorials/rendering/multiple_resolutions.html
- Godot class reference, DisplayServer: https://docs.godotengine.org/en/latest/classes/class_displayserver.html
- Godot issue 74835, safe area does not scale with stretch: https://github.com/godotengine/godot/issues/74835
- Godot engine source, viewport drag and drop: https://raw.githubusercontent.com/godotengine/godot/master/scene/main/viewport.cpp

Project files

- `CLAUDE.md`, `ARCHITECTURE.md` (sections 4, 6, 7, 11), `CARDS.md`, `HEROES.md`
- `crates/core/src/api.rs` (`Command`, `Event`, `DamageSource`, `RuleError`), `crates/lobby/src/view.rs`, `crates/lobby/src/lib.rs`, `data/tunables.toml`
- `client/scripts/` as audited on 17 November 2026
