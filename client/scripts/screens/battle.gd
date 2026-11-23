## A battle, regular or endgame (docs/ui-research.md 2.3 to 2.5).
##
## Two answers sit under each thumb; the centre, which thumbs never cover,
## holds what is only read: category, timer, question and both boards.
##
## The host has already decided everything by the time an event arrives; this
## screen only decides what to show when. It keeps its own copy of the
## question on screen, because the host's snapshot moves to the next question
## the moment both players lock in, while the player still needs to see the
## answer to the one just finished.
extends Control

## The player tapped the result screen to move on.
signal continue_pressed

## Name, command, the view's counter, the picture in `client/assets/`, and
## what holding the button says.
const HELPS := [
	["Second Opinion", "SecondOpinion", "second_opinion_left", "kinds/help.png", "Removes one wrong answer from this question."],
	["Flaming Laser Sword", "FlamingSword", "sword_left", "abilities/newton.png", "Removes two wrong answers from this question."],
	["The Oracle", "Oracle", "oracle_left", "kinds/oracle.png", "Answers this question for you."],
]
## Height of the two portraits on the result.
@export var result_portrait := 400
## Height of the painted frame around the countdown bar.
@export var timer_frame_height := 56
## Measured on the plate picture (`assets/skin/plate.png`, 72 high): how far
## the bar stays from the plate's edges, sideways (past the rivets) and up
## and down (inside the rim), and how much of each end must not stretch.
@export var timer_bar_inset := Vector2(30, 15)
@export var timer_plate_end := 40
## Size of a help button in a battle.
@export var help_size := 170

## Width given to each card and its text when a build is shown.
@export var peek_column := 230
@export var wing_width := 552
@export var answer_height := 250
@export var question_font := 56
## The size of the difficulty under the category, while the category is revealed.
@export var difficulty_font := 44
## The size of the difficulty beside the timer, under the category.
@export var difficulty_tag_font := 22
@export var answer_font := 54
## Smallest size a long answer shrinks to.
@export var answer_font_min := 26
## Width of an answer plate not available to its text: the mark and the margins.
@export var answer_text_trim := 200

## Which side of the battle the local player is: 0 for A, 1 for B.
var side := 0

## The question on screen: id, text, options, category, number, count.
var _shown := {}
## Result of the question on screen, once revealed: correct, mine, theirs.
var _reveal := {}
var _my_choice := -1
## Where each option sits on this screen: slot -> option as the host numbers it.
## Shuffled for every question, so each player sees the options in their own order.
var _order: Array = [0, 1, 2, 3]
var _removed := 0
var _locked := false
## Right or wrong per finished question, for both players, with its category.
var _ledger: Array = []
## Helps the player had at some point in this battle; spent ones stay in place.
var _helps_seen := {}
## The last battle snapshot, kept for the result screen.
var _last: Dictionary = {}
## Set when the battle is over: verdict and score, damage lines, and when
## to switch from the last answer to the result.
var _result := {}
var _lines: Array = []
var _result_at := 0
## "", "mine" or "theirs": the board shown at card size.
var _board_open := ""
var _board_caption := ""
var _stage_key := ""
var _timer: Label
## The countdown ticks over this many last seconds of the answer time.
@export var tick_from := 3
## While the question is read: the word over the timer, its size, and the
## slim bar that fills under the timer until the answers open.
@export var reading_text := "Read…"
@export var reading_font := 40
@export var reading_bar_height := 8
@export var reading_bar_color := Color("8fb3d9")
## How much the answers are dimmed while they are read and not yet open.
@export var reading_answer_tint := Color(0.8, 0.8, 0.84)
## The soft cue as the answers open: the reveal sound, this many decibels
## quieter. Turned off with `answers_open_cue` false.
@export var answers_open_cue := true
@export var answers_open_quieter := 10.0
var _read_bar: ColorRect
var _sound_stage := ""
var _ticked := -1
var _bar: ColorRect
## Damage drawn over the screen; kept across rebuilds.
var _fx: DamageFx
## The portraits on screen now, "mine" and "theirs": the plates during the
## questions, the large ones on the result.
var _pictures := {}
## Damage the host has dealt in this battle, waiting for its portraits.
var _hits: Array = []


## A question was resolved. `a_correct` and `b_correct` come from the host.
func on_answer_revealed(payload: Dictionary) -> void:
	var correct := [payload["a_correct"], payload["b_correct"]]
	_reveal = {
		"correct": int(payload["correct_option"]),
		"mine": bool(correct[side]),
		"theirs": bool(correct[1 - side]),
	}
	_ledger.append({"mine": _reveal["mine"], "theirs": _reveal["theirs"], "category": int(_shown.get("category", -1))})


