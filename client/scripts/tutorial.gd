## The guided first match: an offline match against bots in which the game
## stops at each new thing, lights it up and says what it is. The match
## clock stands still while a note is up, so nothing is missed. Holds no
## rules: the notes describe the rules and quote their numbers from the
## tunables the host sent.
##
## Screens mark what a note can point at with `Tutorial.mark(node, name)`.
## A note shows once its screen is up, its condition holds and the thing it
## points at is on screen.
class_name Tutorial
extends CanvasLayer

## The tutorial ended, finished or skipped.
signal ended

static var instance: Tutorial

const FILE := "user://tutorial.cfg"
## The match the tutorial plays: the same every time.
const SEED := 7

## Gap around the lit-up thing, and between it and the note.
@export var hole_margin := 14.0
@export var note_gap := 26.0
## Width of the note.
@export var note_width := 760
@export var dim_strength := 0.62
## Pause after a note is closed before the next one may show, in seconds.
@export var settle := 0.5

## The name of the screen on show; set by `main.gd`.
var screen_name: Callable
## The screen to search for marked things; set by `main.gd`.
var screen: Callable

var _active := false
var _seen := {}
var _note: Control
var _saved_scale := 1.0
var _wait := 0.0
## Something happened that a note waits for: "battle_scored".
var _happened := {}


func _init() -> void:
	layer = 45
	instance = self


## Marks `node` as the thing named `name`, for a note to point at.
static func mark(node: Control, name: String) -> void:
	node.set_meta("tutorial", name)


static func done() -> bool:
	var file := ConfigFile.new()
	return file.load(FILE) == OK and bool(file.get_value("tutorial", "done", false))


static func set_done() -> void:
	var file := ConfigFile.new()
	file.set_value("tutorial", "done", true)
	file.save(FILE)


func active() -> bool:
	return _active


## A note is up and waits to be closed.
func showing() -> bool:
	return _note != null


## The id of the note on show, or "".
func current() -> String:
	return String(_note.get_meta("step")) if _note != null else ""


func begin() -> void:
	_active = true
	_seen = {}
	_happened = {}
	_wait = 0.0


func finish() -> void:
	if not _active:
		return
	_close_note()
	_active = false
	set_done()
	ended.emit()


## Something a note waits for happened: "battle_scored" (the player's own
## battle). Told by `main.gd`, which follows the match's events.
func happened(what: String) -> void:
	if _active:
		_happened[what] = true


# ---- The notes -----------------------------------------------------------------

func _number(key: String) -> int:
	return int(Store.tunables.get(key, 0))


func _battle() -> Variant:
	return Store.view.get("battle") if Store.has_match() else null


## The player has a golden card in hand, still to be placed.
func _holds_golden(me: Dictionary) -> bool:
	for card in me.get("hand", []):
		for part in card["parts"]:
			if part["golden"]:
				return true
	return false


