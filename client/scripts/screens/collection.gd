## The collection, laid out as a book: tabs along the top (heroes, one per
## tier, the cards with no tier), a parchment page of full cards under them,
## and arrows to turn the page. Pressing a card or a hero opens it large with
## its description beside it. It lists what the content files hold and knows
## no rules.
extends Control

signal back

## Cards across and down on one page.
@export var columns := 5
@export var rows := 2
## Size of a card on the page, as a share of the full-size card.
@export var card_scale := 0.58
## Size of the card in the opened view, as a share of the full-size card.
@export var open_scale := 1.35
@export var arrow_width := 96

const HEROES := -1
const PAGE_INK := Color("2b1d12")

## The open tab: `HEROES`, or a tier number (0 holds the cards with no tier).
var _tab := 1
var _page := 0
## What is opened large: a card index or a hero dictionary. Null: nothing.
var _open: Variant = null
## The opened card shows its golden version.
var _golden := false


func _ready() -> void:
	_build()


## The tabs, in order, each with its label.
func _tabs() -> Array:
	var tiers := {}
	for card in Store.cards:
		tiers[int(card["tier"])] = true
	var order: Array = tiers.keys()
	order.sort()
	var tabs: Array = [[HEROES, "Heroes"]]
	for tier in order:
		if tier > 0:
			tabs.append([tier, "Tier %s" % UI.roman(tier)])
	if tiers.has(0):
		tabs.append([0, "Special"])
	return tabs


## What the open tab holds: hero dictionaries, or card indexes.
func _entries() -> Array:
	if _tab == HEROES:
		return Store.heroes
	var found: Array = []
	for index in Store.cards.size():
		if int(Store.cards[index]["tier"]) == _tab:
			found.append(index)
	return found


func _build() -> void:
	UI.clear(self)
	var layout := UI.column(14)
	layout.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(layout)

	# Back on the left, the tabs centred on the screen; an empty seat as wide
	# as Back on the right keeps the centre true.
	var top := UI.row(10)
	top.custom_minimum_size.y = 96
	top.add_child(UI.stamp("Back", func() -> void: back.emit(), Vector2(200, 92), 36, UI.INK))
	var tabs := UI.row(10)
	tabs.alignment = BoxContainer.ALIGNMENT_CENTER
	tabs.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for tab in _tabs():
		var on := int(tab[0]) == _tab
		var plate := UI.stamp(String(tab[1]), _show_tab.bind(int(tab[0])), Vector2(178, 92), 34, UI.INK)
		if on:
			plate.self_modulate = Color(1.7, 1.7, 1.9)
			for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
				plate.add_theme_color_override(state, Color("e8eef8"))
		else:
			plate.modulate = Color(0.82, 0.82, 0.88)
		tabs.add_child(plate)
	top.add_child(tabs)
	top.add_child(UI.spacer(200, 0))
	layout.add_child(top)

	var entries := _entries()
	var per_page := columns * rows
	var pages := maxi(1, ceili(entries.size() / float(per_page)))
	_page = clampi(_page, 0, pages - 1)

	var body := UI.row(12)
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_child(_arrow("<", -1, _page > 0))
	var sheet := UI.panel(UI.PAPER, 16, 10)
	sheet.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var page := UI.column(8)
	sheet.add_child(page)
	var grid := GridContainer.new()
	grid.columns = columns
	grid.size_flags_vertical = Control.SIZE_EXPAND_FILL
	grid.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	grid.add_theme_constant_override("h_separation", 26)
	grid.add_theme_constant_override("v_separation", 14)
	for entry in entries.slice(_page * per_page, (_page + 1) * per_page):
		grid.add_child(_tile(entry))
	page.add_child(grid)
	var foot := "Page %d of %d   ·   Press a %s to read it" % [_page + 1, pages, "hero" if _tab == HEROES else "card"]
	page.add_child(UI.centered(foot, 28, PAGE_INK, false))
	body.add_child(sheet)
	body.add_child(_arrow(">", 1, _page < pages - 1))
	layout.add_child(body)

	if _open != null:
		_add_opened()


func _show_tab(tab: int) -> void:
	_tab = tab
	_page = 0
	_build()


func _turn(by: int) -> void:
	_page += by
	_build()


func _arrow(_sign_text: String, by: int, enabled: bool) -> Button:
	var arrow := UI.stamp("", _turn.bind(by), Vector2(arrow_width, 220), 64, UI.INK)
	var head := UI.Arrow.new(by, Color("e8eef8"))
	UI.fill(head, 26)
	arrow.add_child(head)
	if not enabled:
		head.modulate.a = 0.35
	arrow.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	arrow.disabled = not enabled
	return arrow


