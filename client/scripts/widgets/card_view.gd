## One card, drawn from its data in four sizes (docs/ui-research.md 3.1).
## The face is a placeholder: a tint by type, a two-letter seal and the name
## printed where the illustration will go.
class_name CardView
extends Button

enum Mode { TOKEN, SMALL, BOARD, LARGE }

const SIZES := {
	Mode.TOKEN: Vector2(88, 88),
	Mode.SMALL: Vector2(172, 196),
	Mode.BOARD: Vector2(200, 228),
	Mode.LARGE: Vector2(440, 610),
}

## The size of a card on this screen: small cards grow with `UI.zoom`, the
## large card (read from close up, in a sheet) does not.
static func size_of(mode: Mode) -> Vector2:
	var base: Vector2 = SIZES[mode]
	return base if mode == Mode.LARGE else (base * UI.zoom).round()


## Where things sit on the painted frames in `assets/frames/`, measured on a
## frame 900 wide. The three frames share one geometry.
const FRAME_WIDTH := 900.0
const FRAME_WINDOW := Rect2(172, 197, 580, 572)
const FRAME_NAME := Rect2(205, 84, 528, 96)
const FRAME_TEXT := Rect2(160, 850, 610, 190)
## Seal centres: cost, tier, kind, value.
const FRAME_SEALS: Array[Vector2] = [Vector2(95, 88), Vector2(810, 102), Vector2(116, 1153), Vector2(806, 1153)]
## Text colour on each seal, per frame.
const FRAME_INKS := {
	"silver": [UI.INK, UI.PAPER, UI.INK, UI.INK],
	"gold": [UI.INK, UI.INK, UI.INK, UI.INK],
	"galaxy": [UI.INK, UI.PAPER, UI.PAPER, UI.PAPER],
}
## Card kinds that get the galaxy frame, besides cards with a single copy in the pool.
const GALAXY_KINDS := ["endgame", "legendary"]
const FRAME_INK := Color("2b1d12")
## The reward card that stands for an unpicked golden reward. It is not a
## golden card, but it is drawn gilt, on the gold frames.
const REWARD_CARD := "golden_reward"

## The painted oval frames for small cards (`assets/frames/oval2_*.png`),
## measured on a frame 400 wide: the window and the centre of the shield.
const OVAL_WIDTH := 400.0
const OVAL_WINDOW := Rect2(76, 46, 248, 316)
## The part of the window the bottom shield leaves free, for the picture.
const OVAL_PICTURE := Rect2(92, 66, 216, 206)
const OVAL_SHIELD := Vector2(199, 356)

## The card's parts: [{"def": int, "golden": bool}]. Combined cards have two.
var parts: Array = []
## What a drag of this card carries. Empty: the card cannot be dragged.
var drag_data: Dictionary = {}
## The drop zone this card sits in, so a drop on the card counts as a drop on the zone.
var drop_proxy: UI.DropZone
## One line describing the card, for the caption strip.
var caption := ""
## The card, or one of its parts, is golden.
var _golden := false


static func of_def(def_id: Variant, mode: Mode, options := {}) -> CardView:
	return CardView.new([{"def": int(def_id), "golden": false}], mode, options)


static func of_instance(instance: Dictionary, mode: Mode, options := {}) -> CardView:
	var merged := options.duplicate()
	merged["armor_left"] = int(instance["armor_left"])
	merged["disabled"] = bool(instance["disabled"])
	return CardView.new(instance["parts"], mode, merged)


