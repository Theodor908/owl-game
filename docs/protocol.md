# Owl protocol, version 7

How a client talks to the server. The message types are defined in `crates/protocol`; this page explains them. Any change a client could notice bumps `VERSION` there and is written down here.

The offline game uses the same event and view shapes through the built-in host, so screens do not know which host they talk to.

## Two channels

- **HTTP, JSON** for accounts and friends. Short requests, each answered once.
- **One WebSocket** (`/ws`) for lobbies and matches, where the server has to speak first.

Both must run over HTTPS / WSS. The server itself speaks plain HTTP behind a front that adds encryption (`docs/server.md`).

## HTTP

Errors are `{"error": "..."}` with a status: 400 bad input, 401 not logged in or wrong password, 404 not found, 409 name taken, 429 slow down, 503 the server is busy hashing passwords.

A 429 or 503 also has a `Retry-After` header, in whole seconds, and its `error` is a plain sentence a game can show as it is, such as "too many wrong passwords; try again in 8 seconds". 429 comes from the limits in `docs/server.md`: wrong passwords for one name, password checks or new players from one address, or new players on the whole server. `GET /ws` answers 429 instead of opening when the address already has 10 game connections open. The 503 and the `Retry-After` header are additions that older games show as a plain refusal, so they did not change the protocol version.

Everything from `logout` down needs `Authorization: Bearer <token>`.

| Method and path | Body | Answer |
|---|---|---|
| `GET /api/privacy` | | The privacy notice, as text. |
| `GET /api/notice` | | `{version, text}`: the privacy notice and its version, from its `Version N.` line. |
| `POST /api/register` | `username`, `password`, `notice_acknowledged: true`, `notice_version` (optional) | 201 `{token, username, guest: false, notice_version, notice_accepted: true}`. 409 if `notice_version` is not the current one. |
| `POST /api/guest` | `notice_acknowledged: true`, `notice_version` (optional) | 201 `{token, username, guest: true, notice_version, notice_accepted: true}`. A made-up name and no password; erased when its session ends or it logs out. |
| `GET /api/version` | | `{build, notes, ...}`: the newest build of the game, as `release.bat` published it (see "The published build" below). Build 0 when none was published. |
| `GET /download/owl.apk` | | The newest whole game. |
| `GET /download/owl.pck` | | The pack of the newest build: scripts, scenes, pictures and content. |
| `GET /download/libowl_local.so` | | The native library the newest build needs (Android, arm64). |
| `POST /api/login` | `username`, `password` | `{token, username, guest: false, notice_version, notice_accepted}` |
| `POST /api/logout` | | 204. Closes the WebSocket too. |
| `GET /api/me` | | `{username, guest, notice_version, notice_accepted}`. `notice_accepted` is false when the notice changed since the user confirmed it. |
| `POST /api/me/notice` | `version` | 204. The user confirms the notice shown now; 409 if `version` is an older one. |
| `GET /api/me/export` | | Everything held about the user. |
| `DELETE /api/me` | `password` | 204. The account is erased. |
| `POST /api/me/password` | `old`, `new` | 204. Every session ends. |
| `POST /api/me/username` | `password`, `username` | `{username}`. The WebSocket closes; reconnect. |
| `GET /api/users/{username}` | | `{username}` with its exact spelling, or 404. Whole names only. |
| `GET /api/friends` | | `{friends: [{username, online}], incoming: [..], outgoing: [..]}` |
| `POST /api/friends/request` | `username` | 204. Accepts instead, if they already asked. |
| `POST /api/friends/accept` | `username` | 204 |
| `POST /api/friends/remove` | `username` | 204. Also declines or withdraws a request. |

Rules: a username has 3 to 14 letters, digits or underscores and is unique whatever its case. A password has 15 to 128 characters. After three wrong passwords for a name, each further try must wait, 1 second doubling up to 5 minutes. This also counts the password asked for when changing the password or the name, or deleting the account.

## WebSocket

Every message, both ways, is an envelope:

```json
{ "v": 5, "seq": 7, "type": "Events", "payload": { } }
```

`v` is the protocol version. `seq` counts the sender's messages from 1; a gap means something was lost, and the client asks for `Sync`. Messages are at most 16 KB.

