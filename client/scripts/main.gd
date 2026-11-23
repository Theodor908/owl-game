## The root of the client: picks the screen for the current phase and routes
## host events to it. Landscape; every screen sits in a safe frame that keeps
## clear of notches and the home indicator (docs/ui-research.md 2.0).
##
## Command-line switches for development (after `--`):
##   --seed=N        play a specific match
##   --skip-menu     start a match at once, without the home screens
##   --lose          with --autoplay: answer wrong on purpose
##   --online=URL    with --autoplay: log in to that server (--user=, --pass=),
##                   open a lobby and start a match filled with bots
##   --guest         with --online: play as a guest instead of an account
##   --phone         enlarge small text as on a phone
##   --quit-after=S  quit after that many seconds
##   --speed=N       run the match clock N times faster
##   --autoplay      play the local seat automatically
##   --shots=DIR     save a PNG of each screen the first time it appears
##   --quit-at-end   exit when the match finishes
##   --notch         pretend to be a phone with side and bottom insets
##   --perfect       with --autoplay: always answer correctly
##   --tutorial      start the guided first match (with --autoplay: picture
##                   each note and close it)
##   --check-updates update before the menus as a player's game does; runs
##                   from the editor and runs with any switch skip that
##   --updates       take downloaded builds when run from the editor (read by
##                   `boot.gd`)
##   --borders       draw the shop with its painted top and bottom borders
extends Control

const HeroSelect := preload("res://scripts/screens/hero_select.gd")
const Shop := preload("res://scripts/screens/shop.gd")
const Battle := preload("res://scripts/screens/battle.gd")
const Standings := preload("res://scripts/screens/standings.gd")
const Home := preload("res://scripts/screens/home.gd")
const Lobby := preload("res://scripts/screens/lobby.gd")
const Collection := preload("res://scripts/screens/collection.gd")
const Versus := preload("res://scripts/screens/versus.gd")
const Placement := preload("res://scripts/screens/placement.gd")
const Spectate := preload("res://scripts/screens/spectate.gd")
const Account := preload("res://scripts/screens/account.gd")
const OnlineLobby := preload("res://scripts/screens/online_lobby.gd")
const GameMenu := preload("res://scripts/widgets/game_menu.gd")
const UpdateGate := preload("res://scripts/screens/update_gate.gd")

## The most the board is enlarged on a wide screen, and how much small
## reading text is enlarged on a phone.
## The build number shown in a corner of every screen: size, opacity and
## distance from the bottom left corner.
@export var build_mark_size := 26
@export var build_mark_alpha := 0.8
@export var build_mark_inset := Vector2(14, 6)
@export var board_zoom_max := 1.15
@export var phone_text_zoom := 1.2
## Development: behave as on a phone (`--phone`).
var _as_phone := false
## Size of the gear button that opens the game menu during a match.
@export var menu_button_size := 70
## How far the gear sits from the right edge: past the owl in the border's corner.
@export var menu_button_inset := 190
## How long the "VS" screen stays up when a match begins, in milliseconds.
@export var versus_ms := 4000
## Until when the "VS" screen shows, and whether this match has shown it.
var _versus_until := 0
var _versus_seen := false
## What the player chose on the placement screen: "" (not yet), "spectate" or "standings".
var _after_placement := ""
var _menu_button: Button
## The sheet shown when a friend invites the player.
var _invitation: Control
## The update screen is up: no menu, match or invitation until it steps aside.
var _updating := false
## Development: update first even in a development run (`--check-updates`).
var _check_updates := false
var _game_menu: Control

## How long the battle result stays up before the shop appears, in milliseconds.
@export var summary_ms := 2500

var _frame: MarginContainer
var _backdrop: TextureRect
## How strongly the painted background is darkened so the interface reads on it.
@export var backdrop_veil := 0.5
## Screen name -> background file in `client/assets/backgrounds/`.
const BACKDROPS := {
	"hero": "menu", "shop": "arena", "battle": "battle",
	"endgame": "battle", "standings": "menu",
	"home": "home", "lobby": "lobby", "collection": "menu", "versus": "battle",
	"placement": "menu", "spectate": "arena", "account": "menu", "online": "menu",
}
## Screens outside a match, which play the menu music.
const MENU_MUSIC := ["home", "lobby", "collection", "account", "online"]
## Screens of a battle or the endgame, which play the battle music.
const BATTLE_MUSIC := ["versus", "battle", "endgame"]
## Screens whose painted background must read clearly get a lighter veil.
const VEILS := {"home": 0.0, "shop": 0.12, "lobby": 0.0, "collection": 0.1, "hero": 0.1, "battle": 0.1, "endgame": 0.1, "standings": 0.1, "versus": 0.1, "placement": 0.1, "spectate": 0.12, "account": 0.1, "online": 0.1}
var _veil: ColorRect
## The player is in the menus (home, lobby, collection), not in a match.
var _in_menu := true
## Where "Back" from the collection returns to.
var _collection_from := "home"
var _skip_menu := false
## Development: start with the guided first match.
var _tutorial_first := false
## Development: the shop is drawn with its painted borders.
var _borders := false
var _screen: Control
var _screen_name := ""
## Keep showing the battle screen until this time, even if the phase moved on.
var _hold_until := 0
var _my_side := 0
var _opponent := -1
var _in_battle := false
## The local battle is scored but the turn is not resolved yet: other
## battles are still running, and damage is dealt only when all are done.
var _awaiting_resolve := false
var _reveal_until := 0
var _last_battle: Array = []

