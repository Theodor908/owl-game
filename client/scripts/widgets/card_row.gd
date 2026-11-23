## A row of cards in a line, centred, a fixed distance apart; with too many
## for its width they close up. The row can open a gap between two cards, to
## show where a card held over it would go. It knows nothing about what a
## card is: it lays out whatever controls it is given.
class_name CardRow
extends Control

## Distance from one card to the next, in pixels.
@export var spacing := 216.0
## How far the cards on each side of a gap move away, as a share of the
## distance between cards, and how long that takes.
@export var gap_share := 0.5
@export var gap_seconds := 0.12
## Extra room before the card at `split_at`, to set a second group apart.
@export var split_gap := 36.0

## The first card of the second group, or -1 for one group.
var split_at := -1

var _cards: Array[Control] = []
var _card_size := Vector2.ZERO
var _gap := -1
var _tween: Tween


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	resized.connect(_place.bind(false))


## Lays out `cards`, each of `card_size`, left to right. Call once.
func hold(cards: Array, card_size: Vector2) -> void:
	_card_size = card_size
	for card in cards:
		_cards.append(card as Control)
		add_child(card)
	_place(false)


## Opens a gap before card `index` (after the last card when `index` is the
## number of cards); -1 closes it. `at_once` skips the movement.
func open_gap(index: int, at_once := false) -> void:
	if index == _gap:
		return
	_gap = index
	_place(not at_once)


## The gap a card held at `global_x` asks for: the index it would take.
func slot_at(global_x: float) -> int:
	var x := global_x - global_position.x
	for index in _cards.size():
		if x < _rest_x(index) + _card_size.x / 2.0:
			return index
	return _cards.size()


## The room the cards take at rest, in the row's own coordinates.
func extent() -> Rect2:
	if _cards.is_empty():
		return Rect2(size / 2.0, Vector2.ZERO)
	var left := _rest_x(0)
	return Rect2(left, (size.y - _card_size.y) / 2.0, _rest_x(_cards.size() - 1) + _card_size.x - left, _card_size.y)


func _step() -> float:
	var count := _cards.size()
	if count < 2:
		return spacing
	var extra := split_gap if split_at > 0 and split_at < count else 0.0
	return minf(spacing, (size.x - _card_size.x - extra) / (count - 1))


func _rest_x(index: int) -> float:
	var count := _cards.size()
	var step := _step()
	var extra := split_gap if split_at > 0 and split_at < count else 0.0
	var left := (size.x - (count - 1) * step - extra - _card_size.x) / 2.0
	return left + index * step + (extra if split_at > 0 and index >= split_at else 0.0)


func _place(moving: bool) -> void:
	if _tween != null and _tween.is_valid():
		_tween.kill()
	if _cards.is_empty():
		return
	var shift := gap_share * _step()
	var y := (size.y - _card_size.y) / 2.0
	moving = moving and is_inside_tree()
	if moving:
		_tween = create_tween().set_parallel().set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	for index in _cards.size():
		var x := _rest_x(index)
		if _gap >= 0:
			x += -shift if index < _gap else shift
		var control := _cards[index]
		control.size = _card_size
		if moving:
			_tween.tween_property(control, "position", Vector2(x, y), gap_seconds)
		else:
			control.position = Vector2(x, y)