## The battle is over. The last answer stays up briefly, then the result.
func on_battle_over(title: String, score: String, reveal_ms: int) -> void:
	_result = {"title": title, "score": score}
	_result_at = Transport.now() + reveal_ms


func add_summary_line(line: String) -> void:
	_lines.append(line)
	_stage_key = ""


## Damage the host dealt to the local player or the opponent
## (`DamageDealt`). It is drawn flying from the other portrait: on the
## result after a regular battle, at once on the plates in the endgame. A
## reflection waits until the hit before it has landed.
func play_damage(payload: Dictionary) -> void:
	_hits.append(payload)


func _play_hits(stage: String) -> void:
	if _hits.is_empty() or (stage != "result" and not _is_endgame()):
		return
	if _fx == null:
		_fx = DamageFx.new()
		add_child(_fx)
	_fx.speed = maxf(1.0, Transport.time_scale)
	for payload in _hits:
		var target := "mine" if int(payload["to"]) == Store.my_id() else "theirs"
		var other := "theirs" if target == "mine" else "mine"
		var amount := int(payload["amount"])
		var lost := int(payload["hp_lost"])
		if String(payload["source"]) == "Reflect":
			_fx.reflect(_picture.bind(other), _picture.bind(target), amount, lost)
		else:
			_fx.strike(_picture.bind(other), _picture.bind(target), amount, lost)
	_hits.clear()


func _picture(which: String) -> Control:
	var node = _pictures.get(which)
	return node if is_instance_valid(node) else null


## Sounds that follow what is on screen: a sweep as a category is revealed,
## a softer one as the answers open, a chime or a thud as the player's answer
## turns out right or wrong, and a tick each second over the last `tick_from`
## seconds of the answer time, counted to the host's deadline.
func _sounds(stage: String, battle: Variant) -> void:
	if stage != _sound_stage:
		if stage == "category":
			Sound.instance.play("reveal")
		elif stage == "question" and _sound_stage == "reading" and answers_open_cue:
			Sound.instance.play("reveal", answers_open_quieter)
		elif stage == "reveal" and _sound_stage == "question" and not _reveal.is_empty():
			Sound.instance.play("correct" if bool(_reveal["mine"]) else "wrong")
		_sound_stage = stage
	if stage == "question" and battle != null and battle["deadline"] != null and not _locked:
		var second := ceili(UI.ms_left(battle["deadline"]) / 1000.0)
		if second >= 1 and second <= tick_from and second != _ticked:
			_ticked = second
			Sound.instance.play("tick")
	elif stage != "question":
		_ticked = -1


func _process(_delta: float) -> void:
	var battle = Store.view.get("battle")
	var now := Transport.now()
	var stage := ""
	if battle != null:
		_last = battle
	if not _result.is_empty():
		stage = "reveal" if now < _result_at else "result"
	elif battle == null:
		return
	else:
		var opens := int(battle["opens_at"]) if battle["opens_at"] != null else now
		if now >= opens:
			# The question and its answers are read first; they open at `answers_at`.
			stage = "question" if now >= _answers_at(battle) else "reading"
			_take_question(battle)
		elif _reveal.is_empty() or now >= opens - int(Store.tunables["category_reveal_ms"]):
			stage = "category"
		else:
			stage = "reveal"

	var key := "%s|%s|%s|%s|%s|%s|%s|%s" % [
		stage, _shown.get("id", -1), _locked, _removed, _reveal.size(), _lines.size(), _board_open + _board_caption,
		"" if battle == null else "%s%s%s%s" % [battle["opponent_locked"], battle["second_opinion_left"], battle["sword_left"], battle["oracle_left"]],
	]
	_sounds(stage, battle)
	if key != _stage_key:
		_stage_key = key
		_build(stage, battle)
		if _fx != null:
			move_child(_fx, -1)
	_play_hits(stage)

	if _timer != null and _bar != null:
		_timer.add_theme_font_size_override("font_size", reading_font if stage == "reading" else UI.NUMERAL)
		if _read_bar != null:
			_read_bar.visible = stage == "reading"
		if stage == "question" and battle != null and battle["deadline"] != null:
			var left := UI.ms_left(battle["deadline"])
			var total := maxf(1.0, float(battle["deadline"]) - float(_answers_at(battle)))
			_timer.text = str(ceili(left / 1000.0))
			_bar.anchor_right = clampf(left / total, 0.0, 1.0)
			_bar.color = UI.VERMILION if left < 3000 else UI.VERDIGRIS
		elif stage == "reading" and battle != null:
			# The answer time has not started: the bar stays full and still,
			# and a slim bar under it fills until the answers open.
			_timer.text = reading_text
			_bar.anchor_right = 1.0
			_bar.color = UI.VERDIGRIS
			if _read_bar != null:
				var opens := float(battle["opens_at"])
				var span := maxf(1.0, float(_answers_at(battle)) - opens)
				_read_bar.anchor_right = clampf((now - opens) / span, 0.0, 1.0)
		else:
			_timer.text = ""
			_bar.anchor_right = 1.0