var _seed := 1
var _autoplay := false
var _shots_dir := ""
var _quit_at_end := false
var _fake_notch := false
var _perfect := false
## Development: the autoplayer answers wrong on purpose, to be eliminated.
var _lose := false
## Development: the server to play on, the account to use, and how far the walk has got.
var _online_url := ""
var _online_user := "Tester"
var _online_pass := "a long test password"
var _online_step := 0
var _online_guest := false
var _quit_after := 0.0
var _picked_hero := false
var _shots_taken := {}
## Held-detail screenshots in progress: label -> how far along (0 to 3).
var _hold_shown := {}
var _autoplay_wait := 0.0
var _quitting := false
## A screenshot is being taken; the autoplayer leaves the screen alone meanwhile.
var _shooting := false


## The build number, small in the bottom left corner of every screen, so a
## screenshot or a report always says which build it came from.
func _add_build_mark() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 120
	add_child(layer)
	var mark := UI.label("Build %d" % Online.build(), build_mark_size, Color(UI.PAPER, build_mark_alpha), false)
	mark.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	mark.grow_vertical = Control.GROW_DIRECTION_BEGIN
	mark.offset_left = build_mark_inset.x
	mark.offset_bottom = -build_mark_inset.y
	mark.offset_top = -build_mark_inset.y
	layer.add_child(mark)


func _ready() -> void:
	var background := ColorRect.new()
	background.color = UI.INK
	background.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(background)
	_backdrop = TextureRect.new()
	_backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	_backdrop.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_backdrop.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	_backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_backdrop)
	_veil = ColorRect.new()
	_veil.color = Color(UI.INK, backdrop_veil)
	_veil.set_anchors_preset(Control.PRESET_FULL_RECT)
	_veil.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_veil)
	_frame = MarginContainer.new()
	_frame.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_frame)
	# The way to the game menu during a match: a small button in the bottom
	# right corner, in the margin that every screen leaves free.
	_menu_button = UI.gear_button(_open_game_menu, menu_button_size)
	_menu_button.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	_menu_button.offset_left = -menu_button_size - menu_button_inset
	_menu_button.offset_top = -menu_button_size - 3
	_menu_button.offset_right = -menu_button_inset
	_menu_button.offset_bottom = -3
	_menu_button.visible = false
	add_child(_menu_button)
	add_child(Peek.new())
	_add_build_mark()
	add_child(Sound.new())
	var tutorial := Tutorial.new()
	tutorial.screen_name = func() -> String: return _screen_name
	tutorial.screen = func() -> Control: return _screen
	add_child(tutorial)

	_seed = int(Time.get_unix_time_from_system())
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--seed="):
			_seed = int(argument.trim_prefix("--seed="))
		elif argument.begins_with("--speed="):
			Transport.time_scale = float(argument.trim_prefix("--speed="))
		elif argument.begins_with("--shots="):
			_shots_dir = argument.trim_prefix("--shots=")
		elif argument == "--autoplay":
			_autoplay = true
		elif argument == "--quit-at-end":
			_quit_at_end = true
		elif argument == "--notch":
			_fake_notch = true
		elif argument == "--perfect":
			_perfect = true
		elif argument == "--lose":
			_lose = true
		elif argument.begins_with("--online="):
			_online_url = argument.trim_prefix("--online=")
		elif argument.begins_with("--user="):
			_online_user = argument.trim_prefix("--user=")
		elif argument.begins_with("--pass="):
			_online_pass = argument.trim_prefix("--pass=")
		elif argument == "--phone":
			_as_phone = true
		elif argument == "--guest":
			_online_guest = true
		elif argument == "--check-updates":
			_check_updates = true
		elif argument.begins_with("--quit-after="):
			_quit_after = float(argument.trim_prefix("--quit-after="))
		elif argument == "--skip-menu":
			_skip_menu = true
		elif argument == "--tutorial":
			_tutorial_first = true
		elif argument == "--borders":
			_borders = true

	_apply_safe_area()
	get_window().size_changed.connect(_apply_safe_area)
	Transport.events_received.connect(_on_events)
	Online.match_started.connect(_online_match_started)
	Online.invited.connect(_invited)
	Online.refused.connect(func(text: String) -> void:
		if not _in_menu and _screen != null and _screen.has_method("show_refusal"):
			_screen.show_refusal(text))
	Store.changed.connect(_on_store_changed)
	# Nothing of the game is reachable until it is up to date.
	if _updates_first():
		_updating = true
		_show("update")
	else:
		_open_game()


