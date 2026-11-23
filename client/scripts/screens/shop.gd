## The shop phase, laid out on the painted board (`backgrounds/arena.jpg`):
## the player list on the left, and on the right edge the timer, the three
## shop buttons and the gold. Between them three bands, each a share of the
## screen's height: the shop row, the board row, and the desk with the hero,
## the ability and the hand.
##
## The board has no drawn places: its cards stand in a centred line, and a
## card from the hand held over it opens a gap where it is pointed.
##
## The hand rests at the bottom edge beside the hero, as a fan of its cards
## half off the screen. A tap raises it over the desk; there a card is held
## to read it, tapped for its actions and dragged up to place it.
##
## Buy is a drag down from the shop (onto the board row or anywhere on the foot), place a drag up from the hand, sell a
## drag up to the sign. Every drag has a tap equivalent: tap a card to
## open it, then use the buttons beside it. Holding anything shows its details.
##
## The screen knows no rules. It offers actions and shows the host's answer
## when one is refused. The two checks that still read card data to decide
## which button to show are in `_card_actions`, and go away when the view
## lists the allowed actions itself (view addition V6).
extends Control

@export var toast_ms := 2000
## Size of the hero portrait at the bottom centre.
@export var hero_size := 236
## Size of the round ability button beside the hero.
@export var ability_size := 160
## The painted borders (`board/top.png`, `board/bottom.png`) are drawn over
## the board. Off: the board is the plain background, and the hero, the
## ability and the hand stand on it by themselves.
@export var use_borders := false
## Where the shop row and the board row begin and end, and where the desk
## begins, as shares of the screen's height.
@export var shop_band := Vector2(0.04, 0.30)
@export var board_band := Vector2(0.33, 0.60)
@export var desk_top := 0.63
## The distance between cards in both rows, as a share of the screen's
## height; never more than the field's width shared among `row_fit` cards,
## and never so little that two cards touch while there is room.
@export var row_spacing := 0.20
@export var row_fit := 7
@export var row_min_gap := 6
## A card from the hand held over the board: how far the cards beside it
## move away, as a share of the distance between cards, and how fast.
@export var row_gap_share := 0.5
@export var row_gap_seconds := 0.12
## The board while a card is held over it, while it refuses one, and how
## strongly its hint and its count show.
@export var board_lit := Color(0.62, 0.8, 1.0)
@export var board_refuses := Color(0.95, 0.35, 0.25)
@export var board_hint_alpha := 0.4
## The hand at rest, at the bottom edge beside the hero: the width of a
## card and of the whole fan as shares of the screen's height, how much of a
## card is below the screen, the distance between cards as a share of a
## card's width, how far the fan opens, and from how many cards it carries
## their number.
@export var rest_card := 0.14
@export var rest_width := 0.43
@export var rest_sunk := 0.4
@export var rest_spread := 0.6
@export var rest_arc := 24.0
@export var rest_badge_from := 8
## The raised hand: the width of a card, where the cards' tops stand and the
## distance between cards, as shares of the screen's height, and how far the
## fan opens.
@export var raised_card := 0.27
@export var raised_top := 0.70
@export var raised_spacing := 0.185
@export var raised_arc := 24.0
## How dark the desk goes behind the raised hand, and how much of the way
## down from the board the darkness takes to set in.
@export var fan_dim := 0.5
@export var fan_dim_edge := 0.22
## Without the borders: the gap between the hero and its two neighbours, how
## far the hero stands from the bottom of the board, and how far the ability
## sits below the hero's middle.
@export var hero_gap := 26
@export var hero_raise := 24
@export var ability_drop := 30
## Without the borders: the shadow the hero, the ability and the hand stand
## on, as shares of the hero's size (width, height), and how dark it is.
@export var plinth := Vector2(2.3, 0.2)
@export var plinth_alpha := 0.5
## Room kept free on both sides of the centre column, for the rail and the side column.
@export var gutter := 176
@export var side_width := 156
## Height of each shop button in the right column, and the gap between its items.
@export var side_button := 150
@export var side_gap := 18
## The player list on the left: its portraits grow as the lobby is smaller,
## up to this size, and never beyond what six of them need to fit.
@export var rail_max := 190
@export var rail_gap := 12

## Lines about the last battle, shown in the caption strip.
var last_battle: Array = []

## The card open in the inspect panel: {"zone": "shop" | "extra" | "hand" | "board", "index": int}.
var _selected := {}
## "", "darwin", "combine", "confirm" or "record".
var _mode := ""
var _mode_data: Variant = null
## Benz: the first card of a combine, waiting for the second.
var _combine_first := -1
## Nietzsche: house categories picked so far.
var _house_picks: Array[int] = []
## The reward or the house picker was put aside to look at the board.
var _put_aside := false
## The hand is opened as a large fan over the board; a small fan otherwise.
var _hand_open := false
## The raised hand, while it is up, and the hand at rest.
var _hand_layer: Control
var _rest_fan: HandFan
## The board's zone, its row of cards and its hint.
var _board_zone: UI.DropZone
var _board_row: CardRow
var _board_hint: Label
## What the board shows for the card held over it: "" (nothing), "lit" or
## "full", and the gap it has opened.
var _board_state := ""
var _board_gap := -1
## Where the pointer was last seen.
var _pointer := Vector2.ZERO
## A card is being dragged out of the opened hand, which folds for it.
var _hand_dragged := false
## The shop clock ticks over this many last seconds.
@export var tick_from := 10
var _ticked := -1
## A finger or the mouse button is down. The screen is not rebuilt meanwhile:
## a rebuild would throw away the card under the finger and end its drag or
## hold. A change that comes in meanwhile is drawn on release (`_stale`).
var _pressed := false
var _stale := false
## The opened card shows its golden version, as a preview.
var _preview_golden := false
## The colour of ice, for a reserved shop.
@export var frozen_color := Color("7fd4ff")
## The shop was reserved the last time the screen was built, so a change
## plays as ice growing or melting.
var _was_frozen := false
var _toast := ""
var _toast_until := 0
var _idle_caption := ""

var _caption: Label
## The full-width borders (`board/top.png`, `board/bottom.png`), used when
## `use_borders` is on. They are drawn the width of the screen, keeping their
## shape; the top one is lifted this far off the screen, and the bottom one
## raised this far (a negative drop).
@export var top_border_lift := 40
@export var bottom_border_drop := -50
## Room left between the lowest point of the top border and the shop row.
@export var top_border_clear := 6
## With the borders: the size of the hero as a share of the arch's height,
## of the ability as a share of the round socket's width, and how far above
## the tray's floor the hand rests, as a share of the border's height.
@export var arch_fill := 1.08
@export var socket_fill := 1.16
@export var tray_rest := 0.02
## Where the sockets are on `board/bottom.png` and how low `board/top.png`
## reaches, in pixels of those pictures: the opening of the arch, the
## opening of the round socket, the floor of the tray, and the lowest point
## of the top border.
const ART_ARCH := Rect2(1082, 53, 235, 253)
const ART_SOCKET := Rect2(800, 208, 139, 138)
const ART_TRAY := Rect2(1570, 213, 600, 30)
const ART_TOP_REACH := 326.0
## The plaques behind the two side columns: how far they reach past them.
@export var plaque_side := 0.2
@export var plaque_ends := 56
@export var plaque_alpha := 0.9
## The lengths above as written for a 16:9 screen; `_apply_zoom` enlarges
## the ones that grow with the board on a wider screen.
const GROWS: Array[String] = ["hero_size", "ability_size", "gutter", "side_width", "side_button", "rail_max", "hero_gap", "ability_drop"]
var _drawn_for := {}
var _timer: Label
var _sell_zone: UI.DropZone
var _buy_zone: UI.DropZone
## The size of the board the screen was last built for.
var _built_for := Vector2.ZERO