## Options: cost (bool), armor_left (int), disabled (bool), selected (bool),
## chosen (bool, the endgame effect in use), still (bool, not touchable),
## summary (bool: a large card shown small, so a short name in big letters
## and no rules text), cost_text (String: stands on the cost seal as it is).
func _init(card_parts: Array, mode: Mode, options: Dictionary) -> void:
	parts = card_parts
	var def: Dictionary = Store.card(parts[0]["def"])
	var golden := false
	for part in parts:
		golden = golden or bool(part["golden"]) or String(Store.card(part["def"]).get("id", "")) == REWARD_CARD
	_golden = golden
	var kind: Array = UI.KINDS.get(String(def["kind"]), ["??", UI.SLATE])
	var tint: Color = kind[1]
	if def.get("category") != null:
		tint = UI.CATEGORY_TINTS[int(def["category"])]

	custom_minimum_size = size_of(mode)
	size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	size_flags_vertical = Control.SIZE_SHRINK_CENTER
	caption = _caption(def, golden)

	var paper := UI.INK if golden else UI.PAPER
	var ink := UI.GILT if golden else UI.INK
	var border := UI.GILT if golden else UI.PAPER_SHADE
	var width := 5
	if options.get("chosen", false):
		border = UI.VERDIGRIS
		width = 7
	if options.get("selected", false):
		border = UI.VERDIGRIS
		width = 9
	var disabled: bool = options.get("disabled", false)

	var body: Control
	var frame_name := "gold" if golden else ("galaxy" if int(def.get("copies", 0)) == 1 or GALAXY_KINDS.has(String(def["kind"])) else "silver")
	var frame := UI.art("frames/%s.png" % frame_name) if mode == Mode.LARGE else null
	if frame != null:
		UI.bare(self)
		body = _painted(def, kind, tint, frame, frame_name, options)
	elif mode == Mode.LARGE:
		UI.face(self, UI.box(paper, 40, 16, border, width))
		body = _large(def, kind, tint, ink, golden, options)
	elif mode == Mode.TOKEN:
		UI.bare(self)
		body = Control.new()
		body.add_child(_framed("disc", tint.darkened(0.35), UI.GILT if golden else UI.SLATE, 4 if golden else 3, false, disabled))
		var marks := _token(def, kind, options)
		UI.fill(marks)
		body.add_child(marks)
	else:
		UI.bare(self)
		var oval := UI.art("frames/oval2_%s.png" % frame_name)
		if oval != null:
			body = _cameo(def, tint, oval, golden, mode, options)
		else:
			body = _small(def, kind, tint, border, width, golden, mode, options)
	body.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(body)
	if mode == Mode.LARGE and disabled:
		add_child(UI.Hatch.new(Color(0.08, 0.1, 0.18, 0.55)))
	UI.quiet(self)
	if mode == Mode.TOKEN or options.get("still", false):
		mouse_filter = Control.MOUSE_FILTER_IGNORE
	elif mode != Mode.LARGE:
		# Holding a small card shows it large, with its text.
		var zoomed := options.duplicate()
		zoomed["still"] = true
		Peek.attach(self, func() -> Control:
			return UI.with_kind_note(CardView.new(parts, Mode.LARGE, zoomed), Store.card(int(parts[0]["def"]))))


## The silhouette of a card: fill, optional hatch cut to the shape, an inner
## ring for golden cards, then the border on top.
func _framed(shape: String, fill: Color, border: Color, width: float, golden: bool, disabled: bool) -> Control:
	var frame := Control.new()
	frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
	UI.fill(frame)
	var face := UI.Shape.new(shape, fill, Color.TRANSPARENT, 0.0, disabled)
	frame.add_child(face)
	if disabled:
		face.add_child(UI.Hatch.new(Color(0.08, 0.1, 0.18, 0.6)))
	if golden:
		var ring := UI.Shape.new(shape, Color.TRANSPARENT, Color(UI.GILT, 0.7), 2.0)
		UI.fill(ring, 11)
		frame.add_child(ring)
	frame.add_child(UI.Shape.new(shape, Color.TRANSPARENT, border, width))
	return frame


func _caption(def: Dictionary, golden: bool) -> String:
	var names: Array[String] = []
	var texts: Array[String] = []
	for part in parts:
		var part_def: Dictionary = Store.card(part["def"])
		names.append(UI.card_name(part_def) + (" (golden)" if part["golden"] else ""))
		texts.append(UI.card_text(part_def, part["golden"]))
	var tier := "" if int(def["tier"]) == 0 else "  ·  tier %s" % UI.roman(int(def["tier"]))
	var note := ""
	return "%s%s  ·  %s\n%s%s" % [" + ".join(names), tier, String(def["kind"]), "  /  ".join(texts), note]