## True when the game must first bring itself up to the newest build. Runs
## from the editor and runs with a development switch are let through;
## `--check-updates` asks for the check all the same.
func _updates_first() -> bool:
	if _check_updates:
		return true
	return not OS.has_feature("editor") and OS.get_cmdline_user_args().is_empty()


## The update screen stepped aside: the game opens as it always did.
func _open_game() -> void:
	_updating = false
	if _tutorial_first:
		_start_tutorial()
	elif _skip_menu:
		_start_match()
	else:
		_show("home")


## The server started the match of the player's lobby.
func _online_match_started() -> void:
	if is_instance_valid(_invitation):
		_invitation.queue_free()
	_in_menu = false
	_menu_button.visible = true
	_versus_seen = false
	_versus_until = 0
	_after_placement = ""
	_hold_until = 0
	_last_battle = []


## A friend invites the player: a sheet over whatever is on screen.
func _invited(from: String, code: String) -> void:
	if is_instance_valid(_invitation) or not _in_menu or _updating:
		return
	_invitation = Control.new()
	_invitation.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var dim := ColorRect.new()
	dim.color = Color(0.03, 0.03, 0.05, 0.78)
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	_invitation.add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	_invitation.add_child(centre)
	var sheet := UI.panel(UI.INK_RAISED, 16, 30)
	sheet.custom_minimum_size.x = 700
	centre.add_child(sheet)
	var stack := UI.column(16)
	sheet.add_child(stack)
	stack.add_child(UI.centered("%s invites you to a lobby" % from, 44, UI.PAPER))
	# Already in a lobby: joining means leaving it, and the player is told so.
	var in_lobby := not Online.lobby.is_empty()
	if in_lobby:
		var mine := "your own lobby closes for everyone in it" if Online.is_owner() else "you leave the lobby you are in"
		stack.add_child(UI.centered("You are already in a lobby. If you join, %s." % mine, 30, UI.SLATE))
	stack.add_child(UI.stamp("Leave and join" if in_lobby else "Accept", func() -> void:
		_invitation.queue_free()
		if not Online.lobby.is_empty():
			Online.leave_lobby()
		Online.join_lobby(code)
		_open_menu("online"), Vector2(0, 110), 40, UI.VERDIGRIS))
	stack.add_child(UI.stamp("Decline", func() -> void:
		_invitation.queue_free()
		Online.decline_invite(code), Vector2(0, 100), 36, UI.INK))
	add_child(_invitation)


func _start_match() -> void:
	Transport.go_offline()
	_picked_hero = false
	_in_menu = false
	_menu_button.visible = true
	_versus_seen = false
	_versus_until = 0
	_after_placement = ""
	_hold_until = 0
	_last_battle = []
	Transport.start_match(_seed)


## The guided first match: always the same match, with notes along the way.
func _start_tutorial() -> void:
	_seed = Tutorial.SEED
	_start_match()
	Tutorial.instance.begin()


## Esc on a keyboard and the back button or gesture on a phone do the same
## thing: in a match they open or close the game menu, in the menus they go
## one screen back, and on the home screen they leave the game.
func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		_go_back()


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_GO_BACK_REQUEST:
		_go_back()


func _go_back() -> void:
	if _updating:
		get_tree().quit()
	elif not _in_menu:
		if is_instance_valid(_game_menu):
			_game_menu.queue_free()
		else:
			_open_game_menu()
	elif _screen_name == "collection":
		_open_menu(_collection_from)
	elif _screen_name == "online":
		_open_menu("lobby")
	elif _screen_name == "lobby" or _screen_name == "account":
		_open_menu("home")
	else:
		get_tree().quit()


func _open_game_menu() -> void:
	if is_instance_valid(_game_menu):
		return
	_game_menu = GameMenu.new()
	_game_menu.concede.connect(_concede)
	add_child(_game_menu)


## Leaves the match for the home screen. Offline this simply abandons the
## match; what conceding means for the other players is a rule for `core`
## to define when matches are online.
func _concede() -> void:
	if Transport.online:
		# Leaving the lobby gives up the seat; the others play on.
		Online.leave_lobby()
		Transport.go_offline()
	Transport.stop_match()
	_open_menu("home")