## Every note in order: id, screen, what it points at ("" for nothing),
## when it may show, title and text.
func _steps() -> Array:
	var me := Store.me() if Store.has_match() else {}
	return [
		["welcome", "hero", "", func() -> bool: return true,
			"Welcome to Owl",
			"Owl is a trivia battle for up to six players. Each turn you buy cards in a shop and set them on your board, then you face one other player in a battle of questions. Get more answers right than they do and you hit them. Lose all your health and you are out; the last player standing wins."],
		["hero", "hero", "hero_offer", func() -> bool: return true,
			"Pick your hero",
			"You are offered two heroes. Each has its own health, a +25% bonus to one category of questions, and an ability. Hold a hero to read it, then pick one."],
		["trivia", "shop", "", func() -> bool: return true,
			"How the questions work",
			"Every question belongs to one of %d categories, such as History, Physics or Sport, and has four answers. A battle has %d questions, the same for both players. %d are picked by the house from any category. The other %d come from the two players' categories: %d each, or 3 and 1 when one player has put much more into categories than the other." % [
				Store.categories.size(),
				_number("house_questions") + _number("weighted_questions"), _number("house_questions"),
				_number("weighted_questions"), _number("weighted_questions") / 2]],
		["weights", "shop", "", func() -> bool: return true,
			"Choose what you know",
			"Your hero's +25% and your category cards are chances: with 25% in History, each of your own questions has a 25% chance to be History, and the rest are drawn from all categories. Build towards the subjects you know best. Each question is easy, medium or hard; some cards make the questions of a battle easier or harder for both players."],
		["gold", "shop", "gold", func() -> bool: return true,
			"Gold",
			"You get %d gold on the first turn and 1 more each turn, up to %d. Gold you do not spend is gone at the end of the turn." % [_number("gold_start"), _number("gold_cap")]],
		["shop", "shop", "shop_row", func() -> bool: return true,
			"The shop",
			"A card costs %d gold. Drag a card down to buy it, or tap it and press Buy. What you buy goes to your hand." % _number("card_cost")],
		["timer", "shop", "timer", func() -> bool: return true,
			"The clock",
			"The shop closes when this runs out, and the battle begins. While this note is up, the clock waits for you."],
		["hand", "shop", "hand", func() -> bool: return not (me.get("hand", []) as Array).is_empty(),
			"Your hand",
			"Cards in your hand do nothing yet. Tap the hand to open it, then drag a card up to your board, or tap it and press Place. Hold a card to read it. You can hold up to %d cards." % _number("hand_size")],
		["board", "shop", "board", func() -> bool: return not (me.get("board", []) as Array).is_empty(),
			"Your board",
			"Only cards on the board count, and it has %d places. Category cards raise your chance of your categories; others add damage, answer time, armor or reflect. With %d cards of one category on the board, every right answer in it adds %d damage when you win." % [
				_number("board_slots"), _number("category_bonus_cards"), _number("category_bonus_damage")]],
		["sell", "shop", "sign", func() -> bool: return not (me.get("board", []) as Array).is_empty(),
			"Selling",
			"To sell a card, drag it up here. You get %d gold back." % _number("sell_value")],
		["upgrade", "shop", "upgrade", func() -> bool: return int(Store.view.get("turn", 1)) >= 2,
			"Your library",
			"Raising your library level brings more cards and higher tiers into your shop, and adds to the damage you deal. It costs 1 gold less for each turn you wait."],
		["restock", "shop", "restock", func() -> bool: return int(Store.view.get("turn", 1)) >= 2,
			"Restock",
			"New cards in your shop, for %d gold." % _number("reroll_cost")],
		["reserve", "shop", "reserve", func() -> bool: return int(Store.view.get("turn", 1)) >= 2,
			"Reserve",
			"Keeps the cards of your shop for the next turn, for free. The shop turns to ice while it is reserved."],
		["players", "shop", "rail", func() -> bool: return int(Store.view.get("turn", 1)) >= 2,
			"The other players",
			"Everyone in the match, by health. Hold one to see their level, the category they collect and their last battles, then slide your finger to the next one. You only see a player's board while you fight them."],
		["ability", "shop", "ability", func() -> bool: return int(Store.view.get("turn", 1)) >= 2,
			"Your ability",
			"Your hero's ability. Tap it to use it, hold it to read what it does and what it costs."],
		["category", "battle", "question", func() -> bool:
			var battle = _battle()
			return battle != null and battle.get("opens_at") != null and Transport.now() < int(battle["opens_at"]),
			"A battle",
			"You face one player. Before each question you see its category, then the question itself."],
		["answers", "battle", "answer", func() -> bool:
			var battle = _battle()
			return battle != null and battle.has("question_text") and not battle["locked"],
			"Answer",
			"First you get %d seconds or more, longer for a long question, to read the question and its answers. Then pick one of the four answers. You have %d seconds, and cards can add up to %d more. The question ends as soon as you have both answered. Only being right counts, never being fast." % [
				roundi(_number("reading_base_ms") / 1000.0), _number("answer_time"), _number("answer_time_bonus_cap")]],
		["damage", "battle", "", func() -> bool: return _happened.has("battle_scored"),
			"Damage",
			"The player with more right answers wins and deals damage: the difference in right answers, plus %d, plus their library level, plus what their cards add. Armor takes damage before health does. An equal score is a tie and deals nothing." % _number("damage_base")],
		["golden", "shop", "", func() -> bool: return _holds_golden(me) or not (me.get("rewards", []) as Array).is_empty(),
			"A golden card",
			"Three copies of the same card merge into a golden one, with its numbers doubled. It goes to your hand. Place it on your board and you pick one free card from the next tier up. If the shop closes before you pick, the pick waits in your hand as a reward card: place that to open it again."],
		["endgame", "endgame", "", func() -> bool: return true,
			"The last two",
			"Only two players are left. Now it is one question at a time. Damage starts at 1 and grows by 1 with every question. Cards and abilities stop working; health and armor stay. Both right: nobody is hurt. Both wrong: both are."],
		["end", "shop", "", func() -> bool: return int(Store.view.get("turn", 1)) >= 3,
			"That is the game",
			"You know everything you need. Play the rest of this match as you like; hold anything you want to read again."],
	]


# ---- Showing -------------------------------------------------------------------

func _process(delta: float) -> void:
	if not _active:
		return
	if _note != null:
		_place_note()
		return
	_wait -= delta
	if _wait > 0.0:
		return
	var next := _next_step()
	if not next.is_empty():
		_open_note(String(next[0]), next[1], String(next[2]), String(next[3]))


## The first note that may show now, as [id, target, title, text]; empty if none.
func _next_step() -> Array:
	if not screen_name.is_valid() or not Store.has_match():
		return []
	var on_screen := String(screen_name.call())
	for step in _steps():
		var id := String(step[0])
		if _seen.has(id) or String(step[1]) != on_screen or not (step[3] as Callable).call():
			continue
		var target: Control = null
		if String(step[2]) != "":
			target = _find(String(step[2]))
			if target == null:
				continue
		return [id, target, step[4], step[5]]
	return []