func _wrap(inner: Control, margin: int) -> MarginContainer:
	var holder := MarginContainer.new()
	for side in ["margin_left", "margin_right", "margin_top", "margin_bottom"]:
		holder.add_theme_constant_override(side, margin)
	holder.add_child(inner)
	return holder


func _value(def: Dictionary, options: Dictionary) -> String:
	if int(options.get("armor_left", 0)) > 0:
		return str(int(options["armor_left"]))
	return UI.card_value(def, _golden)


func _token(def: Dictionary, kind: Array, options: Dictionary) -> Control:
	var stack := UI.column(0)
	stack.alignment = BoxContainer.ALIGNMENT_CENTER
	var mark := String(kind[0])
	if def.get("category") != null:
		mark = UI.CATEGORY_MARKS[int(def["category"])]
	stack.add_child(UI.centered(mark, 30, UI.PAPER, false))
	stack.add_child(UI.centered(_value(def, options), 26, UI.PAPER, false))
	return stack


func _top_row(def: Dictionary, ink: Color, golden: bool, size: int, options: Dictionary) -> HBoxContainer:
	var top := UI.row(4)
	if options.get("cost", false):
		var coin := UI.panel(UI.GILT, size, 2)
		coin.custom_minimum_size = Vector2(size * 1.5, size * 1.5)
		coin.add_child(UI.centered(str(int(def["cost"])), size, UI.INK, false))
		top.add_child(coin)
	var middle := UI.centered("golden" if golden else "", int(size * 0.8), UI.GILT, false)
	middle.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(middle)
	if int(def["tier"]) > 0:
		var tab := UI.panel(ink, 6, 2)
		tab.custom_minimum_size = Vector2(size * 1.8, size * 1.5)
		tab.add_child(UI.centered(UI.roman(int(def["tier"])), size, UI.INK if golden else UI.PAPER, false))
		top.add_child(tab)
	return top


## A small card in its painted oval frame: the picture (or a tint and the
## name) in the window, the tier on the shield, cost and value on the rim.
func _cameo(def: Dictionary, tint: Color, frame: Texture2D, golden: bool, mode: Mode, options: Dictionary) -> Control:
	var card := size_of(mode)
	var k := card.x / OVAL_WIDTH
	var big := mode == Mode.BOARD
	var root := Control.new()
	var disabled: bool = options.get("disabled", false)

	var window := Control.new()
	window.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_place(window, OVAL_WINDOW, k)
	var picture := UI.art("cards/%s.jpg" % String(def.get("id", "")))
	var face := UI.Shape.new("oval", tint.darkened(0.45) if golden else tint.lerp(UI.INK, 0.25), Color.TRANSPARENT, 0.0, disabled)
	if picture != null:
		face.texture = picture
		face.color = Color.WHITE
	window.add_child(face)
	if disabled:
		face.add_child(UI.Hatch.new(Color(0.04, 0.05, 0.1, 0.7)))
	if options.get("selected", false) or options.get("chosen", false):
		window.add_child(UI.Shape.new("oval", Color.TRANSPARENT, UI.VERDIGRIS, 6.0))
	var emblem := _emblem(def) if picture == null else null
	if emblem == null and picture == null:
		var names: Array[String] = []
		for part in parts:
			names.append(UI.card_name(Store.card(part["def"])))
		var title := UI.centered(" + ".join(names), UI.z(22 if big else 20), UI.PAPER)
		title.autowrap_mode = TextServer.AUTOWRAP_WORD
		title.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		UI.fill(title)
		title.offset_left = 8
		title.offset_right = -8
		title.offset_top = 22
		title.offset_bottom = -22 - (OVAL_WINDOW.end.y - OVAL_PICTURE.end.y) * k
		window.add_child(title)
	root.add_child(window)
	if emblem != null:
		var image := _emblem_image(emblem, 0)
		image.set_anchors_preset(Control.PRESET_TOP_LEFT)
		_place(image, OVAL_PICTURE, k)
		root.add_child(image)

	var ring := TextureRect.new()
	ring.texture = frame
	ring.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	ring.stretch_mode = TextureRect.STRETCH_SCALE
	UI.fill(ring)
	root.add_child(ring)

	if int(def["tier"]) > 0:
		var tier := UI.centered(UI.roman(int(def["tier"])), UI.z(34 if big else 30), UI.PAPER, false)
		_place(tier, Rect2(OVAL_SHIELD - Vector2(50, 40), Vector2(100, 80)), k)
		root.add_child(tier)
	if options.get("cost", false):
		root.add_child(UI.token("coin", str(int(def["cost"])), UI.z(56), UI.z(32)))
	if parts.size() > 1:
		var count := UI.badge("x%d" % parts.size(), UI.z(38), UI.INK, UI.PAPER, UI.z(19), UI.PAPER_SHADE)
		count.position = Vector2(0, card.y * 0.42)
		root.add_child(count)
	var number := _value(def, options)
	if number != "":
		var pill := PanelContainer.new()
		pill.mouse_filter = Control.MOUSE_FILTER_IGNORE
		pill.add_theme_stylebox_override("panel", UI.box(UI.INK, 23, 5, UI.GILT if golden else UI.PAPER_SHADE, 3))
		pill.custom_minimum_size = Vector2(UI.z(52), UI.z(52))
		pill.add_child(UI.centered(number, UI.z(26 if big else 24), UI.PAPER, false))
		pill.set_anchors_preset(Control.PRESET_TOP_RIGHT)
		pill.grow_horizontal = Control.GROW_DIRECTION_BEGIN
		root.add_child(pill)
	return root