func _open_menu(screen_name: String) -> void:
	# Back in the menus, the guided match is over.
	Tutorial.instance.finish()
	_in_menu = true
	_menu_button.visible = false
	_show(screen_name)


func _menu_chosen(where: String) -> void:
	if where == "play":
		_open_menu("lobby")
	elif where == "profile":
		_open_menu("account")
	else:
		_collection_from = "home"
		_open_menu("collection")


## Pads the frame so nothing interactive sits under a notch, a rounded corner
## or the home indicator. The larger side inset is used on both sides to keep
## the layout symmetric. Derived from the documented behaviour of
## `get_display_safe_area()`; not yet tested on a device.
func _apply_safe_area() -> void:
	var side := UI.MARGIN_SIDE
	var top := UI.MARGIN_TOP
	var bottom := UI.MARGIN_BOTTOM
	if _fake_notch:
		side = 175
	elif OS.has_feature("mobile"):
		var safe := DisplayServer.get_display_safe_area()
		var window := DisplayServer.window_get_size()
		var view := get_viewport().get_visible_rect().size
		if window.x > 0 and window.y > 0:
			var sx := view.x / window.x
			var sy := view.y / window.y
			side = maxi(side, ceili(maxi(safe.position.x, window.x - safe.end.x) * sx))
			top = maxi(top, ceili(safe.position.y * sy))
			bottom = maxi(bottom, ceili((window.y - safe.end.y) * sy))
	# The shape of the screen decides how large the board is drawn: seven
	# board cards and both side columns have to fit across it.
	var visible := get_viewport().get_visible_rect().size
	var fits := (visible.x - 2.0 * side - 72.0) / 1752.0
	var wanted := clampf(fits, 1.0, board_zoom_max)
	UI.text_zoom = phone_text_zoom if (OS.has_feature("mobile") or _as_phone) else 1.0
	if not is_equal_approx(wanted, UI.zoom):
		UI.zoom = wanted
		if _screen != null and _screen.has_method("refresh") and Store.has_match():
			_screen.refresh()
	_frame.add_theme_constant_override("margin_left", side)
	_frame.add_theme_constant_override("margin_right", side)
	_frame.add_theme_constant_override("margin_top", top)
	_frame.add_theme_constant_override("margin_bottom", bottom)


func _process(delta: float) -> void:
	if _quit_after > 0.0:
		_quit_after -= delta
		if _quit_after <= 0.0:
			get_tree().quit()
			return
	if _updating:
		if _screen != null and not _shooting:
			_shot_first("update_%s" % _screen.state)
		return
	if _in_menu:
		if _autoplay:
			_menu_autoplay()
		return
	if not Store.has_match():
		return
	_show(_wanted_screen())
	# The finished standings have their own way home; the gear would sit on it.
	_menu_button.visible = not (["standings", "placement"].has(_screen_name) and String(Store.view["phase"]) == "Finished")
	if _autoplay and Tutorial.instance.active() and _tutorial_autoplay():
		pass
	elif _autoplay:
		_autoplay_wait -= delta
		if _autoplay_wait <= 0.0:
			_autoplay_wait = 0.15
			_autoplay_step()
	if _quit_at_end and not _quitting and String(Store.view["phase"]) == "Finished" and Transport.now() >= _hold_until:
		_quitting = true
		await _shoot("results")
		get_tree().quit()


## The battle screen's name for the current phase. The endgame gets its own
## name so that it starts on a fresh screen after the last regular battle.
func _battle_screen() -> String:
	return "endgame" if String(Store.view["phase"]) == "Endgame" else "battle"


func _on_battle_screen() -> bool:
	return _screen_name == "battle" or _screen_name == "endgame"


func _wanted_screen() -> String:
	# Once per match, when hero select is over and everyone has a hero.
	if not _versus_seen and String(Store.view["phase"]) == "Shop":
		_versus_seen = true
		_versus_until = Transport.now() + versus_ms
	if Transport.now() < _versus_until:
		return "versus"
	if Transport.now() < _hold_until:
		return _screen_name if _on_battle_screen() else "battle"
	var alive: bool = Store.me()["alive"]
	var over := String(Store.view["phase"]) == "Finished"
	# Out of the match, or the match is over: the placement first, then
	# whatever the player chose there. Spectating ends with the match.
	if not alive or over:
		if _after_placement == "":
			return "placement"
		return "spectate" if _after_placement == "spectate" and not over else "standings"
	match String(Store.view["phase"]):
		"HeroSelect":
			return "hero"
		"Shop":
			return "shop"
		"Battle", "Endgame":
			return _battle_screen() if Store.view["battle"] != null else "standings"
		_:
			return "standings"