## One place on the page: a full card, or a hero in the hero frame.
func _tile(entry: Variant) -> Button:
	var size: Vector2 = CardView.SIZES[CardView.Mode.LARGE] * card_scale
	var tile := Button.new()
	UI.bare(tile)
	tile.custom_minimum_size = size
	if entry is Dictionary:
		var name_height := 44.0
		var hero := HeroCard.framed(entry, size.y - name_height)
		hero.position = Vector2((size.x - hero.custom_minimum_size.x) / 2.0, 0)
		tile.add_child(hero)
		var name_label := UI.centered(String(entry["name"]), 26, PAGE_INK, false)
		name_label.position = Vector2(-20, size.y - name_height)
		name_label.size = Vector2(size.x + 40, name_height)
		tile.add_child(name_label)
	else:
		tile.add_child(_card(int(entry), card_scale))
	UI.quiet(tile)
	tile.pressed.connect(_open_entry.bind(entry))
	return tile


## A full-size card drawn at `scale`, inside a box of the scaled size.
func _card(index: int, scale_by: float, golden := false) -> Control:
	var holder := Control.new()
	holder.custom_minimum_size = CardView.SIZES[CardView.Mode.LARGE] * scale_by
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var card := CardView.new([{"def": index, "golden": golden}], CardView.Mode.LARGE, {"still": true, "cost": true, "summary": scale_by < 1.0})
	card.scale = Vector2(scale_by, scale_by)
	holder.add_child(card)
	return holder


func _open_entry(entry: Variant) -> void:
	_open = entry
	_golden = false
	_build()


func _flip_golden() -> void:
	_golden = not _golden
	_build()


func _close() -> void:
	_open = null
	_build()


## The opened card or hero: large, with its description, over a dimmed page.
## A press anywhere closes it.
func _add_opened() -> void:
	var dim := Button.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.offset_left = -200
	dim.offset_right = 200
	dim.offset_top = -200
	dim.offset_bottom = 200
	UI.face(dim, UI.box(Color(0.02, 0.02, 0.05, 0.84), 0, 0), 0.0)
	dim.pressed.connect(_close)
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(centre)
	var line := UI.row(36)
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	centre.add_child(line)
	if _open is Dictionary:
		line.add_child(HeroCard.framed(_open, 860))
		line.add_child(HeroCard.facts(_open, "Starting health %d" % int(_open["hp"]), 620))
	else:
		var def: Dictionary = Store.card(int(_open))
		var golden: bool = _golden and def["can_golden"]
		# With a note on what the two letters on its seal stand for.
		line.add_child(UI.with_kind_note(_card(int(_open), open_scale, golden), def))
		var beside := UI.column(18)
		beside.alignment = BoxContainer.ALIGNMENT_CENTER
		beside.mouse_filter = Control.MOUSE_FILTER_IGNORE
		beside.add_child(_card_facts(def, golden))
		if def["can_golden"]:
			# A quick look at what three copies make.
			beside.add_child(UI.stamp("Show normal" if golden else "Show golden", _flip_golden, Vector2(0, 110), 38, UI.INK))
		line.add_child(beside)


## Everything the content file says about a card, in words.
func _card_facts(def: Dictionary, golden := false) -> Control:
	var sheet := UI.panel(UI.INK_RAISED, 16, 22)
	sheet.custom_minimum_size.x = 640
	sheet.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	sheet.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var stack := UI.column(12)
	sheet.add_child(stack)
	stack.add_child(UI.label(UI.card_name(def) + (" (golden)" if golden else ""), 52, UI.GILT))
	var facts: Array[String] = []
	facts.append("No tier" if int(def["tier"]) == 0 else "Tier %s" % UI.roman(int(def["tier"])))
	facts.append(String(def["kind"]).capitalize())
	if def.get("category") != null:
		facts.append(UI.category_label(def["category"]))
	stack.add_child(UI.label("  ·  ".join(facts), 30, UI.SLATE))
	stack.add_child(UI.label(UI.card_text(def, golden), 36))
	if golden:
		stack.add_child(UI.label("Made from three copies. Making one also gives a free card from the next tier.", 28, UI.PAPER_SHADE))
	elif not def["can_golden"]:
		stack.add_child(UI.label("This card has no golden version.", 28, UI.PAPER_SHADE))
	stack.add_child(UI.label("Costs %d gold." % int(def["cost"]), 30, UI.PAPER_SHADE))
	if def.has("copies"):
		var copies := int(def["copies"])
		stack.add_child(UI.label("%d %s in the shared pool." % [copies, "copy" if copies == 1 else "copies"], 30, UI.PAPER_SHADE))
	stack.add_child(UI.label("Press anywhere to close.", 26, UI.SLATE))
	return sheet
