## Preview of the damage effects: two portraits as on the battle result. It
## loops a plain hit, then a hit that is reflected back. Run this scene on
## its own to tune `DamageFx`.
extends Control

## Height of the two portraits.
@export var portrait_size := 400
## Seconds of rest between two rounds.
@export var rest := 0.9
## Damage shown on the hit, and how much of it comes back by reflection.
@export var hit := 7
@export var reflected := 3

var _left: Control
var _right: Control
var _fx: DamageFx


func _ready() -> void:
	var back := ColorRect.new()
	back.color = Color("141a33")
	back.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(back)
	var heroes: Array = Store.heroes
	_left = _side(heroes[0] if heroes.size() > 0 else {}, UI.GILT, Vector2(0.22, 0.5))
	_right = _side(heroes[1] if heroes.size() > 1 else {}, UI.SLATE, Vector2(0.78, 0.5))
	_fx = DamageFx.new()
	add_child(_fx)
	_loop()


func _side(hero: Dictionary, ring: Color, at: Vector2) -> Control:
	var holder := Control.new()
	holder.set_anchors_preset(Control.PRESET_FULL_RECT)
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(holder)
	var picture := UI.portrait(hero, portrait_size, ring, 6, 20)
	picture.position = at * Vector2(1920, 1080) - Vector2(portrait_size, portrait_size) / 2.0
	holder.add_child(picture)
	return picture


func _loop() -> void:
	while is_inside_tree():
		var landed := _fx.strike(func() -> Control: return _left, func() -> Control: return _right, hit, hit)
		await get_tree().create_timer(landed + 1.4 + rest).timeout
		_fx.strike(func() -> Control: return _right, func() -> Control: return _left, hit, hit)
		_fx.reflect(func() -> Control: return _left, func() -> Control: return _right, reflected, reflected)
		await get_tree().create_timer(3.2 + rest).timeout