func _show(screen_name: String) -> void:
	if screen_name == _screen_name:
		return
	_screen_name = screen_name
	# The menus have their own music; everything inside a match shares the other.
	Sound.instance.music("menu" if MENU_MUSIC.has(screen_name) else ("battle" if BATTLE_MUSIC.has(screen_name) else "game"))
	if _screen != null:
		_frame.remove_child(_screen)
		_screen.queue_free()
	match screen_name:
		"hero":
			_screen = HeroSelect.new()
		"shop":
			_screen = Shop.new()
			_screen.last_battle = _last_battle
			if _borders:
				_screen.use_borders = true
		"battle", "endgame":
			_screen = Battle.new()
			_screen.side = _my_side
			_screen.continue_pressed.connect(_skip_result)
		"placement":
			_screen = Placement.new()
			_screen.spectate.connect(func() -> void: _after_placement = "spectate")
			_screen.standings.connect(func() -> void: _after_placement = "standings")
			_screen.leave.connect(_concede)
		"spectate":
			_screen = Spectate.new()
			_screen.standings.connect(func() -> void: _after_placement = "standings")
			_screen.leave.connect(_concede)
		"versus":
			_screen = Versus.new()
			_screen.skipped.connect(func() -> void: _versus_until = 0)
		"update":
			_screen = UpdateGate.new()
			_screen.done.connect(_open_game)
		"home":
			_screen = Home.new()
			_screen.chosen.connect(_menu_chosen)
		"lobby":
			_screen = Lobby.new()
			_screen.play.connect(_play_again)
			_screen.back.connect(_open_menu.bind("home"))
			_screen.collection.connect(func() -> void:
				_collection_from = "lobby"
				_open_menu("collection"))
			# Friends need an account; without one the profile is where to make it.
			_screen.friends.connect(func() -> void: _open_menu("online" if Online.logged_in() else "account"))
			_screen.tutorial.connect(_start_tutorial)
		"account":
			_screen = Account.new()
			_screen.back.connect(_open_menu.bind("home"))
			_screen.friends.connect(_open_menu.bind("online"))
		"online":
			_screen = OnlineLobby.new()
			_screen.back.connect(_open_menu.bind("lobby"))
		"collection":
			_screen = Collection.new()
			_screen.back.connect(func() -> void: _open_menu(_collection_from))
		_:
			_screen = Standings.new()
			_screen.spectate.connect(func() -> void: _after_placement = "spectate")
			# Online, "play again" goes back to the lobby, which stays open.
			_screen.play_again.connect(func() -> void:
				if Transport.online:
					_open_menu("online")
				else:
					_play_again())
			_screen.home.connect(_open_menu.bind("home"))
	_frame.add_child(_screen)
	_backdrop.texture = UI.art("backgrounds/%s.jpg" % BACKDROPS.get(screen_name, "shop"))
	_veil.color = Color(UI.INK, float(VEILS.get(screen_name, backdrop_veil)))


func _play_again() -> void:
	_seed += 1
	_start_match()


## The player tapped the result: move on, unless damage is still to come.
func _skip_result() -> void:
	if not _awaiting_resolve:
		_hold_until = 0


func _on_store_changed() -> void:
	if not _in_menu and _screen != null and _screen.has_method("refresh"):
		_screen.refresh()


func _on_events(items: Array) -> void:
	if _in_menu:
		return
	var me := Store.my_id()
	for item in items:
		var event: Dictionary = item["event"]
		var kind: String = event.keys()[0]
		var payload: Dictionary = event[kind]
		match kind:
			"BattleStarted", "EndgameStarted":
				if kind == "EndgameStarted":
					_resolved()
				var a := int(payload["a"])
				var b := -1 if payload.get("b") == null else int(payload["b"])
				_in_battle = a == me or b == me
				if _in_battle:
					_my_side = 0 if a == me else 1
					_opponent = b if a == me else a
					if kind == "BattleStarted":
						_last_battle = []
			"AnswerRevealed":
				if _in_battle:
					if not _on_battle_screen():
						_show(_battle_screen())
					_screen.on_answer_revealed(payload)
			"BattleScored":
				if _in_battle:
					_battle_over(payload)
			"CardPlaced":
				if int(payload["player"]) == me:
					Sound.instance.play("place")
			"GoldenMade":
				if int(payload["player"]) == me:
					Sound.instance.play("golden")
			"DamageDealt":
				if _awaiting_resolve:
					_note_damage(payload)
				var hurt := int(payload["to"])
				if _on_battle_screen() and (hurt == me or hurt == _opponent):
					_screen.play_damage(payload)
			"TurnStarted", "MatchFinished":
				_resolved()
				if kind == "MatchFinished":
					if _in_battle and _on_battle_screen():
						_hold_until = Transport.now() + summary_ms
						_screen.on_battle_over("The match is over", "", 0)
					_in_battle = false