## Shop and hand cards are arched plates; a card in play is an oval cameo.
## Cost, tier, kind and value sit as seals on the rim.
func _small(def: Dictionary, kind: Array, tint: Color, border: Color, width: int, golden: bool, mode: Mode, options: Dictionary) -> Control:
	var big := mode == Mode.BOARD
	var card := size_of(mode)
	var root := Control.new()
	var fill := tint.darkened(0.5) if golden else tint.lerp(UI.PAPER, 0.45)
	var frame := _framed("oval" if big else "arch", fill, border, width, golden, options.get("disabled", false))
	frame.offset_left = 4
	frame.offset_right = -4
	frame.offset_top = 16
	frame.offset_bottom = -18
	root.add_child(frame)

	var names: Array[String] = []
	for part in parts:
		names.append(UI.card_name(Store.card(part["def"])))
	var title := UI.centered(" + ".join(names), 20 if big else 19, UI.PAPER if golden else UI.INK)
	title.autowrap_mode = TextServer.AUTOWRAP_WORD
	UI.fill(title)
	title.offset_left = 20 if big else 12
	title.offset_right = -title.offset_left
	title.offset_top = 52
	title.offset_bottom = -60
	title.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	root.add_child(title)

	if options.get("cost", false):
		root.add_child(UI.badge(str(int(def["cost"])), 52, UI.GILT, UI.INK, 30, UI.INK))
	if int(def["tier"]) > 0:
		var tab := UI.badge(UI.roman(int(def["tier"])), 44, UI.GILT if golden else UI.INK, UI.INK if golden else UI.PAPER, 22, UI.PAPER_SHADE)
		tab.position = Vector2((card.x - 44) / 2.0, 0)
		root.add_child(tab)
	var seal := UI.badge(String(kind[0]), 48, tint.darkened(0.25), UI.PAPER, 22, UI.PAPER)
	seal.position = Vector2(0, card.y - 52)
	root.add_child(seal)
	if parts.size() > 1:
		var count := UI.badge("x%d" % parts.size(), 40, UI.INK, UI.PAPER, 20)
		count.position = Vector2((card.x - 40) / 2.0, card.y - 44)
		root.add_child(count)

	var number := _value(def, options)
	if number != "":
		var pill := PanelContainer.new()
		pill.mouse_filter = Control.MOUSE_FILTER_IGNORE
		pill.add_theme_stylebox_override("panel",
			UI.box(UI.GILT if golden else UI.INK, 28, 6, UI.INK if golden else UI.PAPER_SHADE, 3))
		pill.custom_minimum_size = Vector2(56, 56)
		pill.add_child(UI.centered(number, 28 if big else 26, UI.INK if golden else UI.PAPER, false))
		pill.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
		pill.grow_horizontal = Control.GROW_DIRECTION_BEGIN
		pill.grow_vertical = Control.GROW_DIRECTION_BEGIN
		root.add_child(pill)
	return root