## When the open question's answers open. A host without reading time sends
## no `answers_at`; then they open with the question.
func _answers_at(battle: Dictionary) -> int:
	if battle.get("answers_at") != null:
		return int(battle["answers_at"])
	return int(battle["opens_at"]) if battle.get("opens_at") != null else 0


## Copies the open question from the host's snapshot onto the screen.
func _take_question(battle: Dictionary) -> void:
	var id := int(battle["question"])
	if _shown.get("id", -1) != id:
		var text: Dictionary = battle.get("question_text", {"text": "...", "options": ["", "", "", ""]})
		_shown = {
			"id": id,
			"text": text["text"],
			"options": text["options"],
			"category": int(battle["category"]),
			"difficulty": battle.get("difficulty"),
			"number": int(battle["question_number"]),
			"count": int(battle["question_count"]),
		}
		_reveal = {}
		_my_choice = -1
		_order = [0, 1, 2, 3]
		_order.shuffle()
	_removed = int(battle["removed"])
	_locked = bool(battle["locked"])


func _is_endgame() -> bool:
	return not _last.is_empty() and int(_last["question_count"]) == 0


## The opponent's seat, or the seat a ghost copies, or -1.
func _opponent_seat() -> int:
	if _last.is_empty():
		return -1
	if _last["opponent"] != null:
		return int(_last["opponent"])
	return -1 if _last["ghost_of"] == null else int(_last["ghost_of"])


func _opponent_name() -> String:
	var seat := _opponent_seat()
	if seat < 0:
		return "Ghost"
	var text := Store.player_name(seat)
	return text if _last["opponent"] != null else "Ghost of %s" % text


# ---- Question layout ---------------------------------------------------------

func _build(stage: String, battle: Variant) -> void:
	for child in get_children():
		if child != _fx:
			remove_child(child)
			child.queue_free()
	_pictures.clear()
	_timer = null
	_bar = null
	_read_bar = null
	if stage == "result":
		_build_result()
		return
	if battle != null:
		for help in HELPS:
			if int(battle[help[2]]) > 0:
				_helps_seen[help[1]] = true

	var layout := UI.row(24)
	layout.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(layout)
	layout.add_child(_wing(stage, battle, true))
	var centre := _centre(stage, battle)
	Tutorial.mark(centre, "question")
	layout.add_child(centre)
	layout.add_child(_wing(stage, battle, false))

	if _board_open != "":
		_add_board_overlay(battle)


## One side column: plate, ledger or helps, and two answer buttons.
func _wing(stage: String, battle: Variant, left: bool) -> Control:
	var wing := UI.column(0)
	wing.custom_minimum_size.x = wing_width
	wing.add_child(_plate(stage, battle, left))
	wing.add_child(UI.spacer(0, 28))
	var middle: Control
	if _is_endgame():
		middle = _endgame_block(stage, battle, left)
	else:
		middle = _ledger_block() if left else _help_block(stage, battle)
	middle.custom_minimum_size.y = 200
	wing.add_child(middle)
	wing.add_child(UI.spacer(0, 40))
	var first := _answer(_order[0 if left else 1], stage, left)
	if left:
		Tutorial.mark(first, "answer")
	wing.add_child(first)
	wing.add_child(UI.spacer(0, 36))
	wing.add_child(_answer(_order[2 if left else 3], stage, left))
	return wing