func _battle_over(payload: Dictionary) -> void:
	var scores := [int(payload["a_correct"]), int(payload["b_correct"])]
	var mine: int = scores[_my_side]
	var theirs: int = scores[1 - _my_side]
	var result := "Victory" if mine > theirs else ("Defeat" if mine < theirs else "Tie")
	_last_battle = ["Last battle: %s %d - %d" % [result.to_lower(), mine, theirs]]
	Tutorial.instance.happened("battle_scored")
	var reveal_ms := int(Store.tunables["answer_reveal_ms"])
	# Stay on the battle screen until the whole turn is resolved; the damage
	# lines arrive then. `_resolved` sets the real end of the hold.
	_awaiting_resolve = true
	_reveal_until = Transport.now() + reveal_ms
	_hold_until = Transport.now() + 10 * 60 * 1000
	if not _on_battle_screen():
		_show("battle")
	_screen.on_battle_over(result, "%d – %d" % [mine, theirs], reveal_ms)
	# The sting sounds as the result shows, once the last answer has been seen.
	if result != "Tie":
		Sound.instance.cue("victory" if result == "Victory" else "defeat", reveal_ms / 1000.0)
	_in_battle = false


## The turn has been resolved: show the result a little longer, then move on.
func _resolved() -> void:
	if not _awaiting_resolve:
		return
	_awaiting_resolve = false
	_hold_until = maxi(_reveal_until, Transport.now()) + summary_ms


## Adds a line about damage in the local player's battle to the result.
func _note_damage(payload: Dictionary) -> void:
	var target := int(payload["to"])
	if target != Store.my_id() and target != _opponent:
		return
	var who := "You took" if target == Store.my_id() else "%s took" % Store.player_name(target)
	var line := "%s %d damage" % [who, int(payload["amount"])]
	if int(payload["armor_absorbed"]) > 0:
		line += " (%d blocked by armor)" % int(payload["armor_absorbed"])
	if String(payload["source"]) != "Battle":
		line += ", from %s" % String(payload["source"]).to_lower()
	_last_battle.append(line)
	if _on_battle_screen():
		_screen.add_summary_line(line)


# ---- Development helpers -----------------------------------------------------

## Saves a PNG of the screen, once per label. Waits for the frame to be
## drawn first, so the picture shows what was just built.
func _shoot(label: String) -> void:
	if _shots_dir == "" or _shots_taken.has(label):
		return
	_shots_taken[label] = true
	var number := _shots_taken.size()
	_shooting = true
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(_shots_dir.path_join("%02d_%s.png" % [number, label]))
	_shooting = false


## For the screenshots only: Benz combines two cards of his board once, to
## picture his ability's counter before the use (2/2) and after it (1/2,
## greyed for the rest of the round). Returns true while it is busy.
func _benz_shots(me: Dictionary) -> bool:
	if _shots_dir == "" or _shots_taken.has("ability_used") or me["hero"] == null:
		return false
	if String(Store.hero(me["hero"]).get("kind", "")) != "Benz":
		return false
	var board: Array = me["board"]
	if int(me["hero_state"]["used_turn"]) != int(Store.view["turn"]):
		if board.size() < 2:
			return false
		if _shot_first("ability_unused"):
			return true
		var combine := {"Combine": {"a": int(board[0]["id"]), "b": int(board[1]["id"])}}
		Transport.send({"UseAbility": {"player": Store.my_id(), "action": combine}})
		return true
	return _shot_first("ability_used")


## Starts a screenshot if this label has none yet. Returns true when it did,
## so the autoplayer can leave the screen alone until the picture is taken.
func _shot_first(label: String) -> bool:
	if _shots_dir == "" or _shots_taken.has(label):
		return false
	_shoot(label)
	return true


## Walks through the menus once, taking a picture of each, then starts the match.
func _menu_autoplay() -> void:
	if _shooting:
		return
	if _online_url != "":
		_online_autoplay()
		return
	match _screen_name:
		"home":
			if not _shot_first("home"):
				_open_menu("lobby")
		"lobby":
			if _shots_dir != "" and _shots_taken.has("lobby") and not _shots_taken.has("lobby_choose"):
				_screen.choose_match()
				_shoot("lobby_choose")
			elif not _shot_first("lobby"):
				_collection_from = "lobby"
				_open_menu("collection")
		"collection":
			if not _shot_first("collection"):
				_start_match()


