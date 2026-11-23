## The lobby table (docs/ui-research.md 2.6). Shown while the local player is
## not fighting (eliminated, or sitting a round out) and as the final results.
extends Control

signal play_again
signal home
## An eliminated player goes back to watching the match.
signal spectate

@export var row_height := 104


func _ready() -> void:
	refresh()


func refresh() -> void:
	UI.clear(self)
	var finished := String(Store.view["phase"]) == "Finished"
	var me := Store.me()
	var page := UI.row(24)
	page.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(page)

	var list := UI.column(12)
	list.custom_minimum_size.x = 1100
	var header := UI.row()
	header.custom_minimum_size.y = 80
	var title := UI.label("Standings", UI.GLANCE, UI.PAPER, false)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	if not finished:
		header.add_child(UI.label("Turn %d" % int(Store.view["turn"]), UI.BODY, UI.SLATE, false))
	list.add_child(header)

	# Players still alive first, by HP; then the eliminated, best placement first.
	var players: Array = (Store.view["players"] as Array).duplicate()
	players.sort_custom(func(a, b): return _rank(a) < _rank(b))
	for player in players:
		list.add_child(_row(player))
	page.add_child(list)

	var side := UI.column(24)
	side.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var result := UI.panel(UI.INK_RAISED, 16, 20)
	result.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var stack := UI.column(6)
	stack.alignment = BoxContainer.ALIGNMENT_CENTER
	result.add_child(stack)

	var heading := "Sitting out this round"
	if finished:
		heading = "You won!" if me["placement"] != null and int(me["placement"]) == 1 else "Match over"
	elif not me["alive"]:
		heading = "You are out"
	stack.add_child(UI.centered(heading, UI.GLANCE, UI.GILT))
	if me["hero"] != null:
		stack.add_child(UI.portrait(Store.hero(me["hero"]), 150, UI.GILT, 6, 16))
	if me["placement"] != null:
		stack.add_child(UI.centered("#%d" % int(me["placement"]), 104, UI.PAPER, false))
		stack.add_child(UI.centered("of %d players" % (Store.view["players"] as Array).size(), 40, UI.SLATE))
	if int(me["asked"]) > 0:
		stack.add_child(UI.centered("%d of %d answered correctly" % [int(me["correct"]), int(me["asked"])], 40))
	side.add_child(result)
	if finished:
		side.add_child(UI.stamp("Play again", func(): play_again.emit(), Vector2(0, 130), UI.GLANCE, UI.VERDIGRIS))
		side.add_child(UI.stamp("Home", func(): home.emit(), Vector2(0, 100), 38, UI.INK))
	elif not me["alive"]:
		# Out, and the match goes on: the way back to watching it.
		side.add_child(UI.stamp("Spectate", func(): spectate.emit(), Vector2(0, 130), UI.GLANCE, UI.VERDIGRIS))
	page.add_child(side)


func _row(player: Dictionary) -> Control:
	var seat := int(player["id"])
	var own := seat == Store.my_id()
	var alive: bool = player["alive"]
	var panel := PanelContainer.new()
	panel.custom_minimum_size.y = row_height
	panel.add_theme_stylebox_override("panel", UI.plate(65, 10))
	if own:
		panel.self_modulate = Color(1.25, 1.15, 0.85)
	panel.get_theme_stylebox("panel").content_margin_left = 36
	panel.get_theme_stylebox("panel").content_margin_right = 36
	var line := UI.row(20)
	panel.add_child(line)

	var place := "" if player["placement"] == null else "#%d" % int(player["placement"])
	var place_label := UI.label(place, UI.GLANCE, UI.PAPER, false)
	place_label.custom_minimum_size.x = 110
	line.add_child(place_label)
	line.add_child(UI.portrait(Store.hero_of(seat), 100, UI.GILT if own else UI.SLATE))
	var name_label := UI.label(Store.player_name(seat), 44, UI.PAPER, false)
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_label.clip_text = true
	line.add_child(name_label)
	if alive:
		line.add_child(UI.label("%d HP" % int(player["hp"]), UI.BODY, UI.PAPER, false))
		line.add_child(UI.label("Armor %d" % int(player["armor"]), UI.SMALLEST, UI.SLATE, false))
	else:
		line.add_child(UI.label("out", UI.BODY, UI.SLATE, false))
	# Library level, on a dark seal like the tier on a card.
	var tab := UI.badge(UI.roman(int(player["tavern"])), 72, Color(0.07, 0.08, 0.14), UI.PAPER, 34, UI.PAPER_SHADE)
	tab.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	line.add_child(tab)
	if not alive:
		line.modulate.a = 0.55
	return panel


func _rank(player: Dictionary) -> float:
	if player["placement"] != null:
		return float(player["placement"])
	return -float(player["hp"]) / 1000.0