func _ready() -> void:
	# A shop that opens reserved shows its ice at once, without growing it.
	_was_frozen = bool(Store.me()["frozen"])
	refresh()
	# Everything below the rows is placed from the size of the board, so a
	# board that changes size is built again.
	resized.connect(func() -> void:
		if _board() != _built_for:
			refresh.call_deferred())


func _process(_delta: float) -> void:
	var deadline = Store.view.get("phase_deadline")
	if _timer != null:
		_timer.text = str(UI.seconds_left(deadline))
		_timer.add_theme_color_override("font_color", UI.VERMILION if UI.ms_left(deadline) < 10000 else UI.PAPER)
		# The clock ticks once a second over the last `tick_from` seconds of the shop.
		var second := UI.seconds_left(deadline) if deadline != null else 0
		if second >= 1 and second <= tick_from and second != _ticked:
			_ticked = second
			Sound.instance.play("tick")
	if _toast != "" and Time.get_ticks_msec() >= _toast_until:
		_toast = ""
		_show_caption(_idle_caption)
	_track_drag(false)


func _input(event: InputEvent) -> void:
	if event is InputEventMouse:
		_pointer = (event as InputEventMouse).position
	var press := -1
	if event is InputEventMouseButton and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
		press = 1 if event.pressed else 0
	elif event is InputEventScreenTouch and (event as InputEventScreenTouch).index == 0:
		press = 1 if event.pressed else 0
	if press == 1:
		_pressed = true
	elif press == 0:
		_pressed = false
		if _stale:
			_stale = false
			refresh.call_deferred()


