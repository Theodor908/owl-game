## A hand of cards held as a fan: the cards stand on an arc, each turned
## about a point far below the hand, and overlap their neighbours.
##
## Small and still, it is one thing to tap (`tapped`). Large and `live`, each
## card takes its own taps, holds and drags, and the card under the finger
## rises out of the fan. The fan knows nothing about what a card is: it lays
## out whatever controls it is given.
class_name HandFan
extends Control

## The whole fan was tapped. Only a fan that is not `live` says so.
signal tapped

## The widest the fan opens, edge card to edge card, in degrees.
@export var arc_degrees := 36.0
## The turn between two neighbours, while the arc above allows it.
@export var step_degrees := 5.0
## How much of a card its neighbour covers, as a share of its width.
@export var overlap := 0.5
## The distance from one card to the next, in pixels, in place of `overlap`. 0: use `overlap`.
@export var spacing := 0.0
## Size of the cards, as a share of the size they were built at.
@export var card_scale := 1.0
## The fan is never wider than this; the cards overlap more to fit. 0: no limit.
@export var max_width := 0.0
## The cards take input themselves, and the touched one rises.
@export var live := false
## How far the touched card rises out of the fan, how much it grows, and how fast.
@export var lift := 46.0
@export var lift_scale := 1.12
@export var lift_seconds := 0.09

var _cards: Array[Control] = []
var _card_size := Vector2.ZERO
## Where each card rests: its centre and its turn, in radians.
var _rest: Array = []
var _tweens := {}


## Lays out `cards`, each built at `card_size`, left to right. Call once.
func hold(cards: Array, card_size: Vector2) -> void:
	_card_size = card_size
	mouse_filter = Control.MOUSE_FILTER_IGNORE if live else Control.MOUSE_FILTER_STOP
	for card in cards:
		var control := card as Control
		_cards.append(control)
		add_child(control)
		if live:
			var index := _cards.size() - 1
			control.mouse_entered.connect(raise.bind(index))
			control.mouse_exited.connect(lower.bind(index))
	_arrange()


## The room the fan takes with its cards at rest.
func extent() -> Vector2:
	return custom_minimum_size


## Lifts card `index` out of the fan. Also used by the preview.
func raise(index: int) -> void:
	_move(index, lift, lift_scale, 1)


## Puts card `index` back in its place.
func lower(index: int) -> void:
	_move(index, 0.0, 1.0, 0)


func _gui_input(event: InputEvent) -> void:
	if not live and event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		accept_event()
		tapped.emit()


func _arrange() -> void:
	var count := _cards.size()
	var card := _card_size * card_scale
	if count == 0:
		# An empty hand keeps the room of one card, so there is a place to point at.
		custom_minimum_size = card
		size = card
		return
	var apart := spacing if spacing > 0.0 else card.x * (1.0 - overlap)
	if max_width > 0.0 and count > 1:
		apart = minf(apart, (max_width - card.x) / (count - 1))
	var step := 0.0
	var radius := 0.0
	if count > 1:
		step = deg_to_rad(minf(step_degrees, arc_degrees / (count - 1)))
		radius = apart / step
	var widest := step * (count - 1) / 2.0
	# The room a turned card takes around its centre.
	var half := Vector2(card.x * cos(widest) + card.y * sin(widest), card.y * cos(widest) + card.x * sin(widest)) / 2.0
	var width := 2.0 * (radius * sin(widest) + half.x)
	var height := card.y / 2.0 + radius * (1.0 - cos(widest)) + half.y
	custom_minimum_size = Vector2(width, height)
	size = custom_minimum_size
	_rest.clear()
	for index in count:
		var angle := (index - (count - 1) / 2.0) * step
		var centre := Vector2(width / 2.0 + radius * sin(angle), card.y / 2.0 + radius * (1.0 - cos(angle)))
		_rest.append([centre, angle])
		var control := _cards[index]
		control.size = _card_size
		control.pivot_offset = _card_size / 2.0
		control.scale = Vector2(card_scale, card_scale)
		control.rotation = angle
		control.position = centre - _card_size / 2.0


func _move(index: int, up: float, grow: float, order: int) -> void:
	if index < 0 or index >= _cards.size() or not is_instance_valid(_cards[index]):
		return
	var control := _cards[index]
	var centre: Vector2 = _rest[index][0]
	var angle: float = _rest[index][1]
	# Up along the card's own axis, so it leaves the fan the way it leans.
	var target := centre + Vector2(sin(angle), -cos(angle)) * up - _card_size / 2.0
	control.z_index = order
	if _tweens.has(index) and (_tweens[index] as Tween).is_valid():
		(_tweens[index] as Tween).kill()
	if not is_inside_tree():
		control.position = target
		control.scale = Vector2(card_scale, card_scale) * grow
		return
	var tween := create_tween().set_parallel().set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	tween.tween_property(control, "position", target, lift_seconds)
	tween.tween_property(control, "scale", Vector2(card_scale, card_scale) * grow, lift_seconds)
	_tweens[index] = tween
