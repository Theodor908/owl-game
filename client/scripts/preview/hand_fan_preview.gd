## Preview of the hand: at rest on the bottom edge, half below the screen,
## and raised, with stand-in cards. The two take turns; the number of cards
## goes up and down, and each card of the raised fan rises in turn. Run this
## scene on its own to tune `HandFan`. The sizes are shares of the screen's
## height, as in the shop.
extends Control

## Seconds each number of cards stays up, and each card stays raised.
@export var hold := 1.2
@export var raise_every := 0.25
## Size the stand-in cards are built at, and the sizes of the two fans.
@export var card := Vector2(440, 610)
@export var rest_card := 0.14
@export var rest_width := 0.43
@export var rest_sunk := 0.4
@export var rest_spread := 0.6
@export var rest_arc := 24.0
@export var raised_card := 0.27
@export var raised_top := 0.70
@export var raised_spacing := 0.185
@export var raised_arc := 24.0
@export var most := 10

var _count := 0
var _step := 1
var _small: HandFan
var _big: HandFan
var _raised := -1
var _since := 0.0
var _tick := 0.0


func _ready() -> void:
	var back := ColorRect.new()
	back.color = Color("141a33")
	back.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(back)
	_next()


func _process(delta: float) -> void:
	_since += delta
	_tick += delta
	if _tick >= raise_every and _big != null and _count > 0:
		_tick = 0.0
		_big.lower(_raised)
		_raised = (_raised + 1) % _count
		_big.raise(_raised)
	if _since >= hold:
		_since = 0.0
		_next()


func _next() -> void:
	if _count == most:
		_step = -1
	elif _count <= 1:
		_step = 1
	_count += _step
	_raised = -1
	for fan in [_small, _big]:
		if fan != null:
			fan.queue_free()
	var view := get_viewport_rect().size
	_small = _fan(rest_card * view.y, rest_spread * rest_card * view.y, rest_width * view.y, rest_arc)
	_small.position = Vector2(view.x * 0.5 + 150.0, view.y - (1.0 - rest_sunk) * card.y * _small.card_scale)
	_small.visible = _count % 2 == 1
	add_child(_small)
	_big = _fan(raised_card * view.y, raised_spacing * view.y, view.x - 400.0, raised_arc)
	_big.live = true
	_big.position = Vector2((view.x - _big.extent().x) / 2.0, raised_top * view.y)
	_big.visible = _count % 2 == 0
	add_child(_big)


func _fan(wide: float, apart: float, room: float, arc: float) -> HandFan:
	var fan := HandFan.new()
	fan.card_scale = wide / card.x
	fan.spacing = apart
	fan.max_width = room
	fan.arc_degrees = arc
	var cards: Array = []
	for index in _count:
		cards.append(_stand_in(index))
	fan.hold(cards, card)
	return fan


func _stand_in(index: int) -> Control:
	var face := Panel.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color.from_hsv(index / float(most), 0.45, 0.6)
	style.set_corner_radius_all(36)
	style.set_border_width_all(10)
	style.border_color = Color("d9dceb")
	face.add_theme_stylebox_override("panel", style)
	face.custom_minimum_size = card
	var number := Label.new()
	number.text = str(index + 1)
	number.add_theme_font_size_override("font_size", 200)
	number.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	number.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	number.set_anchors_preset(Control.PRESET_FULL_RECT)
	face.add_child(number)
	return face
