## Watching the others after being eliminated. The host sends the living
## players as they were a few seconds ago (`view.spectate`); this screen
## shows one of them at a time: the shop they are offered (or, in a battle,
## the question and the score), their board, their gold and level. Tap a player on the left to watch them. Nothing here can
## be acted on. Holds no rules.
extends Control

signal standings
signal leave

## The seat being watched; -1 until one is picked.
var _watching := -1


func _ready() -> void:
	refresh()


func refresh() -> void:
	UI.clear(self)
	var seen: Array = Store.view.get("spectate", [])
	var by_seat := {}
	for entry in seen:
		by_seat[int(entry["id"])] = entry
	if not by_seat.has(_watching):
		_watching = -1 if seen.is_empty() else int(seen[0]["id"])

	var rail := UI.column(10)
	rail.set_anchors_preset(Control.PRESET_LEFT_WIDE)
	rail.custom_minimum_size.x = 150
	add_child(rail)
	var players: Array = (Store.view["players"] as Array).duplicate()
	players.sort_custom(func(a, b): return _rank(a) < _rank(b))
	for player in players:
		rail.add_child(_rail_entry(player, by_seat.has(int(player["id"]))))

	# Standings and Leave sit in the top right corner, above the rows, so
	# the shop and board rows have the whole width and never run under them.
	var corner := UI.row(14)
	corner.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	corner.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	corner.add_child(UI.stamp("Standings", func() -> void: standings.emit(), Vector2(230, 76), 30, UI.INK))
	corner.add_child(UI.stamp("Leave", func() -> void: leave.emit(), Vector2(170, 76), 30, UI.INK))
	add_child(corner)

	var centre := UI.column(18)
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.offset_left = 176
	centre.offset_top = 84
	centre.offset_right = -16
	centre.alignment = BoxContainer.ALIGNMENT_CENTER
	add_child(centre)
	var delay := int(Store.tunables.get("spectate_delay_ms", 0)) / 1000
	if _watching < 0:
		centre.add_child(UI.centered("Spectating", UI.GLANCE, UI.PAPER, false))
		centre.add_child(UI.centered("You watch the others %d seconds behind the match. The view opens in a moment." % delay, 32, UI.SLATE))
		return
	var entry: Dictionary = by_seat[_watching]

	# Who is being watched, and their numbers.
	var head := UI.row(24)
	head.alignment = BoxContainer.ALIGNMENT_CENTER
	head.add_child(UI.portrait(Store.hero_of(_watching), 170, UI.SLATE))
	var facts := UI.column(2)
	facts.alignment = BoxContainer.ALIGNMENT_CENTER
	facts.add_child(UI.label("Watching %s" % Store.player_name(_watching), 48, UI.PAPER, false))
	facts.add_child(UI.label("Level %s   Gold %d   %d in hand" % [UI.roman(int(entry["tavern"])), int(entry["gold"]), int(entry["hand_size"])],
		32, UI.GILT, false))
	facts.add_child(UI.label("%d seconds behind the match" % delay, 26, UI.SLATE, false))
	head.add_child(facts)
	centre.add_child(head)

	# In a battle the question takes the place of the shop.
	if entry.get("battle") != null:
		centre.add_child(_battle(entry["battle"]))
	else:
		centre.add_child(_row("Their shop", entry["shop"], true))
	centre.add_child(_row("Their board", entry["board"], false))


## The battle the watched player is in: who against, the score, and the
## question with its four answers. The right answer is not known here; the
## score is how the watcher follows who is ahead.
func _battle(battle: Dictionary) -> Control:
	var block := UI.column(10)
	var against := "a ghost"
	if battle["opponent"] != null:
		against = Store.player_name(int(battle["opponent"]))
	elif battle["ghost_of"] != null:
		against = "the ghost of %s" % Store.player_name(int(battle["ghost_of"]))
	var count := int(battle["question_count"])
	var progress := "Question %d of %d" % [int(battle["question_number"]), count] if count > 0 else "Endgame, question %d" % int(battle["question_number"])
	block.add_child(UI.centered("Against %s   ·   %s" % [against, progress], 32, UI.PAPER, false))
	if count > 0:
		block.add_child(UI.centered("%d – %d" % [int(battle["my_correct"]), int(battle["opponent_correct"])], 64, UI.PAPER, false))
	var sheet := UI.panel(UI.PAPER, 16, 10)
	sheet.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	sheet.custom_minimum_size.x = 1000
	var words := UI.column(8)
	sheet.add_child(words)
	var ink := Color("2b1d12")
	words.add_child(UI.centered(UI.category_line(battle["category"], battle.get("difficulty")), 28, ink, false))
	var text = battle.get("question_text")
	if text == null:
		words.add_child(UI.centered("The question is on its way.", 34, ink))
	else:
		words.add_child(UI.centered(String(text["text"]), 40, ink))
		var options := UI.row(40)
		options.alignment = BoxContainer.ALIGNMENT_CENTER
		for index in (text["options"] as Array).size():
			options.add_child(UI.label("%s  %s" % ["ABCD"[index], String(text["options"][index])], 34, ink, false))
		words.add_child(options)
	block.add_child(sheet)
	var locks := "%s %s   ·   opponent %s" % [Store.player_name(_watching),
		"has answered" if battle["locked"] else "is thinking", "has answered" if battle["opponent_locked"] else "is thinking"]
	block.add_child(UI.centered(locks, 28, UI.SLATE, false))
	return block


## One line of cards with a small title: shop offers (definitions) or board cards (instances).
func _row(title: String, cards: Array, offers: bool) -> Control:
	var block := UI.column(4)
	block.add_child(UI.centered(title, 28, UI.SLATE, false))
	var line := UI.row(16)
	line.alignment = BoxContainer.ALIGNMENT_CENTER
	line.custom_minimum_size.y = CardView.size_of(CardView.Mode.BOARD).y
	for card in cards:
		if offers:
			line.add_child(CardView.of_def(card, CardView.Mode.SMALL, {"cost": true}))
		else:
			line.add_child(CardView.of_instance(card, CardView.Mode.BOARD))
	if cards.is_empty():
		line.add_child(UI.centered("Nothing here", 30, UI.SLATE, false))
	block.add_child(line)
	return block


## A player on the left: tap a living one to watch them.
func _rail_entry(player: Dictionary, watchable: bool) -> Button:
	var seat := int(player["id"])
	var entry := Button.new()
	entry.custom_minimum_size = Vector2(150, 150)
	UI.bare(entry)
	var alive: bool = player["alive"]
	var health := float(player["hp"]) / maxf(1.0, float(player["max_hp"])) if alive else 0.0
	var icon := UI.icon(Store.hero_of(seat), 150, UI.GILT if seat == _watching else UI.SLATE, 3, health)
	if not alive:
		icon.modulate = Color(0.5, 0.5, 0.5, 0.6)
	entry.add_child(icon)
	UI.quiet(entry)
	if watchable:
		entry.pressed.connect(_watch.bind(seat))
	return entry


func _watch(seat: int) -> void:
	_watching = seat
	refresh()


func _rank(player: Dictionary) -> float:
	if not player["alive"]:
		return 1000.0 + (0.0 if player["placement"] == null else float(player["placement"]))
	return -float(player["hp"])