## Portrait, name, HP and running score. Own on the left, opponent mirrored.
func _plate(stage: String, battle: Variant, own: bool) -> Control:
	var plate := Button.new()
	plate.custom_minimum_size = Vector2(wing_width, 146)
	var wood := UI.skin("plate", 40, 34, 10)
	if wood != null:
		UI.face_skin(plate, wood)
	else:
		UI.face(plate, UI.box(UI.INK_RAISED, 70, 10, UI.GILT if own else UI.SLATE.darkened(0.3), 4))
	var seat := Store.my_id() if own else _opponent_seat()
	var hero := Store.hero_of(seat) if seat >= 0 else {}
	var ghost := not own and not _last.is_empty() and _last["opponent"] == null

	var picture := UI.portrait(hero, 116, UI.GILT if own else UI.SLATE)
	_pictures["mine" if own else "theirs"] = picture
	if ghost:
		picture.modulate.a = 0.6
		_no_armor(picture)
	var facts := UI.column(0)
	facts.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	facts.alignment = BoxContainer.ALIGNMENT_CENTER
	var title := UI.label(Store.player_name(Store.my_id()) if own else _opponent_name(), 34, UI.PAPER, false)
	title.clip_text = true
	facts.add_child(title)
	var state := ""
	if seat >= 0 and not ghost:
		var player: Dictionary = Store.view["players"][seat]
		state = "HP %d  ·  Armor %d" % [int(player["hp"]), int(player["armor"])]
	facts.add_child(UI.label(state, 30, UI.SLATE, false))

	var status: Control
	if stage == "reveal" and not _reveal.is_empty():
		var right: bool = _reveal["mine"] if own else _reveal["theirs"]
		status = UI.Mark.new("check" if right else "cross", UI.VERDIGRIS if right else UI.VERMILION, 64)
	elif (stage == "question" or stage == "reading") and battle != null:
		var locked: bool = battle["locked"] if own else battle["opponent_locked"]
		status = UI.Mark.new("lock" if locked else "ring", UI.GILT if locked else UI.SLATE, 56)
	else:
		status = UI.spacer(56, 56)

	var score: Control
	if not _is_endgame() and battle != null:
		score = UI.label(str(int(battle["my_correct"] if own else battle["opponent_correct"])), UI.GLANCE, UI.PAPER, false)
		(score as Label).custom_minimum_size.x = 48
		(score as Label).horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	else:
		score = UI.spacer(0, 0)

	var line := UI.row(14)
	UI.fill(line, 14)
	var order: Array = [picture, facts, status, score]
	if not own:
		order.reverse()
		title.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		(facts.get_child(1) as Label).horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	for node in order:
		line.add_child(node)
	plate.add_child(line)
	UI.quiet(plate)
	plate.button_down.connect(_open_board.bind("mine" if own else "theirs"))
	return plate


## Right or wrong for every question so far, for both players.
func _ledger_block() -> Control:
	var panel := UI.panel(UI.INK_RAISED, 12, 26)
	var grid := GridContainer.new()
	grid.columns = 7
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 8)
	panel.add_child(grid)
	for who in ["mine", "theirs"]:
		var name_label := UI.label("You" if who == "mine" else "Them", UI.SMALLEST, UI.SLATE, false)
		name_label.custom_minimum_size.x = 100
		grid.add_child(name_label)
		for index in 6:
			grid.add_child(_ledger_mark(index, who, 60))
	return panel


func _ledger_mark(index: int, who: String, px: int) -> Control:
	if index >= _ledger.size():
		return UI.Mark.new("dot", UI.SLATE.darkened(0.3), px)
	var right: bool = _ledger[index][who]
	return UI.Mark.new("check" if right else "cross", UI.VERDIGRIS if right else UI.VERMILION, px)


## Second Opinion, Flaming Laser Sword and The Oracle: one tap, no confirmation.
func _help_block(stage: String, battle: Variant) -> Control:
	var line := UI.row(24)
	line.alignment = BoxContainer.ALIGNMENT_END
	if battle == null:
		return line
	for help in HELPS:
		if _helps_seen.has(help[1]):
			line.add_child(_help_button(help, stage, battle))
	# Clear of the painted border on the right.
	line.add_child(UI.spacer(30, 0))
	return line