### Opening

The client's first message must be `Hello`, within 10 seconds:

```json
{ "v": 5, "seq": 1, "type": "Hello", "payload": { "v": 5, "token": "..." } }
```

The token goes in this message, never in the address. A wrong version, a bad token, a player who has not confirmed the current privacy notice, or anything but `Hello` gets one `Error` and the connection is closed. A good one gets `Welcome`. A second connection for the same player replaces the first.

### Client to server

| Type | Payload | Meaning |
|---|---|---|
| `Hello` | `v`, `token` | First message. |
| `CreateLobby` | | Opens a lobby owned by the sender. |
| `Invite` | `username` | Owner only. The player must be a friend and online. |
| `JoinLobby` | `code` | Joins the lobby with that code: one the sender was invited to, or one whose owner told them the code. A player already in a lobby sends `LeaveLobby` first. |
| `AddBot`, `RemoveBot` | | Owner only. Puts a bot in an empty seat, or takes one out. |
| `DeclineInvite` | `code` | Turns an invitation down; the sender leaves the lobby's invited list. |
| `LeaveLobby` | | Leaves. If the owner leaves, the lobby closes for everyone. |
| `StartMatch` | `fill_with_bots` | Owner only. Plays with the lobby's players and bots, at least two seats in all. `fill_with_bots` gives every empty seat to a bot. |
| `Command` | `id`, `command` | A game command, as in `owl_core::Command`. |
| `Sync` | | Asks for a fresh `Snapshot`. |

### Server to client

| Type | Payload | Meaning |
|---|---|---|
| `Welcome` | `username`, `content`, `now` | Logged in. `content` is the cards, heroes and numbers. |
| `LobbyState` | `code`, `owner`, `members`, `invited`, `bots`, `max` | The sender's lobby, whenever it changes. `bots` is how many seats bots hold. |
| `Invited` | `from`, `code` | A friend invites the player. |
| `LobbyClosed` | | The player is no longer in a lobby. |
| `MatchStarted` | `seats`, `you` | Names by seat (bots end in "(bot)") and the player's own seat. |
| `Snapshot` | `view`, `now` | The player's whole view: at the start, on reconnect, after `Sync`. |
| `Events` | `events`, `view`, `now` | What happened, in order, and the view after it. |
| `Accepted` | `id` | The command was applied. |
| `Rejected` | `id`, `reason` | The command was refused; nothing changed. |
| `Error` | `reason` | A lobby request was refused. |

`now` is the match clock in milliseconds; deadlines in views and events use the same clock, so a client shows a countdown from the difference.

### Views and events

`view` is `owl_lobby::PlayerView` as JSON, with the text of an open question added as `battle.question_text`. Each item of `events` is an event with its timing, plus `question` (text and four options) on `QuestionShown`. Neither ever contains the correct answer before the reveal.

A question has three times, on its `QuestionShown` event and in `battle` of the view (and of each `spectate` entry): `opens_at`, when the question and its four answers show; `answers_at`, when the answers open and the answer time starts; and `deadline`, when the player's own answer time ends. Between `opens_at` and `answers_at` (`reading_base_ms`, plus `reading_per_word_ms` for every word of the question and its four answers, at most `reading_max_ms`; the same for both players) the question is only read: `SubmitAnswer` and `UseHelp` are refused with "the answers are not open yet", and an opponent, a ghost included, is never shown as having answered. Answer time bonuses count from `answers_at`. Other events with timing (`TurnStarted`) have `answers_at: null`.

A question's difficulty (`"Easy"`, `"Medium"` or `"Hard"`) is sent with its category: as `difficulty` on the `CategoryRevealed` event and in `battle.difficulty` of the view. It is the difficulty of the question that was drawn, after cards have moved it.

`damage_percent` in the view is what this turn's regular battle damage is multiplied by, in percent: 100 until `escalation_turn`, then one entry of `escalation_percent` per turn (the last repeating), and 100 in the endgame. The amounts on `DamageDealt` are already multiplied.

A golden reward is on offer while `me.rewards` is not empty; `PickReward` takes from the first entry. A golden card in `me.hand` whose part has `reward_due: true` has not given its reward yet. The reward card is an ordinary card instance in `me.hand` (card id `golden_reward`); the `Place` command opens its pick.