## Logs in (making the account if need be), opens a lobby and starts a
## match filled with bots, taking a picture of each screen on the way.
func _online_autoplay() -> void:
	match _online_step:
		0:
			Online.set_server(_online_url)
			if _online_guest:
				Online.play_as_guest(true)
			else:
				# If the account exists already, registering fails and logging in follows.
				Online.failed.connect(func(_text: String) -> void:
					if _online_step == 1 and not Online.logged_in():
						Online.login(_online_user, _online_pass))
				Online.register(_online_user, _online_pass, true)
			_online_step = 1
		1:
			# A changed privacy notice: picture the request, then read and confirm it.
			if Online.logged_in() and Online.notice_pending and not _shots_taken.has("account_notice"):
				_open_menu("account")
				_shoot("account_notice")
				Online.fetch_privacy(func(_text: String) -> void: Online.accept_notice())
			elif Online.logged_in() and Online.connected:
				_open_menu("account")
				_online_step = 2
		2:
			if not _shot_first("account"):
				_open_menu("online")
				Online.create_lobby()
				_online_step = 3
		3:
			# Two bots, then a match of three.
			if not Online.lobby.is_empty():
				Online.add_bot()
				Online.add_bot()
				_online_step = 4
		4:
			if int(Online.lobby.get("bots", 0)) == 2 and not _shot_first("online_lobby"):
				Online.start_match(false)
				_online_step = 5


func _is_square(node: Node, side: int) -> bool:
	return not node is CardView and (node as Control).custom_minimum_size == Vector2(side, side)


## For the screenshots: shows the held details of the first thing `wanted`
## accepts, and takes the picture one step later, once they are laid out.
## Returns true while it still has something to do.
func _hold_shot(what: String, wanted: Callable) -> bool:
	var label := "shop_hold_%s" % what
	var stage := int(_hold_shown.get(label, 0))
	if _shots_dir == "" or stage == 3:
		return false
	_hold_shown[label] = stage + 1
	match stage:
		0:
			Peek.instance.demo(_screen, wanted)
		1:
			_shoot(label)
		2:
			Peek.instance.end_demo()
	return true


## In the guided match: pictures each note, then closes it. True while the
## tutorial has something to show, so the autoplayer waits for it.
func _tutorial_autoplay() -> bool:
	if _shooting:
		return true
	if Tutorial.instance.showing():
		if not _shot_first("tutorial_%s" % Tutorial.instance.current()):
			Tutorial.instance.close()
		return true
	return Tutorial.instance.settling()


## Presses the button of the screen on show that carries these words, as a tap would.
func _press(words: String) -> void:
	for node in _screen.find_children("*", "Button", true, false):
		if (node as Button).text == words:
			(node as Button).pressed.emit()
			return


## Where the reward card sits in the hand, or -1.
func _reward_card_index(me: Dictionary) -> int:
	var hand: Array = me["hand"]
	for index in hand.size():
		if String(Store.card(hand[index]["parts"][0]["def"]).get("id", "")) == CardView.REWARD_CARD:
			return index
	return -1


## The shop offer the autoplayer buys: a copy of a card it already holds,
## so that golden cards get made; otherwise the first.
func _wanted_offer(me: Dictionary) -> int:
	var shop: Array = me["shop"]
	for index in shop.size():
		for card in (me["board"] as Array) + (me["hand"] as Array):
			var parts: Array = card["parts"]
			if parts.size() == 1 and not parts[0]["golden"] and int(parts[0]["def"]) == int(shop[index]) 					and Store.card(shop[index])["can_golden"]:
				return index
	return 0