func _help_button(help: Array, stage: String, battle: Dictionary) -> Button:
	var left := int(battle[help[2]])
	var press := Peek.guard(_use_help.bind(help[1]))
	var picture := UI.art(String(help[3]))
	var button: Button
	if picture == null:
		button = UI.medallion("%s\n%d" % [help[0], left], "", press, help_size, UI.PAPER, 26)
	else:
		button = UI.emblem(picture, "", "", press, Vector2(help_size, help_size))
		# Uses left, on a dark tab at the foot: not a coin, because nothing is paid in a battle.
		var uses := PanelContainer.new()
		uses.mouse_filter = Control.MOUSE_FILTER_IGNORE
		uses.add_theme_stylebox_override("panel", UI.box(Color(0.07, 0.08, 0.14, 0.95), 18, 4, Color("b9c6dc"), 2))
		uses.get_theme_stylebox("panel").content_margin_left = 14
		uses.get_theme_stylebox("panel").content_margin_right = 14
		uses.add_child(UI.centered("%d left" % left if left > 0 else "Used", 30, UI.PAPER, false))
		uses.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
		uses.grow_horizontal = Control.GROW_DIRECTION_BOTH
		uses.grow_vertical = Control.GROW_DIRECTION_BEGIN
		uses.offset_bottom = 14
		button.add_child(uses)
		if not String(help[3]).begins_with("abilities/"):
			# Card pictures are cut-outs; a dark disc behind makes them a button.
			var disc := UI.Shape.new("disc", Color(0.07, 0.08, 0.14, 0.92), UI.GILT, 5.0)
			button.add_child(disc)
			button.move_child(disc, 0)
			(button.get_child(1) as Control).offset_left = help_size * 0.16
			(button.get_child(1) as Control).offset_top = help_size * 0.16
			(button.get_child(1) as Control).offset_right = -help_size * 0.16
			(button.get_child(1) as Control).offset_bottom = -help_size * 0.16
	button.disabled = left == 0 or stage != "question" or _locked
	if button.disabled:
		button.modulate = Color(0.6, 0.6, 0.66)
	Peek.attach(button, Peek.note.bind(String(help[0]), "%s Uses left in this battle: %d." % [help[4], left]))
	return button


## Endgame: HP is the only score, so it takes the place of ledger and helps.
func _endgame_block(stage: String, battle: Variant, own: bool) -> Control:
	var line := UI.row(24)
	var seat := Store.my_id() if own else _opponent_seat()
	var plate := UI.panel(UI.INK_RAISED, 12, 28)
	plate.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var facts := UI.column(0)
	facts.alignment = BoxContainer.ALIGNMENT_CENTER
	plate.add_child(facts)
	if seat >= 0:
		var player: Dictionary = Store.view["players"][seat]
		var hp := UI.label("HP %d" % int(player["hp"]), 72, UI.PAPER, false)
		var armor := UI.label("Armor %d" % int(player["armor"]), 34, UI.SLATE, false)
		if not own:
			hp.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
			armor.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		facts.add_child(hp)
		facts.add_child(armor)
	if own:
		line.add_child(plate)
	else:
		line.add_child(plate)
		if battle != null and _helps_seen.has("Oracle"):
			line.add_child(_help_button(HELPS[2], stage, battle))
	return line


func _answer(index: int, stage: String, left: bool) -> Button:
	var button := Button.new()
	button.custom_minimum_size = Vector2(wing_width, answer_height)
	var options: Array = _shown.get("options", ["", "", "", ""])
	var blank := stage == "category"
	var gone := _removed & (1 << index) != 0 and not blank
	var correct := stage == "reveal" and index == int(_reveal.get("correct", -1))
	var mine := index == _my_choice and not blank
	var wrong_mine := stage == "reveal" and mine and not correct

	var paper := UI.PAPER_SHADE.darkened(0.25) if blank else UI.PAPER
	var border := UI.PAPER_SHADE
	var width := 4
	if correct:
		# The same pale ring and glow as a chosen answer.
		border = Color("cfe0f5")
		width = 6
	elif wrong_mine:
		border = UI.VERMILION
		width = 10
	elif mine:
		border = Color("cfe0f5")
		width = 6
	var plaque := UI.skin("answer", 70, 60, 14)
	if plaque != null:
		if blank:
			plaque.modulate_color = Color(0.6, 0.58, 0.55)
		UI.face_skin(button, plaque)
		if width > 4:
			var glow := Panel.new()
			glow.mouse_filter = Control.MOUSE_FILTER_IGNORE
			UI.fill(glow, 2)
			var ring := UI.box(Color.TRANSPARENT, 44, 0, border, width)
			ring.shadow_color = Color(border if wrong_mine else Color("4fd1b0"), 0.5)
			ring.shadow_size = 16
			glow.add_theme_stylebox_override("panel", ring)
			button.add_child(glow)
	else:
		UI.face(button, UI.box(paper, 60, 14, border, width))

	var words := "" if blank else String(options[index])
	var text := UI.centered(words, answer_font, UI.INK)
	_fit(text, words, wing_width - answer_text_trim, answer_height - 2 * (22 if plaque == null else 38))
	var mark: Control
	if correct:
		mark = UI.Mark.new("check", UI.VERDIGRIS_DEEP, 84)
	elif wrong_mine:
		mark = UI.Mark.new("cross", UI.VERMILION_DEEP, 84)
	else:
		mark = UI.spacer(84, 84)

	# The mark at the outer end, under the thumb; text at the inner end, by the question.
	var line := UI.row(12)
	UI.fill(line, 22 if plaque == null else 38)
	var order: Array = [mark, text] if left else [text, mark]
	for node in order:
		line.add_child(node)
	button.add_child(line)
	if plaque == null and (blank or wrong_mine):
		button.add_child(UI.Hatch.new())
	if blank:
		# Nothing to read yet: the empty plates step back.
		button.modulate.a = 0.35
	elif gone:
		button.modulate = Color(0.32, 0.3, 0.3) if plaque != null else Color(1, 1, 1, 0.3)
	elif (stage == "reveal" and not correct and not mine) or (stage == "question" and _locked and not mine):
		button.modulate = Color(0.5, 0.48, 0.46) if plaque != null else Color(1, 1, 1, 0.5)
	elif stage == "reading":
		# Readable, but not yet to be chosen: they light up when they open.
		button.modulate = reading_answer_tint
	UI.quiet(button)
	if stage == "question" and not _locked and not gone:
		button.pressed.connect(_answer_pressed.bind(index))
	else:
		button.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return button


