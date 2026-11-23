## Shown once when the match begins: the other players fanned across the
## top, "VS" in the middle, the local player at the foot. Each portrait
## carries the player's name, not the hero's. A press anywhere skips it.
## Holds no rules.
extends Control

signal skipped

@export var other_size := 310
@export var own_size := 350
## How much each portrait in the fan turns and drops, per step from the middle.
@export var fan_degrees := 6.0
@export var fan_drop := 22.0
@export var fan_top := 20.0

var _others: Array[Control] = []
var _own: Control
var _versus: Label


func _ready() -> void:
	var skip := Button.new()
	UI.bare(skip)
	skip.set_anchors_preset(Control.PRESET_FULL_RECT)
	skip.pressed.connect(func() -> void: skipped.emit())
	add_child(skip)

	var me := Store.my_id()
	for player in Store.view["players"]:
		var seat := int(player["id"])
		if seat != me:
			var tile := _tile(seat, other_size, false)
			_others.append(tile)
			add_child(tile)
	_versus = UI.centered("VS", 200, Color("e8eef8"), false)
	_versus.add_theme_color_override("font_outline_color", UI.OUTLINE_INK)
	_versus.add_theme_constant_override("outline_size", 16)
	# A cold glow, in the colour of the border's filigree.
	_versus.add_theme_color_override("font_shadow_color", Color(Color("8fb4ff"), 0.45))
	_versus.add_theme_constant_override("shadow_offset_y", 0)
	_versus.add_theme_constant_override("shadow_outline_size", 44)
	_versus.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_versus)
	_own = _tile(me, own_size, true)
	add_child(_own)
	resized.connect(_place)
	_place()


## A player's hero in the small frame, with the player's name across its foot.
func _tile(seat: int, px: int, own: bool) -> Control:
	var tile := Control.new()
	tile.size = Vector2(px, px)
	tile.pivot_offset = Vector2(px, px) / 2.0
	tile.mouse_filter = Control.MOUSE_FILTER_IGNORE
	tile.add_child(UI.portrait(Store.hero_of(seat), px, UI.GILT if own else UI.SLATE))
	var name_label := UI.centered(Store.player_name(seat), int(px * 0.13), UI.PAPER, false)
	name_label.add_theme_color_override("font_outline_color", Color(0.03, 0.03, 0.06))
	name_label.add_theme_constant_override("outline_size", 12)
	name_label.clip_text = true
	name_label.position = Vector2(px * 0.12, px * 0.7)
	name_label.size = Vector2(px * 0.76, px * 0.2)
	tile.add_child(name_label)
	return tile


func _place() -> void:
	var count := _others.size()
	# The frame is narrower than the square it sits in, so tiles may overlap a little.
	var step := minf(other_size * 0.9, size.x / maxf(1.0, count))
	for index in count:
		var from_middle := index - (count - 1) / 2.0
		var tile := _others[index]
		tile.position = Vector2(size.x / 2.0 + from_middle * step - other_size / 2.0,
			fan_top + from_middle * from_middle * fan_drop)
		tile.rotation_degrees = from_middle * fan_degrees
	_own.position = Vector2((size.x - own_size) / 2.0, size.y - own_size)
	var gap_top := fan_top + other_size
	_versus.position = Vector2(0, gap_top)
	_versus.size = Vector2(size.x, maxf(120.0, size.y - own_size - gap_top))