## Plays the local seat with simple choices, so a whole match can run unattended.
func _autoplay_step() -> void:
	if _shooting:
		return
	var me := Store.me()
	var id := Store.my_id()
	match _screen_name:
		"versus":
			_shot_first("versus")
		"hero":
			if _shot_first("hero_select"):
				return
			if me["hero"] == null and not _picked_hero:
				# Once: online the answer comes later, and asking again would be refused.
				_picked_hero = true
				Transport.send({"PickHero": {"player": id, "hero": int(me["hero_offer"][0])}})
		"shop":
			if not (me["rewards"] as Array).is_empty():
				if _shot_first("golden_reward"):
					return
				# For the screenshots only: the first reward is left unpicked,
				# so it comes back as a reward card on the next turn.
				if _shots_dir != "" and not _shots_taken.has("shop_reward_card"):
					return
				Transport.send({"PickReward": {"player": id, "index": 0}})
				return
			var reward_card := _reward_card_index(me)
			if _shots_dir != "" and reward_card >= 0 and not _shots_taken.has("shop_reward_inspect"):
				if not _shots_taken.has("shop_reward_card"):
					_screen.open_hand()
					_shoot("shop_reward_card")
					return
				_screen.inspect("hand", reward_card)
				_shoot("shop_reward_inspect")
				return
			if reward_card >= 0:
				_screen.close_overlays()
				Transport.send({"Place": {"player": id, "card": int(me["hand"][reward_card]["id"])}})
				return
			if _benz_shots(me):
				return
			if int(Store.view["turn"]) >= 2 and (me["board"] as Array).size() >= 2 and not (me["shop"] as Array).is_empty():
				if _shot_first("shop"):
					return
				if _shots_dir != "" and not _shots_taken.has("game_menu"):
					_open_game_menu()
					_shoot("game_menu")
					return
				if is_instance_valid(_game_menu):
					_game_menu.queue_free()
				if _shots_dir != "" and not _shots_taken.has("shop_inspect"):
					_screen.inspect("shop", 0)
					_shoot("shop_inspect")
					return
				if _shots_dir != "" and not _shots_taken.has("shop_closed"):
					_shots_taken["shop_closed"] = true
					_screen.close_overlays()
					return
				# Held details cannot be triggered without a finger; show them directly.
				if _hold_shot("card", func(node: Node) -> bool: return node is CardView):
					return
				if _hold_shot("ability", func(node: Node) -> bool: return _is_square(node, _screen.ability_size)):
					return
				if _hold_shot("hero", func(node: Node) -> bool: return _is_square(node, _screen.hero_size)):
					return
				if _hold_shot("player", func(node: Node) -> bool: return _is_square(node, _screen.rail_size())):
					return
			# For the screenshots only: fill the hand once, to show it as a
			# small fan, opened, and with one of its cards held.
			if _shots_dir != "" and not _shots_taken.has("shop_hand_closed") and int(Store.view["turn"]) >= 4:
				if not _shots_taken.has("shop_hand_open"):
					if not (me["shop"] as Array).is_empty() and Transport.send({"Buy": {"player": id, "index": 0}}) == "":
						return
					if not (me["hand"] as Array).is_empty():
						if _shot_first("shop_hand"):
							return
						_screen.open_hand()
						_shoot("shop_hand_open")
						return
				else:
					if _hold_shot("hand_card", func(node: Node) -> bool: return node.get_parent() is HandFan):
						return
					_shots_taken["shop_hand_closed"] = true
					_screen.close_hand()
					return
			if not (me["hand"] as Array).is_empty() and Transport.send({"Place": {"player": id, "card": int(me["hand"][0]["id"])}}) == "":
				pass
			elif not (me["shop"] as Array).is_empty() and Transport.send({"Buy": {"player": id, "index": _wanted_offer(me)}}) == "":
				pass
			elif int(Store.view["turn"]) % 2 == 0:
				Transport.send({"UpgradeTavern": {"player": id}})
		"battle", "endgame":
			var battle = Store.view["battle"]
			if battle == null or Transport.now() < _hold_until:
				if not _awaiting_resolve and _hold_until - Transport.now() < summary_ms - 400:
					_shoot("battle_result")
				return
			var endgame := int(battle["question_count"]) == 0
			if battle["opens_at"] != null and Transport.now() < int(battle["opens_at"]):
				if int(battle["question_number"]) == 1 or endgame:
					_shoot("endgame_category" if endgame else "battle_category")
				else:
					_shoot("battle_reveal")
				return
			if battle["locked"] or not battle.has("question_text"):
				return
			if _shot_first("endgame_question" if endgame else "battle_question"):
				return
			# The placeholder sums can be worked out; a real question gets a guess.
			var text := String(battle["question_text"]["text"])
			var choice := randi() % 4
			if text.begins_with("What is ") and text.contains(" + "):
				var numbers := text.replace("What is ", "").replace("?", "").split(" + ")
				choice = (battle["question_text"]["options"] as Array).find(str(int(numbers[0]) + int(numbers[1])))
			if _lose:
				choice = (choice + 1) % 4
			elif not _perfect and randf() < 0.25:
				choice = randi() % 4
			Transport.send({"SubmitAnswer": {"player": id, "choice": choice}})
		"standings":
			# For the screenshots only: out of a match that goes on, look at
			# the standings once and go back to watching.
			if _shot_first("standings_out" if not me["alive"] and String(Store.view["phase"]) != "Finished" else "standings"):
				return
			if _shots_dir != "" and not me["alive"] and String(Store.view["phase"]) != "Finished":
				_press("Spectate")
		"placement":
			if not _shot_first("placement"):
				_after_placement = "standings" if String(Store.view["phase"]) == "Finished" else "spectate"
		"spectate":
			# The view opens only once the delay has passed.
			if not (Store.view.get("spectate", []) as Array).is_empty():
				if _shot_first("spectate"):
					return
				for entry in Store.view["spectate"]:
					if entry.get("battle") != null and entry["battle"].has("question_text") and not _shots_taken.has("spectate_battle"):
						_screen._watch(int(entry["id"]))
						_shoot("spectate_battle")
						return
				# For the screenshots only: once, over to the standings and back.
				if _shots_dir != "" and not _shots_taken.has("standings_out"):
					_press("Standings")
