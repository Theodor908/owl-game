## Preview of a row of cards opening a gap: stand-in cards in a centred
## line, with a gap that walks along the row and closes, as when a card is
## held over the board. Run this scene on its own to tune `CardRow`.
extends Control

## Seconds the gap stays at each place.
@export var hold := 0.5
@export var card := Vector2(200, 228)
@export var cards := 5
## The distance between cards, as a share of the screen's height.
@export var spacing := 0.20
@export var gap_share := 0.5
@export var gap_seconds := 0.12

var _row: CardRow
var _at := -2


func _ready() -> void:
	var back := ColorRect.new()
	back.color = Color("141a33")
	back.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(back)
	var view := get_viewport_rect().size
	_row = CardRow.new()
	_row.spacing = spacing * view.y
	_row.gap_share = gap_share
	_row.gap_seconds = gap_seconds
	_row.position = Vector2(0, view.y * 0.33)
	_row.size = Vector2(view.x, view.y * 0.27)
	add_child(_row)
	var faces: Array = []
	for index in cards:
		var face := Panel.new()
		var style := StyleBoxFlat.new()
		style.bg_color = Color.from_hsv(index / float(cards), 0.45, 0.6)
		style.set_corner_radius_all(90)
		face.add_theme_stylebox_override("panel", style)
		faces.append(face)
	_row.hold(faces, card)
	var timer := Timer.new()
	timer.wait_time = hold
	timer.autostart = true
	timer.timeout.connect(_next)
	add_child(timer)


func _next() -> void:
	_at += 1
	if _at > cards:
		_at = -1
	_row.open_gap(_at)