## The zoomed card on a painted frame: the picture behind the window, the
## name on the banner, the rules on the parchment, numbers on the four seals.
func _painted(def: Dictionary, kind: Array, tint: Color, frame: Texture2D, frame_name: String, options: Dictionary) -> Control:
	var k: float = SIZES[Mode.LARGE].x / FRAME_WIDTH
	var root := Control.new()

	var picture := UI.art("cards/%s.jpg" % String(def.get("id", "")))
	var window := Control.new()
	_place(window, FRAME_WINDOW, k)
	if picture != null:
		var image := TextureRect.new()
		image.texture = picture
		image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		UI.fill(image)
		window.add_child(image)
	else:
		var plain := ColorRect.new()
		plain.color = tint.lerp(UI.INK, 0.35)
		UI.fill(plain)
		window.add_child(plain)
		var mark := String(kind[0])
		if def.get("category") != null:
			mark = UI.category_label(def["category"])
		var stand_in := UI.centered(mark, 56 if mark.length() <= 2 else 34, UI.PAPER)
		UI.fill(stand_in, 20)
		window.add_child(stand_in)
		var emblem := _emblem(def)
		if emblem != null:
			stand_in.visible = false
			window.add_child(_emblem_image(emblem, 14))
	root.add_child(window)

	var face := TextureRect.new()
	face.texture = frame
	face.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	face.stretch_mode = TextureRect.STRETCH_SCALE
	UI.fill(face)
	root.add_child(face)

	var names: Array[String] = []
	var texts: Array[String] = []
	for part in parts:
		var part_def: Dictionary = Store.card(part["def"])
		names.append(UI.card_name(part_def))
		texts.append(UI.card_text(part_def, part["golden"]))
	if int(options.get("armor_left", 0)) > 0:
		texts.append("Armor left: %d" % int(options["armor_left"]))
	if options.get("disabled", false):
		texts.append("Disabled this battle")
	var title_text := " + ".join(names)
	var summary: bool = options.get("summary", false)
	var title_size := 32 if title_text.length() <= 14 else (25 if title_text.length() <= 22 else 20)
	var full_name := title_text
	if summary:
		# Shown small: the banner is too narrow for a name, so it carries the
		# one number that matters (or the kind) and the name goes on the
		# parchment below, where two big lines fit.
		title_text = _value(def, options)
		if title_text == "":
			title_text = "Reward" if String(def.get("id", "")) == REWARD_CARD else String(def["kind"]).capitalize()
		title_size = 38
	var title := UI.centered(title_text, title_size, FRAME_INK)
	title.autowrap_mode = TextServer.AUTOWRAP_WORD
	if UI.display_font() != null:
		title.add_theme_font_override("font", UI.display_font())
	_place(title, FRAME_NAME, k)
	root.add_child(title)
	var rules_text := "" if summary else "\n".join(texts)
	var rules_size := 22 if rules_text.length() <= 90 else 18
	if summary:
		rules_text = full_name
		rules_size = 40 if full_name.length() <= 12 else (34 if full_name.length() <= 26 else 28)
	var rules := UI.centered(rules_text, rules_size, FRAME_INK)
	if summary and UI.display_font() != null:
		rules.add_theme_font_override("font", UI.display_font())
	rules.autowrap_mode = TextServer.AUTOWRAP_WORD
	_place(rules, FRAME_TEXT, k)
	root.add_child(rules)

	var inks: Array = FRAME_INKS[frame_name]
	var seals: Array[String] = [
		String(options.get("cost_text", str(int(def["cost"])) if options.get("cost", false) else "")),
		UI.roman(int(def["tier"])) if int(def["tier"]) > 0 else "",
		String(kind[0]),
		_value(def, options),
	]
	var sizes: Array[int] = [34, 34, 36, 26]
	for index in seals.size():
		if seals[index] == "":
			continue
		var words := UI.centered(seals[index], sizes[index], inks[index], false)
		_place(words, Rect2(FRAME_SEALS[index] - Vector2(70, 70), Vector2(140, 140)), k)
		root.add_child(words)
	return root