### Disconnects

In a lobby that has not started, a dropped connection leaves the lobby. In a running match the seat is kept: questions the player misses count as unanswered, and when they connect again they get `LobbyState`, `MatchStarted` and a `Snapshot`.

A lobby stays open after its match ends, so the same friends can start another.

## The published build

`GET /api/version` passes on `version.json` from the server's releases folder. The server does not read it; the `owl-release` tool writes it (`crates/release`). A game compares it with what it is and updates itself before its menus open.

```json
{
  "build": 5,
  "notes": "What is new, shown to players.",
  "shell": "b35f13598d6bd3bc",
  "pack": {"kind": "pack", "build": 5, "shell": "...", "native": "<SHA-256 of the library it needs>",
           "platform": "", "sha256": "...", "size": 40431548, "sig": "..."},
  "lib":  {"kind": "lib", "build": 4, "shell": "...", "native": "<its own SHA-256>",
           "platform": "android.arm64", "sha256": "...", "size": 7282080, "sig": "..."},
  "apk":  {"build": 4, "shell": "...", "native": "..."}
}
```

- `build` and `notes` are all a build 2 game reads. It offers `/download/owl.apk` when `build` is higher than its own.
- `shell` names what only a whole APK can change: the engine version, `project.godot`, `export_presets.cfg`, `owl.gdextension`, the loader (`client/scripts/boot.gd`) and the release key. A game takes a pack or a library only for its own shell.
- `pack` and `lib` describe the two files a game downloads by itself. `native` ties a pack to the one library it was built with. `build` in `lib` is the build the library was last published with.
- `sig` is an Ed25519 signature, in hex, over the entry's other fields in a fixed text form (`Entry::message` in `crates/release`). The private key is on the publishing PC only; the public key is compiled into the game's native library. A game uses a file only if its size, its SHA-256 and the signature all check out, so the server cannot hand out code of its own.
- `apk` says what the APK on the server is. A game that cannot use the pack (another shell, or a library its device will not load) asks the player to install the APK if it is newer than the one installed.

The three downloads are sent with `Cache-Control: no-store`. The game adds `?build=N` to their addresses so that nothing on the way hands out an older copy.

These are additions to HTTP answers that older games ignore, so they did not change the protocol version.

## Versions

- **7:** escalating damage: `damage_percent` in the view; `escalation_turn` and `escalation_percent` in the tunables. The category shows with its question: `category_reveal_ms` is gone from the tunables, and the next question shows `answer_reveal_ms` (now 1500) after the last answer was revealed. `CategoryRevealed` is still sent, just before `QuestionShown`.
- **6:** reading time: `answers_at` on each event (set on `QuestionShown`) and in every battle view, beside `opens_at` and `deadline`; the deadline now counts from `answers_at`; an answer or help before it is refused ("the answers are not open yet"); `reading_base_ms`, `reading_per_word_ms` and `reading_max_ms` in the tunables; `words` on each question in the state.
- **5:** questions have a difficulty (`difficulty` on `CategoryRevealed` and in `battle`); eight difficulty cards (`DifficultyShift` effect, card kind `difficulty`); the golden reward is offered when the golden card is placed (`reward_due` on a card part), and an unpicked one becomes a `golden_reward` card in hand; `difficulty_odds` in the tunables; a hero ability with limited uses per match is refused a second time in one round ("already used this round"), and `hero_state.used_turn` now also records the round of the last use for Darwin, Benz, Herodotus and Shakespeare (the view's shape is unchanged).
- **4:** the privacy notice has a version; a player who has not confirmed the current one gets `Error` on `Hello` until they do (`GET /api/notice`, `POST /api/me/notice`). Accounts unused for a year are erased.
- **3:** bots in lobbies (`AddBot`, `RemoveBot`, `bots` in `LobbyState`); a lobby can be joined by its code alone; guests; the version check and download.
- **2:** `DeclineInvite` added.
- **1:** first version.

## Not built yet

- Matchmaking with strangers. Only lobbies of friends exist.
- A bot taking over the seat of a player who stays away.
- Stored match logs.