## Shrinks a label's font until `words` fit in `width` by `height`, down to
## `answer_font_min`. Long answers wrap first, then get smaller.
func _fit(text: Label, words: String, width: int, height: int) -> void:
	var font := text.get_theme_font("font")
	if font == null or words == "":
		return
	var size := answer_font
	while size > answer_font_min:
		var room := font.get_multiline_string_size(words, HORIZONTAL_ALIGNMENT_CENTER, width, size)
		if room.x <= width and room.y <= height:
			break
		size -= 2
	text.add_theme_font_size_override("font_size", size)


## Category and timer, the question, and both boards as tokens.
func _centre(stage: String, battle: Variant) -> Control:
	var centre := UI.column(0)
	centre.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var category := int(_shown.get("category", -1))
	var difficulty = _shown.get("difficulty")
	if battle != null and stage == "category":
		category = int(battle["category"])
		difficulty = battle.get("difficulty")

	# Category on the left, the timer in the middle (away from both scores),
	# and the question count or the endgame stake on the right.
	var top := UI.row(16)
	top.custom_minimum_size.y = 100
	# The difficulty stands under the category: the corner is too narrow
	# for both on one line.
	var heading_text := "" if category < 0 else UI.category_label(category)
	var long_name := heading_text.length() > 10
	var heading := UI.label(heading_text, 24 if long_name else 34,
		UI.PAPER if category < 0 else UI.CATEGORY_TINTS[category])
	heading.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	heading.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var level := "" if category < 0 or difficulty == null else String(difficulty)
	if level == "":
		top.add_child(heading)
	else:
		var names := UI.column(0)
		names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		names.alignment = BoxContainer.ALIGNMENT_CENTER
		heading.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		names.add_child(heading)
		names.add_child(UI.label(level, difficulty_tag_font, UI.PAPER, false))
		top.add_child(names)
	_timer = UI.centered("", UI.NUMERAL, UI.PAPER, false)
	_timer.custom_minimum_size.x = 140
	# As tall as the numeral even while the smaller "Read…" shows, so nothing
	# moves when the answers open.
	var numeral_font := _timer.get_theme_font("font")
	if numeral_font != null:
		_timer.custom_minimum_size.y = numeral_font.get_height(UI.NUMERAL)
	top.add_child(_timer)
	var count_text := ""
	if _is_endgame():
		count_text = "Damage %d" % int(_last["question_number"])
	else:
		var number := int(_shown.get("number", 1))
		if battle != null and stage == "category":
			number = int(battle["question_number"])
		count_text = "%d / %d" % [number, int(_last.get("question_count", 6))]
	var count := UI.label(count_text, 44 if _is_endgame() else 32, UI.GILT if _is_endgame() else UI.SLATE)
	count.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	count.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	count.size_flags_vertical = Control.SIZE_EXPAND_FILL
	top.add_child(count)
	centre.add_child(top)

	centre.add_child(UI.spacer(0, 12))
	# The time left: a bar that shrinks inside a painted frame, the same
	# slate and silver plate as the buttons. The plate is drawn at one scale
	# in both directions, so its round ends keep their shape, and the bar is
	# a capsule that sits in its hollow, between the two rivets.
	var frame := Control.new()
	frame.custom_minimum_size.y = timer_frame_height
	var inset := Vector2(6, 6)
	var painted := UI.art("skin/plate.png")
	if painted != null:
		var k := timer_frame_height / float(painted.get_height())
		var plate := NinePatchRect.new()
		plate.texture = painted
		plate.patch_margin_left = timer_plate_end
		plate.patch_margin_right = timer_plate_end
		plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
		# Laid out larger and scaled down, it covers the frame exactly.
		plate.anchor_right = 1.0 / k
		plate.anchor_bottom = 1.0 / k
		plate.scale = Vector2(k, k)
		frame.add_child(plate)
		inset = (timer_bar_inset * k).round()
	else:
		var flat := Panel.new()
		flat.add_theme_stylebox_override("panel", UI.box(Color(0.05, 0.06, 0.11), 14, 0, Color(0.62, 0.68, 0.82), 3))
		UI.fill(flat)
		frame.add_child(flat)
	var track := Panel.new()
	var hollow := UI.box(Color(0.03, 0.04, 0.08), 0, 0)
	hollow.set_corner_radius_all(int((timer_frame_height - inset.y * 2.0) / 2.0))
	hollow.corner_detail = 12
	hollow.anti_aliasing_size = 1.0
	track.add_theme_stylebox_override("panel", hollow)
	track.clip_children = CanvasItem.CLIP_CHILDREN_AND_DRAW
	track.set_anchors_preset(Control.PRESET_FULL_RECT)
	track.offset_left = inset.x
	track.offset_right = -inset.x
	track.offset_top = inset.y
	track.offset_bottom = -inset.y
	_bar = ColorRect.new()
	_bar.color = UI.VERDIGRIS
	_bar.set_anchors_preset(Control.PRESET_FULL_RECT)
	track.add_child(_bar)
	# Along the foot of the bar: fills while the question is read.
	_read_bar = ColorRect.new()
	_read_bar.color = reading_bar_color
	_read_bar.anchor_top = 1.0
	_read_bar.anchor_bottom = 1.0
	_read_bar.anchor_right = 0.0
	_read_bar.offset_top = -reading_bar_height
	_read_bar.visible = stage == "reading"
	track.add_child(_read_bar)
	frame.add_child(track)
	centre.add_child(frame)
	centre.add_child(UI.spacer(0, 16))

	var sheet := UI.panel(UI.PAPER, 16, 24)
	sheet.size_flags_vertical = Control.SIZE_EXPAND_FILL
	if stage == "category":
		var stack := UI.column(8)
		stack.alignment = BoxContainer.ALIGNMENT_CENTER
		stack.add_child(UI.centered("Next category", 36, UI.VERMILION_DEEP))
		stack.add_child(UI.centered("" if category < 0 else UI.category_label(category), 84, UI.INK))
		if category >= 0 and difficulty != null:
			stack.add_child(UI.centered(String(difficulty), difficulty_font, UI.VERMILION_DEEP))
		sheet.add_child(stack)
	else:
		var stack := UI.column(8)
		stack.alignment = BoxContainer.ALIGNMENT_CENTER
		if _is_endgame() and category >= 0:
			stack.add_child(UI.centered(UI.category_line(category, difficulty), 34, UI.VERMILION_DEEP))
		var question := UI.centered(String(_shown.get("text", "")), question_font, UI.INK)
		question.size_flags_vertical = Control.SIZE_EXPAND_FILL
		stack.add_child(question)
		if stage == "question" and _locked:
			stack.add_child(UI.centered("Locked in. Waiting for your opponent.", 30, UI.VERDIGRIS_DEEP))
		sheet.add_child(stack)
	centre.add_child(sheet)

	return centre