func _notification(what: int) -> void:
	if what == NOTIFICATION_DRAG_BEGIN:
		if is_instance_valid(_sell_zone):
			_sell_zone.visible = _owned_card_in_flight()
		if is_instance_valid(_buy_zone):
			_buy_zone.visible = _card_in_flight(["shop", "extra"])
		# A card dragged out of the raised hand: the hand goes back to rest,
		# so the board and the shop row can take the card.
		if is_instance_valid(_hand_layer) and _card_in_flight(["hand"]):
			_hand_open = false
			_hand_dragged = true
			_hand_layer.modulate.a = 0.0
			if is_instance_valid(_rest_fan):
				_rest_fan.visible = true
			_hand_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
			for node in _hand_layer.find_children("*", "Control", true, false):
				(node as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	elif what == NOTIFICATION_DRAG_END:
		if _hand_dragged or _stale:
			_hand_dragged = false
			_stale = false
			refresh.call_deferred()
		if is_instance_valid(_sell_zone):
			_sell_zone.visible = false
		if is_instance_valid(_buy_zone):
			_buy_zone.visible = false


# ---- Commands ----------------------------------------------------------------

func _send(command: Dictionary) -> void:
	var refusal := Transport.send(command)
	if refusal != "":
		_say(refusal.capitalize())
	refresh()


## The host refused a command some time after it was sent (online play).
func show_refusal(text: String) -> void:
	_say(text)
	_show_caption(_idle_caption)


func _me_command(kind: String, fields := {}) -> void:
	var payload := fields.duplicate()
	payload["player"] = Store.my_id()
	_send({kind: payload})


func _ability(action: Variant) -> void:
	_mode = ""
	_me_command("UseAbility", {"action": action})


func _say(text: String) -> void:
	_toast = text
	_toast_until = Time.get_ticks_msec() + toast_ms


func _show_caption(text: String) -> void:
	if not is_instance_valid(_caption):
		return
	_caption.text = _toast if _toast != "" else text
	# The plate shows only while it has something to say.
	_caption.get_parent().get_parent().modulate.a = 0.0 if _caption.text == "" else 1.0
	_caption.add_theme_color_override("font_color", UI.VERMILION if _toast != "" else UI.PAPER)


## Opens a card in the inspect panel. Also used by the development autoplayer.
func inspect(zone: String, index: int) -> void:
	_selected = {"zone": zone, "index": index}
	_preview_golden = false
	refresh()


## Raises the hand. Also used by the development autoplayer.
func open_hand() -> void:
	_hand_open = true
	refresh()


## Lowers the hand to rest beside the hero.
func close_hand() -> void:
	_hand_open = false
	refresh()


func close_overlays() -> void:
	_selected = {}
	_mode = ""
	_combine_first = -1
	refresh()


# ---- Building ----------------------------------------------------------------

## Sets the sizes of this screen for the shape of the display.
func _apply_zoom() -> void:
	if _drawn_for.is_empty():
		for length in GROWS:
			_drawn_for[length] = get(length)
	for length in GROWS:
		set(length, UI.z(_drawn_for[length]))


## The size of this screen inside the frame's margins. Worked out from the
## window, so it is known before the screen is first laid out.
func _board() -> Vector2:
	var view := get_viewport_rect().size
	return Vector2(view.x - _margin("margin_left") - _margin("margin_right"), view.y - _margin("margin_top") - _margin("margin_bottom"))


func _margin(side: String) -> float:
	var frame := get_parent() as Control
	return float(frame.get_theme_constant(side)) if frame != null else 0.0


## The borders are wanted and their pictures are there.
func _bordered() -> bool:
	return use_borders and UI.art("board/top.png") != null and UI.art("board/bottom.png") != null


## How many screen pixels one pixel of a border picture takes.
func _border_scale(picture: Texture2D) -> float:
	return get_viewport_rect().size.x / float(picture.get_width())


## The height of the screen, which the bands and the hand are measured in.
func _height() -> float:
	return get_viewport_rect().size.y


## A height on the screen, given as a share of it, in this screen's own coordinates.
func _y(share: float) -> float:
	return share * _height() - _margin("margin_top")


## The field between the two side columns: where it begins, and its width.
func _field() -> Vector2:
	return Vector2(gutter, _board().x - 2.0 * gutter)


## Where the bands are: {"shop": top and bottom, "board": top and bottom,
## "desk": top}. With the borders the two rows start below the top border
## and close up, to stay above the bottom one.
func _bands() -> Dictionary:
	var shop := Vector2(_y(shop_band.x), _y(shop_band.y))
	var board := Vector2(_y(board_band.x), _y(board_band.y))
	if _bordered():
		var top := UI.art("board/top.png")
		var reach := ART_TOP_REACH * _border_scale(top) - _margin("margin_top") - UI.z(top_border_lift) + top_border_clear
		var tall: float = CardView.size_of(CardView.Mode.BOARD).y + 8.0
		shop = Vector2(reach, reach + tall)
		board = Vector2(shop.y + 8.0, shop.y + 8.0 + tall)
	return {"shop": shop, "board": board, "desk": _y(desk_top)}


func refresh() -> void:
	if _pressed or get_viewport().gui_is_dragging():
		_stale = true
		return
	_apply_zoom()
	UI.clear(self)
	var me := Store.me()
	var hero: Dictionary = Store.hero(me["hero"])
	if (me["rewards"] as Array).is_empty() and (me["hero_state"]["house_options"] as Array).is_empty():
		_put_aside = false
	_built_for = _board()
	if _bordered():
		_add_border("board/top.png", true)
		_add_border("board/bottom.png", false)

	_add_rows(me)
	var rail := _rail()
	rail.set_anchors_preset(Control.PRESET_LEFT_WIDE)
	_plaque_behind(rail)
	add_child(rail)
	var side := _side(me)
	side.set_anchors_preset(Control.PRESET_RIGHT_WIDE)
	side.offset_left = -side_width
	_plaque_behind(side)
	add_child(side)
	_add_desk(me, hero)
	_add_toast()
	_add_hand_view(me, hero)
	_add_overlay(me, hero)
	# A card may be held over the board while the screen is built again.
	_board_state = ""
	_board_gap = -1
	_track_drag(true)


## One of the two painted borders across the whole width of the screen, at
## its top or its bottom, behind everything else on the board. They reach
## past the safe frame to the screen's edges, as a painted board would.
func _add_border(file: String, top: bool) -> void:
	var picture := UI.art(file)
	if picture == null:
		return
	var piece := TextureRect.new()
	piece.texture = picture
	piece.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	piece.stretch_mode = TextureRect.STRETCH_SCALE
	piece.mouse_filter = Control.MOUSE_FILTER_IGNORE
	# The screen keeps a safe margin; the border reaches past it, so it is
	# placed from the margins the frame around this screen leaves.
	var frame := get_parent() as Control
	var left := float(frame.get_theme_constant("margin_left")) if frame != null else 0.0
	var right := float(frame.get_theme_constant("margin_right")) if frame != null else 0.0
	var edge := float(frame.get_theme_constant("margin_top" if top else "margin_bottom")) if frame != null else 0.0
	var height := picture.get_height() * _border_scale(picture)
	piece.set_anchors_preset(Control.PRESET_TOP_WIDE if top else Control.PRESET_BOTTOM_WIDE)
	piece.offset_left = -left
	piece.offset_right = right
	if top:
		piece.offset_top = -edge - UI.z(top_border_lift)
		piece.offset_bottom = piece.offset_top + height
	else:
		piece.offset_bottom = edge + UI.z(bottom_border_drop)
		piece.offset_top = piece.offset_bottom - height
	# First on the board, so everything else is drawn over it.
	add_child(piece)
	move_child(piece, 0)


## A painted plaque behind a side column, so its buttons or portraits stand
## on something. It follows the column's content, whatever that holds.
func _plaque_behind(column: BoxContainer) -> void:
	var picture := UI.art("board/side_panel.png")
	if picture == null:
		return
	var plaque := Plaque.new()
	plaque.texture = picture
	plaque.modulate.a = plaque_alpha
	plaque.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(plaque)
	column.sort_children.connect(func() -> void:
		var top := INF
		var bottom := -INF
		for child in column.get_children():
			if child is Control and child.visible:
				top = minf(top, child.position.y)
				bottom = maxf(bottom, child.position.y + child.size.y)
		if top > bottom:
			return
		var reach := column.size.x * plaque_side
		plaque.position = column.position + Vector2(-reach, top - UI.z(plaque_ends))
		plaque.size = Vector2(column.size.x + 2.0 * reach, bottom - top + 2.0 * UI.z(plaque_ends))
		plaque.queue_redraw())


## A tall painting drawn at any height: its two arched ends keep their shape
## and only a plain band of the middle is stretched.
class Plaque extends Control:
	var texture: Texture2D
	## The band of the picture that stretches, as fractions of its height.
	var band := Vector2(0.60, 0.66)

	func _draw() -> void:
		if texture == null:
			return
		var picture := texture.get_size()
		var k := size.x / picture.x
		var head := picture.y * band.x * k
		var foot := picture.y * (1.0 - band.y) * k
		if head + foot > size.y:
			draw_texture_rect(texture, Rect2(Vector2.ZERO, size), false)
			return
		draw_texture_rect_region(texture, Rect2(0, 0, size.x, head), Rect2(0, 0, picture.x, picture.y * band.x))
		draw_texture_rect_region(texture, Rect2(0, head, size.x, size.y - head - foot),
			Rect2(0, picture.y * band.x, picture.x, picture.y * (band.y - band.x)))
		draw_texture_rect_region(texture, Rect2(0, size.y - foot, size.x, foot),
			Rect2(0, picture.y * band.y, picture.x, picture.y * (1.0 - band.y)))


## The player list: six square icons, living players by health, the
## eliminated last. The order is the standing; the bar is the health.
## Holding one shows who it is.
func _rail() -> Control:
	var rail := UI.column(rail_gap)
	rail.alignment = BoxContainer.ALIGNMENT_CENTER
	var px := rail_size()
	rail.custom_minimum_size.x = px
	Tutorial.mark(rail, "rail")
	var players: Array = (Store.view["players"] as Array).duplicate()
	players.sort_custom(func(a, b): return _rank(a) < _rank(b))
	for player in players:
		var seat := int(player["id"])
		var own := seat == Store.my_id()
		var alive: bool = player["alive"]
		var entry := Button.new()
		entry.custom_minimum_size = Vector2(px, px)
		UI.bare(entry)
		var health := float(player["hp"]) / maxf(1.0, float(player["max_hp"])) if alive else 0.0
		var icon := UI.icon(Store.hero_of(seat), px, UI.GILT if own else UI.SLATE, 7 if own else 3, health)
		if not alive:
			icon.modulate = Color(0.5, 0.5, 0.5, 0.6)
		entry.add_child(icon)
		UI.quiet(entry)
		var hero := Store.hero_of(seat)
		if not hero.is_empty():
			Peek.attach(entry, HeroCard.info.bind(hero, _player_state(player)))
		rail.add_child(entry)
	return rail


## How large each portrait of the player list is: they share the height of
## the screen, so fewer players get larger portraits.
func rail_size() -> int:
	var count := maxi(1, (Store.view["players"] as Array).size())
	return mini(rail_max, int((size.y - rail_gap * (count - 1)) / count)) if size.y > 0 else 148


func _player_state(player: Dictionary) -> String:
	var who := Store.player_name(int(player["id"]))
	if not player["alive"]:
		return "%s  ·  eliminated" % who if player["placement"] == null else "%s  ·  eliminated, placed #%d" % [who, int(player["placement"])]
	return who + _build_hint(player) + "\nHP %d/%d\nArmor %d\nLevel %s" % [int(player["hp"]), int(player["max_hp"]), int(player["armor"]), UI.roman(int(player["tavern"]))]


## What everyone may know about a player's build and form, one fact per
## line: the category they hold most of, golden cards, win streak and the
## last two battles. The host sends these; nothing here is worked out.
func _build_hint(player: Dictionary) -> String:
	var lines: Array[String] = []
	var cards := int(player.get("top_category_cards", 0))
	if player.get("top_category") != null:
		lines.append("Mostly %s (%d)" % [UI.category_label(player["top_category"]), cards])
	elif cards > 0:
		lines.append("Mixed categories (%d each)" % cards)
	if int(player.get("goldens", 0)) > 0:
		lines.append("Golden cards %d" % int(player["goldens"]))
	if int(player.get("win_streak", 0)) > 1:
		lines.append("Won %d in a row" % int(player["win_streak"]))
	# Newest first, one battle per line.
	var fights: Array = (player.get("recent", []) as Array).duplicate()
	fights.reverse()
	for index in fights.size():
		var mine := int(fights[index]["correct"])
		var theirs := int(fights[index]["opponent_correct"])
		lines.append("%s: %s %d–%d" % ["Last battle" if index == 0 else "Before that",
			"won" if mine > theirs else ("lost" if mine < theirs else "tied"), mine, theirs])
	return "" if lines.is_empty() else "\n" + "\n".join(lines)


func _rank(player: Dictionary) -> float:
	if not player["alive"]:
		return 1000.0 + (0.0 if player["placement"] == null else float(player["placement"]))
	return -float(player["hp"])


## The shop row and the board row: two centred lines of cards of one size,
## the same distance apart, each in its own band of the screen.
func _add_rows(me: Dictionary) -> void:
	var bands := _bands()
	var field := _field()
	var card := CardView.size_of(CardView.Mode.BOARD)
	var spacing := minf(maxf(row_spacing * _height(), card.x + row_min_gap), field.y / row_fit)

	# Shop row: the offers, then a little apart the extra offers (potion, skull).
	var shop_band_at: Vector2 = bands["shop"]
	var offers := CardRow.new()
	offers.spacing = spacing
	offers.position = Vector2(field.x, shop_band_at.x)
	offers.size = Vector2(field.y, shop_band_at.y - shop_band_at.x)
	var wares: Array = []
	for index in (me["shop"] as Array).size():
		wares.append(_card(CardView.of_def(me["shop"][index], CardView.Mode.BOARD, {"cost": true}), "shop", index, -1, null))
	if not (me["extras"] as Array).is_empty():
		offers.split_at = wares.size()
		for index in (me["extras"] as Array).size():
			wares.append(_card(CardView.of_def(me["extras"][index], CardView.Mode.BOARD, {"cost": true}), "extra", index, -1, null))
	add_child(offers)
	offers.hold(wares, card)
	Tutorial.mark(offers, "shop_row")
	# A reserved shop lies under a sheet of ice, over its cards.
	var ice := Frost.new()
	var iced := offers.extent().grow_individual(UI.z(34), UI.z(6), UI.z(34), UI.z(6))
	ice.position = offers.position + iced.position
	ice.size = iced.size
	add_child(ice)
	ice.show_state(me["frozen"], _was_frozen)
	_was_frozen = me["frozen"]

	# While an owned card is dragged the shop row turns into the place to sell it.
	var sign_at := Control.new()
	sign_at.mouse_filter = Control.MOUSE_FILTER_IGNORE
	sign_at.position = offers.position
	sign_at.size = offers.size
	Tutorial.mark(sign_at, "sign")
	add_child(sign_at)
	_sell_zone = UI.DropZone.new()
	_sell_zone.add_theme_stylebox_override("panel", UI.box(Color(UI.VERMILION_DEEP, 0.92), 30, 8, UI.PAPER, 6))
	_sell_zone.accepts = func(data: Variant) -> bool: return data is Dictionary and ["hand", "board"].has(data.get("zone", ""))
	_sell_zone.dropped = func(data: Variant) -> void: _me_command("Sell", {"card": int(data["id"])})
	_sell_zone.add_child(UI.centered("Sell here  +%d gold" % int(Store.tunables["sell_value"]), UI.GLANCE, UI.PAPER, false))
	_sell_zone.visible = _owned_card_in_flight()
	_sell_zone.position = offers.position
	_sell_zone.size = offers.size
	add_child(_sell_zone)

	# Board row: the cards in play, in a line. No places are drawn.
	var board_band_at: Vector2 = bands["board"]
	_board_zone = UI.DropZone.new()
	_board_zone.accepts = func(data: Variant) -> bool: return data is Dictionary and ["shop", "extra", "hand"].has(data.get("zone", ""))
	_board_zone.dropped = _dropped_on_board
	_board_zone.position = Vector2(field.x, board_band_at.x)
	_board_zone.size = Vector2(field.y, board_band_at.y - board_band_at.x)
	add_child(_board_zone)
	_board_row = CardRow.new()
	_board_row.spacing = spacing
	_board_row.gap_share = row_gap_share
	_board_row.gap_seconds = row_gap_seconds
	_board_zone.add_child(_board_row)
	var in_play: Array = []
	for index in (me["board"] as Array).size():
		var instance: Dictionary = me["board"][index]
		var chosen: bool = me["endgame_choice"] != null and int(me["endgame_choice"]) == int(instance["id"])
		in_play.append(_card(CardView.of_instance(instance, CardView.Mode.BOARD, {"chosen": chosen}), "board", index, int(instance["id"]), _board_zone))
	_board_row.hold(in_play, card)
	_board_hint = UI.centered("", UI.z(40), Color(UI.PAPER, board_hint_alpha), false)
	_board_hint.size_flags_vertical = Control.SIZE_FILL
	_board_zone.add_child(_board_hint)
	var count := UI.label("%d/%d" % [in_play.size(), int(Store.tunables["board_slots"])], UI.t(UI.z(26)), Color(UI.PAPER, board_hint_alpha), false)
	count.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	count.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
	count.size_flags_vertical = Control.SIZE_FILL
	_board_zone.add_child(count)
	Tutorial.mark(_board_zone, "board")
	_show_board("")


## How the board looks: at rest (""), with a card held over it ("lit"), or
## refusing the card held over it ("full").
func _show_board(state: String) -> void:
	var tone := board_refuses if state == "full" else board_lit
	var style := UI.box(Color(tone, 0.0 if state == "" else 0.1), 44, 14, Color(tone, 0.0 if state == "" else 0.7), 3)
	style.corner_detail = 12
	_board_zone.add_theme_stylebox_override("panel", style)
	var empty := (Store.me()["board"] as Array).is_empty()
	_board_hint.text = "The board is full" if state == "full" else ("Place cards here" if empty else "")
	# The refusal stands along the bottom edge, clear of the cards and of
	# the card in the hand, which is held above the finger.
	_board_hint.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM if state == "full" else VERTICAL_ALIGNMENT_CENTER
	_board_hint.add_theme_font_size_override("font_size", UI.z(26 if state == "full" else 40))
	_board_hint.add_theme_color_override("font_color", Color(tone if state == "full" else UI.PAPER, 0.95 if state == "full" else board_hint_alpha))


## Follows a card dragged over the board: the board lights up, and for a
## card that would be placed its neighbours part where the pointer is. The
## gap only shows; the host decides where the card goes. `at_once` skips
## the movement, for a screen that was just built.
func _track_drag(at_once: bool) -> void:
	if not is_instance_valid(_board_zone) or not is_inside_tree():
		return
	var state := ""
	var gap := -1
	if get_viewport().gui_is_dragging():
		var data = get_viewport().gui_get_drag_data()
		if data is Dictionary and ["shop", "extra", "hand"].has(data.get("zone", "")) and _board_zone.get_global_rect().has_point(_pointer):
			state = "lit"
			var me := Store.me()
			var index := int(data.get("index", -1))
			# Only a card that goes onto the board asks for a place on it.
			# A reward card is dropped there too, but it opens and takes no place.
			if String(data["zone"]) == "hand" and index >= 0 and index < (me["hand"] as Array).size() \
					and String(_card_actions("hand", me["hand"][index])[0][1]) == "Place" \
					and not _is_reward_card(me["hand"][index]):
				if (me["board"] as Array).size() >= int(Store.tunables["board_slots"]):
					state = "full"
				else:
					gap = _board_row.slot_at(_pointer.x)
	if state != _board_state:
		_board_state = state
		_show_board(state)
	if gap != _board_gap:
		_board_gap = gap
		_board_row.open_gap(gap, at_once)


## The three shop buttons, top to bottom: upgrade, restock, reserve.
## They stand in the column on the right edge, between the timer and the gold.
func _shop_buttons(me: Dictionary) -> Array[Button]:
	var cost = Store.view["upgrade_cost"]
	var level := int(me["tavern"])
	# The upgrade picture shows the level the player is on; all but the last
	# carry an arrow. Missing pictures fall back to plain round buttons.
	var steps := "Top\nlevel" if cost == null else "Level\n%s > %s" % [UI.roman(level), UI.roman(level + 1)]
	var upgrade := _shop_button("upgrade_%d" % level, steps, "" if cost == null else str(int(cost)), "UpgradeTavern", side_button)
	upgrade.disabled = cost == null
	var upgrade_text := "This is the top level."
	if cost != null:
		upgrade_text = "Raise your library to level %s for %d gold. A higher level puts higher-tier cards in your shop and adds to the damage you deal. The price drops by 1 each turn you wait." % [UI.roman(level + 1), int(cost)]
	Peek.attach(upgrade, Peek.note.bind("Library level %s" % UI.roman(level), upgrade_text))

	var restock := _shop_button("restock", "Restock", str(int(Store.tunables["reroll_cost"])), "Reroll", side_button)
	Peek.attach(restock, Peek.note.bind("Restock", "Swap the cards in your shop for new ones, for %d gold." % int(Store.tunables["reroll_cost"])))
	var reserve := _shop_button("reserve_on" if me["frozen"] else "reserve_off",
		"Release\nshop" if me["frozen"] else "Reserve\nshop", "", "ToggleFreeze", side_button, UI.GILT if me["frozen"] else UI.PAPER)
	Peek.attach(reserve, Peek.note.bind("Reserve", "Keep these shop cards for your next turn. Free. Tap again to release them."))
	return [upgrade, restock, reserve]


## Where the hero, the ability and the hand stand at the foot of the board:
## {"hero": centre, "ability": centre, "hand": where the hand at rest begins,
## from the left}. With the borders they are the sockets of the bottom
## border, wherever its size puts them; without, a row of their own. Also
## sets `hero_size` and `ability_size`.
func _spots() -> Dictionary:
	var board := _board()
	if _bordered():
		var picture := UI.art("board/bottom.png")
		var k := _border_scale(picture)
		# The top left corner of the border, on this screen.
		var origin := Vector2(-_margin("margin_left"),
			board.y + _margin("margin_bottom") + UI.z(bottom_border_drop) - picture.get_height() * k)
		hero_size = int(ART_ARCH.size.y * k * arch_fill)
		ability_size = int(ART_SOCKET.size.x * k * socket_fill)
		return {
			"hero": origin + ART_ARCH.get_center() * k,
			"ability": origin + ART_SOCKET.get_center() * k,
			"hand": origin + Vector2(ART_TRAY.position.x, ART_TRAY.end.y - picture.get_height() * tray_rest) * k,
		}
	var middle := Vector2(board.x / 2.0, board.y - hero_raise - hero_size / 2.0)
	return {
		"hero": middle,
		"ability": middle + Vector2(-(hero_size + ability_size) / 2.0 - hero_gap, ability_drop),
		"hand": middle + Vector2(hero_size / 2.0 + hero_gap, hero_size / 2.0),
	}


## The foot of the board: the hero in the middle, the ability as a round
## button on its left, and the hand as a small fan on its right. A tap on
## the fan opens the hand.
func _add_desk(me: Dictionary, hero: Dictionary) -> void:
	var cards: Array = me["hand"]
	var targeting := _mode == "darwin" or _mode == "combine"
	var spots := _spots()
	if not _bordered():
		_add_plinth(spots["hero"])

	var ability := _ability_button(hero, targeting)
	Tutorial.mark(ability, "ability")
	ability.position = (spots["ability"] as Vector2) - Vector2(ability_size, ability_size) / 2.0
	add_child(ability)

	var portrait := _hero_button(me, hero)
	portrait.position = (spots["hero"] as Vector2) - Vector2(hero_size, hero_size) / 2.0
	add_child(portrait)

	# The hand at rest: a fan at the bottom edge, its cards partly below the
	# screen. Hidden while the hand is raised, when its cards are up there.
	var wide := rest_card * _height()
	var large := CardView.size_of(CardView.Mode.LARGE)
	var fan := HandFan.new()
	fan.card_scale = wide / large.x
	fan.spacing = rest_spread * wide
	fan.max_width = rest_width * _height()
	fan.arc_degrees = rest_arc
	var small: Array = []
	for instance in cards:
		small.append(CardView.of_instance(instance, CardView.Mode.LARGE, {"still": true, "summary": true}))
	fan.hold(small, large)
	fan.position = Vector2((spots["hand"] as Vector2).x,
		_board().y + _margin("margin_bottom") - (1.0 - rest_sunk) * large.y * fan.card_scale)
	fan.tapped.connect(open_hand)
	fan.visible = not _hand_shown(me)
	if cards.size() >= rest_badge_from:
		var number := UI.badge(str(cards.size()), UI.z(56), UI.INK, UI.PAPER, UI.z(30), UI.PAPER_SHADE)
		number.position = Vector2(UI.z(6), -UI.z(44))
		fan.add_child(number)
	Tutorial.mark(fan, "hand")
	_rest_fan = fan
	add_child(fan)

	# The whole foot of the board takes a card dragged from the shop: while
	# one is in flight a "buy" zone covers the hero and the hand.
	_buy_zone = UI.DropZone.new()
	_buy_zone.add_theme_stylebox_override("panel", UI.box(Color(UI.VERDIGRIS_DEEP, 0.9), 30, 8, UI.PAPER, 6))
	_buy_zone.accepts = func(data: Variant) -> bool: return data is Dictionary and ["shop", "extra"].has(data.get("zone", ""))
	_buy_zone.dropped = _dropped_on_hand
	_buy_zone.add_child(UI.centered("Drop here to buy", UI.GLANCE, UI.PAPER, false))
	_buy_zone.visible = _card_in_flight(["shop", "extra"])
	var top: float = _bands()["desk"]
	_buy_zone.position = Vector2(_field().x, top)
	_buy_zone.size = Vector2(_field().y, _board().y - top)
	add_child(_buy_zone)


## Without the borders: a soft shadow under the hero, the ability and the
## hand, so they stand on the board and do not float over it.
func _add_plinth(hero_at: Vector2) -> void:
	var shade := Panel.new()
	shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var style := UI.box(Color(0.01, 0.01, 0.04, plinth_alpha), 400, 0)
	style.corner_detail = 16
	style.shadow_color = Color(0.01, 0.01, 0.04, plinth_alpha * 0.7)
	style.shadow_size = int(hero_size * plinth.y * 0.7)
	shade.add_theme_stylebox_override("panel", style)
	shade.size = Vector2(hero_size * plinth.x, hero_size * plinth.y)
	# Under the ability and the hero together, so a little left of the hero.
	shade.position = hero_at + Vector2(-shade.size.x / 2.0 - (ability_size + hero_gap) / 2.0, hero_size / 2.0 - shade.size.y * 0.75)
	add_child(shade)


## The hand is raised: it was tapped, or a card is being picked for the
## hero's ability.
func _hand_shown(me: Dictionary) -> bool:
	return (_hand_open or _mode == "darwin" or _mode == "combine") and not (me["hand"] as Array).is_empty()


## The raised hand: a large fan across the desk, its cards running off the
## bottom of the screen. It covers the hero and the ability, never the
## board. A tap beside it lowers it again. Each card is held to read it
## large in the middle of the screen, tapped for its actions and dragged up
## to the board.
func _add_hand_view(me: Dictionary, hero: Dictionary) -> void:
	_hand_layer = null
	if not _hand_shown(me):
		_hand_open = false
		return
	var targeting := _mode == "darwin" or _mode == "combine"
	var layer := Control.new()
	layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_hand_layer = layer
	add_child(layer)
	# The desk darkens behind the hand; the rows above stay as they are.
	var fade := Gradient.new()
	fade.offsets = PackedFloat32Array([0.0, fan_dim_edge, 1.0])
	fade.colors = PackedColorArray([Color(0.03, 0.04, 0.08, 0.0), Color(0.03, 0.04, 0.08, fan_dim), Color(0.03, 0.04, 0.08, fan_dim)])
	var falling := GradientTexture2D.new()
	falling.gradient = fade
	falling.fill_to = Vector2(0, 1)
	falling.width = 4
	falling.height = 128
	var shade := TextureRect.new()
	shade.texture = falling
	shade.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	shade.stretch_mode = TextureRect.STRETCH_SCALE
	shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var from: float = (_bands()["board"] as Vector2).y
	shade.position = Vector2(-300, from)
	shade.size = Vector2(_board().x + 600, _board().y + 300 - from)
	layer.add_child(shade)
	# Picking a card for the ability: the board stays in reach, since its
	# cards can be picked too. Otherwise a tap anywhere else lowers the hand.
	if not targeting:
		var beside := Button.new()
		beside.set_anchors_preset(Control.PRESET_FULL_RECT)
		beside.offset_left = -300
		beside.offset_right = 300
		beside.offset_top = -300
		beside.offset_bottom = 300
		UI.bare(beside)
		beside.pressed.connect(close_hand)
		layer.add_child(beside)
	var fan := HandFan.new()
	fan.live = true
	var size_built := CardView.size_of(CardView.Mode.LARGE)
	fan.card_scale = raised_card * _height() / size_built.x
	fan.spacing = raised_spacing * _height()
	fan.arc_degrees = raised_arc
	fan.max_width = _field().y
	var cards: Array = me["hand"]
	var large: Array = []
	for index in cards.size():
		var instance: Dictionary = cards[index]
		var chosen: bool = me["endgame_choice"] != null and int(me["endgame_choice"]) == int(instance["id"])
		var card := _card(CardView.of_instance(instance, CardView.Mode.LARGE, {"chosen": chosen, "summary": true}), "hand", index, int(instance["id"]), null)
		Peek.attach(card, func() -> Control: return CardView.of_instance(instance, CardView.Mode.LARGE, {"still": true}), true)
		large.append(card)
	fan.hold(large, size_built)
	fan.position = Vector2((_board().x - fan.extent().x) / 2.0, _y(raised_top))
	Tutorial.mark(fan, "hand")
	layer.add_child(fan)
	# The greyed ability button cancels the pick; it stays in reach above the hand.
	if targeting:
		var cancel := _ability_button(hero, targeting)
		cancel.position = (_spots()["ability"] as Vector2) - Vector2(ability_size, ability_size) / 2.0
		layer.add_child(cancel)


## The hero at the bottom centre, with health on a seal. Hold to read the hero.
func _hero_button(me: Dictionary, hero: Dictionary) -> Button:
	var plate := Button.new()
	plate.custom_minimum_size = Vector2(hero_size, hero_size)
	UI.bare(plate)
	plate.add_child(UI.portrait(hero, hero_size, UI.GILT, 6, 16))
	var health := UI.token("health", str(int(me["hp"])), UI.z(96), UI.z(38), UI.PAPER, UI.VERMILION_DEEP)
	health.position = Vector2(hero_size - UI.z(84), hero_size - UI.z(88))
	plate.add_child(health)
	UI.quiet(plate)
	Peek.attach(plate, HeroCard.info.bind(hero, _player_state(Store.view["players"][Store.my_id()])))
	return plate


## The hero's ability as a round button beside the hero. Tap to use it, hold
## to read it. Its price stands on a coin on the rim; an ability that is free
## carries no coin, and one that is not pressed here says so on a tag.
func _ability_button(hero: Dictionary, targeting: bool) -> Button:
	var ability := Ability.of(hero)
	var press := Peek.guard(_hero_pressed.bind(hero))
	if ability == null:
		# No ability: an empty seat of the same size keeps the layout.
		var seat := Button.new()
		UI.bare(seat)
		seat.custom_minimum_size = Vector2(ability_size, ability_size)
		seat.disabled = true
		seat.mouse_filter = Control.MOUSE_FILTER_IGNORE
		return seat
	var button: Button
	var coin := "" if targeting else ability.coin_text()
	# An ability bought here and used in battle (Newton's sword) has three
	# looks: greyed while the shop does not offer it, lit with its price
	# while it does, and lit with the uses left once it is owned.
	var bought_for_battle := ability.when == Ability.When.BATTLE and ability.cost > 0
	var owned := int(Store.me()["hero_state"]["uses"])
	var offered := int(Store.me()["hero_state"].get("sword_offer_turn", 0)) == int(Store.view["turn"]) and owned == 0
	if bought_for_battle and not offered:
		coin = ""
	if ability.icon != null:
		button = UI.emblem(ability.icon, "", coin, press, Vector2(ability_size, ability_size))
	else:
		button = UI.medallion(ability.name, coin, press, ability_size, UI.INK_RAISED, 22)
	if targeting:
		# In use: greyed until the targets are picked. A tap on it still cancels.
		button.modulate = Color(0.45, 0.45, 0.5)
		return button
	var tag_text := ability.tag()
	var hero_state: Dictionary = Store.me()["hero_state"]
	# Used up for this turn: greyed until the next one.
	if ability.used_this_round(hero_state, int(Store.view["turn"])):
		button.modulate = Color(0.45, 0.45, 0.5)
		tag_text = "Used"
	# Limited per match: the uses left out of the limit, greyed for good at none.
	if ability.counted(_alive_count()):
		tag_text = ability.counter(hero_state)
		if ability.left(hero_state) == 0:
			button.modulate = Color(0.45, 0.45, 0.5)
	if bought_for_battle:
		tag_text = "%d/%d" % [owned, ability.uses_per_purchase] if owned > 0 else ("Buy" if offered else "Not in shop")
		if owned == 0 and not offered:
			button.modulate = Color(0.45, 0.45, 0.5)
	if tag_text != "":
		var tag := PanelContainer.new()
		tag.mouse_filter = Control.MOUSE_FILTER_IGNORE
		tag.add_theme_stylebox_override("panel", UI.box(Color(0.07, 0.08, 0.14, 0.95), 16, 4, Color("b9c6dc"), 2))
		tag.get_theme_stylebox("panel").content_margin_left = 12
		tag.get_theme_stylebox("panel").content_margin_right = 12
		tag.add_child(UI.centered(tag_text, UI.t(UI.z(24)), UI.PAPER, false))
		tag.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
		tag.grow_horizontal = Control.GROW_DIRECTION_BOTH
		tag.grow_vertical = Control.GROW_DIRECTION_BEGIN
		tag.offset_bottom = 10
		button.add_child(tag)
	Peek.attach(button, Peek.note.bind(ability.name, "%s\n%s" % [ability.text, ability.terms()], 640))
	return button


## How many players are still in the match.
func _alive_count() -> int:
	var alive := 0
	for player in Store.view["players"]:
		if player["alive"]:
			alive += 1
	return alive


func _status_line(_me: Dictionary) -> String:
	if _mode == "darwin":
		return "Pick the card to evolve. Tap the ability button to cancel."
	if _mode == "combine":
		return "Pick the second card to combine." if _combine_first >= 0 else "Pick two cards to combine. Tap the ability button to cancel."
	return ""


## A card in the shop, on the board or in the hand: a tap opens its actions,
## a hold shows it large (the card does that itself), a drag moves it.
func _card(card: CardView, zone: String, index: int, id: int, drop_zone: UI.DropZone) -> CardView:
	card.drag_data = {"zone": zone, "index": index, "id": id}
	card.drop_proxy = drop_zone
	card.pressed.connect(Peek.guard(_tap.bind(zone, index)))
	return card


## The right edge: the timer beside the felt, gold at the foot.
func _side(me: Dictionary) -> Control:
	var column := UI.column(side_gap)
	column.custom_minimum_size.x = side_width
	column.alignment = BoxContainer.ALIGNMENT_CENTER

	var clock := PanelContainer.new()
	clock.add_theme_stylebox_override("panel", UI.plate(60, 8))
	clock.custom_minimum_size.y = UI.z(112)
	_timer = UI.centered("", UI.z(78), UI.PAPER, false)
	clock.add_child(_timer)
	Tutorial.mark(clock, "timer")
	column.add_child(clock)

	var names := ["upgrade", "restock", "reserve"]
	var buttons := _shop_buttons(me)
	for index in buttons.size():
		buttons[index].size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		Tutorial.mark(buttons[index], names[index])
		column.add_child(buttons[index])

	if _put_aside:
		column.add_child(UI.stamp("Reward" if not (me["rewards"] as Array).is_empty() else "Categories",
			_take_back, Vector2(0, 80), 28, UI.GILT))
	var purse := UI.row(0)
	purse.alignment = BoxContainer.ALIGNMENT_CENTER
	purse.add_child(UI.token("coin", str(int(me["gold"])), UI.z(140), UI.z(68)))
	Tutorial.mark(purse, "gold")
	column.add_child(purse)
	return column


## A plate over the foot of the felt for hints and refusals. Hidden while empty.
func _add_toast() -> void:
	var strip := PanelContainer.new()
	strip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	strip.add_theme_stylebox_override("panel", UI.plate(48, 8))
	strip.get_theme_stylebox("panel").content_margin_left = 40
	strip.get_theme_stylebox("panel").content_margin_right = 40
	strip.custom_minimum_size = Vector2(900, 84)
	strip.set_anchors_preset(Control.PRESET_CENTER_TOP)
	strip.grow_horizontal = Control.GROW_DIRECTION_BOTH
	strip.offset_top = (_bands()["board"] as Vector2).y + 14
	var strip_row := UI.row(12)
	strip_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	strip.add_child(strip_row)
	_caption = UI.centered("", UI.t(30))
	_caption.max_lines_visible = 2
	_caption.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
	strip_row.add_child(_caption)
	add_child(strip)
	_idle_caption = _status_line(Store.me())
	_show_caption(_idle_caption)


## One of the three shop buttons: its painting from `assets/buttons/` when
## the file is there, a plain round button with `words` when it is not.
func _shop_button(file: String, words: String, coin: String, command: String, height: int, color := UI.PAPER) -> Button:
	var picture := UI.art("buttons/%s.png" % file)
	if picture == null:
		return UI.medallion(words, coin, Peek.guard(_me_command.bind(command)), mini(height, 150), color)
	var width := int(height * picture.get_width() / float(picture.get_height()))
	return UI.emblem(picture, "", coin, Peek.guard(_me_command.bind(command)), Vector2(width, height))


func _owned_card_in_flight() -> bool:
	return _card_in_flight(["hand", "board"])


## A card from one of these zones is being dragged right now.
func _card_in_flight(zones: Array) -> bool:
	if not is_inside_tree() or not get_viewport().gui_is_dragging():
		return false
	var data = get_viewport().gui_get_drag_data()
	return data is Dictionary and zones.has(data.get("zone", ""))


# ---- Taps and drops ----------------------------------------------------------

func _tap(zone: String, index: int) -> void:
	var owned := zone == "hand" or zone == "board"
	if _mode == "darwin" and owned:
		var instance: Dictionary = Store.me()[zone][index]
		_ability({"DarwinMerge": {"def": int(instance["parts"][0]["def"])}})
	elif _mode == "combine" and owned:
		var id := int(Store.me()[zone][index]["id"])
		if _combine_first < 0:
			_combine_first = id
			refresh()
		else:
			var first := _combine_first
			_combine_first = -1
			_ability({"Combine": {"a": first, "b": id}})
	else:
		inspect(zone, index)


func _dropped_on_board(data: Variant) -> void:
	match String(data["zone"]):
		"shop":
			_me_command("Buy", {"index": int(data["index"])})
		"extra":
			_me_command("BuyExtra", {"index": int(data["index"])})
		"hand":
			var instance: Dictionary = Store.me()["hand"][int(data["index"])]
			_me_command(String(_card_actions("hand", instance)[0][1]), {"card": int(data["id"])})


func _dropped_on_hand(data: Variant) -> void:
	_me_command("Buy" if String(data["zone"]) == "shop" else "BuyExtra", {"index": int(data["index"])})


func _hero_pressed(hero: Dictionary) -> void:
	if _mode == "darwin" or _mode == "combine":
		close_overlays()
		return
	var limited := Ability.of(hero)
	if limited != null and limited.when == Ability.When.SHOP:
		var hero_state: Dictionary = Store.me()["hero_state"]
		if limited.counted(_alive_count()) and limited.left(hero_state) == 0:
			_say("%s has no uses left this match." % limited.name)
			_show_caption(_idle_caption)
			return
		if limited.used_this_round(hero_state, int(Store.view["turn"])):
			_say("%s was used this round. It can be used again next turn." % limited.name)
			_show_caption(_idle_caption)
			return
	match String(hero["kind"]):
		"Herodotus":
			_open_mode("confirm", {"title": "Rewrite your last battle into a win? Once per game.", "action": "Rewrite"})
		"Shakespeare":
			_open_mode("confirm", {"title": "To be or not to be? This costs all your gold and must be your first action this round.", "action": "ToBeOrNotToBe"})
		"Darwin":
			_open_mode("darwin", null)
		"Benz":
			_open_mode("combine", null)
		"Lumiere":
			_open_mode("record", null)
		"Nietzsche":
			_ability("Ubermensch")
		"MarcoPolo":
			_ability("SilkRoad")
		_:
			var ability := Ability.of(hero)
			if ability != null and ability.when == Ability.When.BATTLE and ability.cost > 0:
				var state: Dictionary = Store.me()["hero_state"]
				if int(state["uses"]) > 0:
					_say("You hold the %s: %d left. Use it during a battle." % [ability.name, int(state["uses"])])
				elif int(state.get("sword_offer_turn", 0)) == int(Store.view["turn"]):
					_ability("BuySword")
					return
				else:
					_say("The shop is not offering the %s this turn." % ability.name)
			elif ability != null and ability.when == Ability.When.BATTLE:
				_say("%s is used during a battle, for free. Hold the button to read it." % ability.name)
			elif ability != null:
				_say("%s works by itself. Hold the button to read it." % ability.name)
			_show_caption(_idle_caption)


func _open_mode(mode: String, data: Variant) -> void:
	_mode = mode
	_mode_data = data
	_selected = {}
	_combine_first = -1
	refresh()


func _take_back() -> void:
	_put_aside = false
	refresh()


func _put_away() -> void:
	_put_aside = true
	refresh()


# ---- Overlays ----------------------------------------------------------------

func _add_overlay(me: Dictionary, hero: Dictionary) -> void:
	if not (me["rewards"] as Array).is_empty():
		if not _put_aside:
			_modal(_reward(me["rewards"][0]), Callable())
	elif not (me["hero_state"]["house_options"] as Array).is_empty():
		if not _put_aside:
			_modal(_house_picker(me["hero_state"]["house_options"]), Callable())
	elif _mode == "confirm":
		_modal(_confirm(_mode_data), close_overlays)
	elif _mode == "record":
		_modal(_record_picker(me, hero), close_overlays)
	elif not _selected.is_empty():
		var content := _inspect_panel(me)
		if content != null:
			_modal(content, close_overlays)


## Dims the screen and centres `content` on it. A tap outside calls `on_close`.
func _modal(content: Control, on_close: Callable) -> void:
	var dim := Button.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	# Past the frame's margins, so the whole screen dims and no edge shows.
	dim.offset_left = -300
	dim.offset_right = 300
	dim.offset_top = -300
	dim.offset_bottom = 300
	UI.face(dim, UI.box(Color(0.03, 0.04, 0.08, 0.78), 0, 0), 0.0)
	if on_close.is_valid():
		dim.pressed.connect(on_close)
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	centre.add_child(content)
	add_child(centre)


## TEMPORARY rule knowledge (docs/ui-research.md, open question 1): which
## actions a card offers. Returns [[label, command, colour], ...], the main
## action first. Delete when the view lists the allowed actions (V6).
func _card_actions(zone: String, instance: Dictionary) -> Array:
	var actions: Array = []
	var first: Dictionary = Store.card(instance["parts"][0]["def"])
	if _is_reward_card(instance):
		# Placed like any card, which opens its pick; it sells for nothing.
		return [["Open", "Place", UI.VERDIGRIS], ["Discard", "Sell", UI.VERMILION]]
	if zone == "hand":
		var one_use := false
		for effect in first["effects"]:
			one_use = one_use or String(effect["effect"]) == "HealOnce"
		actions.append(["Use", "UseCard", UI.VERDIGRIS] if one_use else ["Place", "Place", UI.VERDIGRIS])
	if String(first["kind"]) == "endgame":
		actions.append(["Use in the endgame", "ChooseEndgameEffect", UI.PAPER])
	actions.append(["Sell  +%d gold" % int(Store.tunables["sell_value"]), "Sell", UI.VERMILION])
	return actions


## The card that stands for an unpicked golden reward.
func _is_reward_card(instance: Dictionary) -> bool:
	return String(Store.card(instance["parts"][0]["def"]).get("id", "")) == CardView.REWARD_CARD


func _inspect_panel(me: Dictionary) -> Control:
	var zone := String(_selected["zone"])
	var index := int(_selected["index"])
	var source: Array = me[{"shop": "shop", "extra": "extras", "hand": "hand", "board": "board"}[zone]]
	if index >= source.size():
		_selected = {}
		return null
	var line := UI.row(32)
	var buttons := UI.column(16)
	buttons.alignment = BoxContainer.ALIGNMENT_CENTER
	# The card's definition, when it is one plain card that has a golden version to preview.
	var plain_def := -1
	if zone == "shop" or zone == "extra":
		plain_def = int(source[index])
	elif (source[index]["parts"] as Array).size() == 1 and not source[index]["parts"][0]["golden"]:
		plain_def = int(source[index]["parts"][0]["def"])
	if plain_def >= 0 and not Store.card(plain_def)["can_golden"]:
		plain_def = -1
	var preview := _preview_golden and plain_def >= 0
	if preview:
		line.add_child(CardView.new([{"def": plain_def, "golden": true}], CardView.Mode.LARGE, {"still": true}))
	if zone == "shop" or zone == "extra":
		if not preview:
			line.add_child(CardView.of_def(source[index], CardView.Mode.LARGE, {"cost": true, "still": true}))
		var cost := int(Store.card(source[index])["cost"])
		buttons.add_child(UI.stamp("Buy  %d gold" % cost, _act.bind("Buy" if zone == "shop" else "BuyExtra", {"index": index}),
			Vector2(300, 150), 40, UI.VERDIGRIS))
	else:
		var instance: Dictionary = source[index]
		if not preview:
			line.add_child(CardView.of_instance(instance, CardView.Mode.LARGE, {"still": true}))
		for action in _card_actions(zone, instance):
			buttons.add_child(UI.stamp(String(action[0]), _act.bind(String(action[1]), {"card": int(instance["id"])}),
				Vector2(300, 150), 40, action[2]))
	if plain_def >= 0:
		buttons.add_child(UI.stamp("Show normal" if preview else "Show golden", _flip_preview, Vector2(300, 116), 36, UI.INK_RAISED))
	buttons.add_child(UI.stamp("Close", close_overlays, Vector2(300, 116), 36, UI.INK_RAISED))
	# The card shown, with a note on what the two letters on its seal stand for.
	var shown := line.get_child(0) as Control
	var shown_def: Dictionary = Store.card(plain_def if plain_def >= 0 else (int(source[index]) if zone == "shop" or zone == "extra" else int(source[index]["parts"][0]["def"])))
	line.remove_child(shown)
	var noted := UI.with_kind_note(shown, shown_def)
	line.add_child(noted)
	line.move_child(noted, 0)
	line.add_child(buttons)
	return line


func _flip_preview() -> void:
	_preview_golden = not _preview_golden
	refresh()


func _act(kind: String, fields: Dictionary) -> void:
	_selected = {}
	_me_command(kind, fields)


func _reward(choices: Array) -> Control:
	var stack := UI.column(16)
	var header := UI.row(24)
	var words := UI.column(2)
	words.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	words.add_child(UI.label("Golden reward", 48, UI.PAPER, false))
	words.add_child(UI.label("Choose one, free. If time runs out, the pick waits in your hand as a reward card.", 26, UI.GILT, false))
	header.add_child(words)
	header.add_child(UI.stamp("Hide", _put_away, Vector2(220, 100), 36, UI.INK_RAISED))
	stack.add_child(header)
	var options := UI.row(40)
	options.alignment = BoxContainer.ALIGNMENT_CENTER
	for index in choices.size():
		var option := UI.column(12)
		option.add_child(CardView.of_def(choices[index], CardView.Mode.LARGE, {"still": true, "cost_text": "0"}))
		option.add_child(UI.stamp("Take", _me_command.bind("PickReward", {"index": index}), Vector2(420, 130), UI.BODY, UI.VERDIGRIS))
		options.add_child(option)
	stack.add_child(options)
	return stack


func _house_picker(options: Array) -> Control:
	var sheet := UI.panel(UI.INK_RAISED, 16, 24)
	var stack := UI.column(20)
	sheet.add_child(stack)
	var header := UI.row(24)
	header.add_child(UI.label("Pick two categories for the house questions", 40, UI.GILT))
	header.add_child(UI.stamp("Hide", _put_away, Vector2(220, 100), 36, UI.INK))
	stack.add_child(header)
	var tiles := UI.row(24)
	for option in options:
		var id := int(option)
		var picked := _house_picks.has(id)
		var tile := UI.stamp(UI.category_label(id) + ("\n(picked)" if picked else ""), _pick_house.bind(id),
			Vector2(300, 240), 38, UI.GILT if picked else UI.CATEGORY_TINTS[id])
		tiles.add_child(tile)
	stack.add_child(tiles)
	return sheet


func _pick_house(id: int) -> void:
	if _house_picks.has(id):
		_house_picks.erase(id)
	else:
		_house_picks.append(id)
	if _house_picks.size() == 2:
		var action := {"ChooseHouse": {"first": _house_picks[0], "second": _house_picks[1]}}
		_house_picks.clear()
		_ability(action)
	else:
		refresh()


func _sheet(width: int) -> Array:
	var sheet := UI.panel(UI.INK_RAISED, 16, 24)
	sheet.custom_minimum_size.x = width
	var stack := UI.column(16)
	sheet.add_child(stack)
	return [sheet, stack]


func _confirm(data: Dictionary) -> Control:
	var made := _sheet(820)
	var stack: VBoxContainer = made[1]
	stack.add_child(UI.label(String(data["title"]), 40))
	var buttons := UI.row(24)
	var yes := UI.stamp("Do it", _ability.bind(data["action"]), Vector2(0, 150), 40, UI.VERDIGRIS)
	yes.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(yes)
	var no := UI.stamp("Cancel", close_overlays, Vector2(0, 150), 40, UI.INK)
	no.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(no)
	stack.add_child(buttons)
	return made[0]


## Lumiere: the film questions of the last battle. The view carries no
## question text for past questions yet (V11), so rows are numbered.
func _record_picker(me: Dictionary, hero: Dictionary) -> Control:
	var made := _sheet(820)
	var stack: VBoxContainer = made[1]
	stack.add_child(UI.label("Record: pick a film question to meet again", 40, UI.GILT))
	var film := int(hero["category"])
	var found := 0
	if me["last_battle"] != null:
		var questions: Array = me["last_battle"]["questions"]
		for number in questions.size():
			if int(questions[number]["category"]) == film:
				found += 1
				stack.add_child(UI.stamp("Question %d  (%s)" % [number + 1, UI.category_label(film)],
					_ability.bind({"Record": {"question": int(questions[number]["id"])}}), Vector2(0, 150), 40, UI.PAPER))
	if found == 0:
		stack.add_child(UI.label("There was no film question in your last battle.", 36, UI.SLATE))
	stack.add_child(UI.stamp("Close", close_overlays, Vector2(0, 116), 36, UI.INK))
	return made[0]