## The card's emblem: its category's from `assets/categories/`, or its kind's from `assets/kinds/`.
func _emblem(def: Dictionary) -> Texture2D:
	if def.get("category") == null:
		# Other cards: a picture of their own (`kinds/<id>.png`), else their kind's.
		var own := UI.art("kinds/%s.png" % String(def.get("id", "")))
		return own if own != null else UI.art("kinds/%s.png" % String(def["kind"]))
	return UI.art("categories/%s.png" % String(Store.categories[int(def["category"])]))


func _emblem_image(emblem: Texture2D, margin: int) -> TextureRect:
	var image := TextureRect.new()
	image.texture = emblem
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.mouse_filter = Control.MOUSE_FILTER_IGNORE
	UI.fill(image, margin)
	return image


## Puts `node` on a rectangle given in frame pixels.
func _place(node: Control, rect: Rect2, k: float) -> void:
	node.position = rect.position * k
	node.size = rect.size * k


func _large(def: Dictionary, kind: Array, tint: Color, ink: Color, golden: bool, options: Dictionary) -> Control:
	var stack := UI.column(10)
	stack.add_child(_top_row(def, ink, golden, 36, options))

	var art := Control.new()
	art.custom_minimum_size = Vector2(0, 170)
	art.add_child(UI.Shape.new("oval", tint.lerp(UI.PAPER, 0.35) if not golden else tint.darkened(0.45), ink, 4.0))
	var mark := String(kind[0])
	if def.get("category") != null:
		mark = UI.category_label(def["category"])
	var art_text := UI.centered(mark, 56 if mark.length() <= 2 else 36, UI.PAPER if golden else UI.INK)
	UI.fill(art_text, 40)
	art.add_child(art_text)
	stack.add_child(art)

	for part in parts:
		var part_def: Dictionary = Store.card(part["def"])
		stack.add_child(UI.label(UI.card_name(part_def), 40 if parts.size() == 1 else 34, ink))
		var text := UI.card_text(part_def, part["golden"])
		var rules := UI.label(text, 30 if parts.size() == 1 else 26, ink)
		rules.size_flags_vertical = Control.SIZE_EXPAND_FILL
		stack.add_child(rules)
	if int(options.get("armor_left", 0)) > 0:
		stack.add_child(UI.label("Armor left: %d" % int(options["armor_left"]), 30, ink))
	var kind_line := String(def["kind"]).capitalize()
	if options.get("disabled", false):
		kind_line += "  ·  disabled this battle"
	stack.add_child(UI.label(kind_line, 28, UI.SLATE if golden else UI.VERMILION_DEEP))
	return _wrap(stack, 16)


func _get_drag_data(_at: Vector2) -> Variant:
	if drag_data.is_empty():
		return null
	var ghost := CardView.new(parts, Mode.SMALL, {"still": true})
	ghost.modulate.a = 0.85
	ghost.rotation_degrees = 4.0
	# Drawn up and to the side of the finger, so the face stays visible.
	ghost.position = Vector2(-86, -214) * UI.zoom
	var holder := Control.new()
	holder.add_child(ghost)
	set_drag_preview(holder)
	return drag_data


func _can_drop_data(at: Vector2, data: Variant) -> bool:
	return drop_proxy != null and drop_proxy._can_drop_data(at, data)


func _drop_data(at: Vector2, data: Variant) -> void:
	drop_proxy._drop_data(at, data)