## Holding a player's plate shows that player's build; letting go hides it.
func _open_board(which: String) -> void:
	_board_open = which


func _input(event: InputEvent) -> void:
	if _board_open == "":
		return
	var released := false
	if event is InputEventMouseButton:
		released = not event.pressed and event.button_index == MOUSE_BUTTON_LEFT
	elif event is InputEventScreenTouch:
		released = not event.pressed
	if released:
		_board_open = ""


## A build at card size with each card's text under it, over the question.
## Nothing in it takes touches: it is only up while a plate is held.
func _add_board_overlay(battle: Variant) -> void:
	var board: Array = Store.me()["board"]
	if _board_open == "theirs":
		board = [] if battle == null else battle["opponent_board"]
	var dim := ColorRect.new()
	dim.color = Color(0.03, 0.03, 0.05, 0.86)
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(centre)
	var stack := UI.column(24)
	stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
	centre.add_child(stack)
	stack.add_child(UI.centered("Your build" if _board_open == "mine" else "%s's build" % _opponent_name(), UI.GLANCE, UI.GILT, false))
	var line := UI.row(20)
	line.alignment = BoxContainer.ALIGNMENT_CENTER
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	for instance in board:
		var slot := UI.column(8)
		slot.custom_minimum_size.x = peek_column
		slot.mouse_filter = Control.MOUSE_FILTER_IGNORE
		slot.add_child(CardView.of_instance(instance, CardView.Mode.BOARD, {"still": true}))
		var first: Dictionary = Store.card(instance["parts"][0]["def"])
		var words := UI.centered(String(first["text"]), 24)
		words.vertical_alignment = VERTICAL_ALIGNMENT_TOP
		slot.add_child(words)
		line.add_child(slot)
	if board.is_empty():
		line.add_child(UI.centered("No cards placed.", 40, UI.SLATE, false))
	stack.add_child(line)