func _find(name: String) -> Control:
	var root = screen.call() if screen.is_valid() else null
	if not root is Node or not is_instance_valid(root):
		return null
	for node in (root as Node).find_children("*", "Control", true, false):
		if node.get_meta("tutorial", "") == name and (node as Control).is_visible_in_tree():
			return node
	return null


func _open_note(id: String, target: Control, title: String, text: String) -> void:
	_seen[id] = true
	_saved_scale = Transport.time_scale
	Transport.time_scale = 0.0
	var holder := Spotlight.new()
	holder.set_meta("step", id)
	holder.dim = Color(0.02, 0.02, 0.05, dim_strength)
	holder.target = target
	holder.margin = hole_margin
	holder.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(holder)
	var sheet := UI.panel(UI.INK_RAISED, 16, 30)
	sheet.custom_minimum_size.x = UI.t(note_width)
	var stack := UI.column(12)
	sheet.add_child(stack)
	stack.add_child(UI.label(title, UI.t(44), UI.GILT))
	stack.add_child(UI.label(text, UI.t(30), Color("e6eaf2")))
	var buttons := UI.row(14)
	var ok := UI.stamp("Got it", _close_note, Vector2(0, 92), 34, UI.VERDIGRIS)
	ok.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(ok)
	var stop := UI.stamp("End the tutorial", finish, Vector2(0, 92), 28, UI.INK)
	stop.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(stop)
	stack.add_child(buttons)
	sheet.modulate.a = 0.0
	holder.add_child(sheet)
	holder.sheet = sheet
	_note = holder
	if id == "end":
		set_done()


## Puts the note beside what it points at: under it if there is room,
## else above, else centred. A note pointing at nothing sits in the middle.
func _place_note() -> void:
	var holder := _note as Spotlight
	var sheet := holder.sheet
	if sheet == null:
		return
	sheet.reset_size()
	var size := sheet.size
	var view := get_viewport().get_visible_rect().size
	var spot := holder.hole()
	var at := (view - size) * 0.5
	if spot.size != Vector2.ZERO:
		var below := spot.end.y + note_gap
		var above := spot.position.y - note_gap - size.y
		if below + size.y <= view.y - note_gap:
			at = Vector2(spot.get_center().x - size.x * 0.5, below)
		elif above >= note_gap:
			at = Vector2(spot.get_center().x - size.x * 0.5, above)
		elif spot.end.x + note_gap + size.x <= view.x:
			at = Vector2(spot.end.x + note_gap, spot.get_center().y - size.y * 0.5)
		elif spot.position.x - note_gap - size.x >= 0.0:
			at = Vector2(spot.position.x - note_gap - size.x, spot.get_center().y - size.y * 0.5)
		else:
			# What it points at fills the screen: the note keeps to the foot.
			at = Vector2((view.x - size.x) * 0.5, view.y - size.y - note_gap)
	at.x = clampf(at.x, note_gap, maxf(note_gap, view.x - size.x - note_gap))
	at.y = clampf(at.y, note_gap, maxf(note_gap, view.y - size.y - note_gap))
	sheet.position = at
	sheet.modulate.a = 1.0
	holder.queue_redraw()


## Closes the note on show, as "Got it" does.
func close() -> void:
	_close_note()


## Just after a note closed, or a note is about to show.
func settling() -> bool:
	return _active and (_wait > 0.0 or not _next_step().is_empty())


func _close_note() -> void:
	if _note == null:
		return
	_note.queue_free()
	_note = null
	Transport.time_scale = _saved_scale
	_wait = settle


## The dark layer with a lit hole over the thing a note points at. It takes
## every touch, so the match waits until the note is closed.
class Spotlight extends Control:
	var dim := Color(0, 0, 0, 0.6)
	var target: Control
	var margin := 14.0
	var sheet: Control

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP

	func hole() -> Rect2:
		if target == null or not is_instance_valid(target) or not target.is_inside_tree():
			return Rect2()
		return target.get_global_rect().grow(margin)

	func _draw() -> void:
		var all := get_viewport_rect()
		var spot := hole()
		if spot.size == Vector2.ZERO:
			draw_rect(all, dim)
			return
		spot = spot.intersection(all)
		draw_rect(Rect2(0, 0, all.size.x, spot.position.y), dim)
		draw_rect(Rect2(0, spot.end.y, all.size.x, all.size.y - spot.end.y), dim)
		draw_rect(Rect2(0, spot.position.y, spot.position.x, spot.size.y), dim)
		draw_rect(Rect2(spot.end.x, spot.position.y, all.size.x - spot.end.x, spot.size.y), dim)
		draw_rect(spot, UI.GILT, false, 4.0)