# ---- Result ------------------------------------------------------------------

func _build_result() -> void:
	var layout := UI.row(24)
	layout.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(layout)
	layout.add_child(_result_side(Store.my_id(), Store.player_name(Store.my_id()), true))

	var centre := UI.column(16)
	centre.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	centre.add_child(UI.centered(String(_result["title"]), UI.NUMERAL, UI.GILT, false))
	if String(_result["score"]) != "":
		centre.add_child(UI.centered(String(_result["score"]), UI.NUMERAL, UI.PAPER, false))

	if not _ledger.is_empty() and not _is_endgame():
		var panel := UI.panel(UI.INK_RAISED, 12, 12)
		var grid := GridContainer.new()
		grid.columns = _ledger.size() + 1
		grid.add_theme_constant_override("h_separation", 12)
		grid.add_theme_constant_override("v_separation", 8)
		panel.add_child(grid)
		grid.add_child(UI.spacer(100, 0))
		for entry in _ledger:
			var category := int(entry["category"])
			var tag := UI.centered("" if category < 0 else UI.CATEGORY_MARKS[category], 32,
				UI.SLATE if category < 0 else (UI.CATEGORY_TINTS[category] as Color).lightened(0.35), false)
			tag.custom_minimum_size.x = 72
			grid.add_child(tag)
		for who in ["mine", "theirs"]:
			grid.add_child(UI.label("You" if who == "mine" else "Them", UI.SMALLEST, UI.SLATE, false))
			for index in _ledger.size():
				grid.add_child(_ledger_mark(index, who, 72))
		var holder := CenterContainer.new()
		holder.add_child(panel)
		centre.add_child(holder)

	var lines := UI.column(4)
	lines.size_flags_vertical = Control.SIZE_EXPAND_FILL
	if _lines.is_empty():
		lines.add_child(UI.centered("Waiting for the other battles", 36, UI.SLATE))
	for line in _lines:
		lines.add_child(UI.centered(String(line), 28))
	centre.add_child(lines)
	centre.add_child(UI.stamp("Tap to continue", func(): continue_pressed.emit(), Vector2(0, 116), 36, UI.INK_RAISED))
	layout.add_child(centre)

	layout.add_child(_result_side(_opponent_seat(), _opponent_name(), false))


## One player on the result: the portrait standing free, and under it a
## plate with the name, health and armor.
func _result_side(seat: int, title: String, own: bool) -> Control:
	var side := UI.column(18)
	side.custom_minimum_size.x = wing_width
	side.alignment = BoxContainer.ALIGNMENT_CENTER
	var hero := Store.hero_of(seat) if seat >= 0 else {}
	var picture := UI.portrait(hero, result_portrait, UI.GILT if own else UI.SLATE, 6, 20)
	_pictures["mine" if own else "theirs"] = picture
	if not own and _last.get("opponent") == null:
		_no_armor(picture)
	side.add_child(picture)
	var plate := UI.panel(UI.INK_RAISED, 16, 18)
	var stack := UI.column(2)
	plate.add_child(stack)
	stack.add_child(UI.centered(title, 44, UI.PAPER, false))
	if seat >= 0 and (own or _last.get("opponent") != null):
		var player: Dictionary = Store.view["players"][seat]
		stack.add_child(UI.centered("HP %d / %d" % [int(player["hp"]), int(player["max_hp"])], 56, UI.PAPER, false))
		stack.add_child(UI.centered("Armor %d" % int(player["armor"]), 32, UI.SLATE, false))
	else:
		stack.add_child(UI.centered("A ghost takes no damage", 30, UI.SLATE))
	side.add_child(plate)
	return side


## A ghost copies a board, not a player: its portrait carries no armor.
func _no_armor(picture: Control) -> void:
	var shield := ArmorBadge.of(picture)
	if shield != null:
		shield.hero_id = ""


# ---- Input -------------------------------------------------------------------

func _answer_pressed(index: int) -> void:
	if Transport.send({"SubmitAnswer": {"player": Store.my_id(), "choice": index}}) == "":
		_my_choice = index


func _use_help(help: String) -> void:
	Transport.send({"UseHelp": {"player": Store.my_id(), "help": help}})
