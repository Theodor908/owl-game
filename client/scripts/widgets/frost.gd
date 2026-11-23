## A sheet of ice over the shop row while the shop is reserved. It covers
## the rectangle it is given, not the shape of the board underneath. It
## grows in from the edges when the shop is reserved and melts back when it
## is released. Holds no rules: it shows what `frozen` says.
class_name Frost
extends ColorRect

const SHADER := preload("res://shaders/frost.gdshader")

## How long the ice takes to grow over the shop, and to melt away, in seconds.
@export var freeze_time := 0.7
@export var thaw_time := 0.45
@export var freeze_curve := Tween.EASE_OUT
@export var tint := Color(0.62, 0.86, 1.0)
@export_range(0.0, 1.0) var opacity := 0.34
@export var corner_radius := 40.0

var _tween: Tween


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	material = ShaderMaterial.new()
	(material as ShaderMaterial).shader = SHADER
	_set_progress(0.0)


func _ready() -> void:
	var sheet := material as ShaderMaterial
	sheet.set_shader_parameter("tint", tint)
	sheet.set_shader_parameter("opacity", opacity)
	sheet.set_shader_parameter("corner_radius", corner_radius)
	resized.connect(func() -> void: sheet.set_shader_parameter("rect_size", size))
	sheet.set_shader_parameter("rect_size", size)


## Shows the ice as it should be now. `was` is what it showed before, so a
## change plays as growing or melting and anything else shows at once.
func show_state(frozen: bool, was: bool) -> void:
	if frozen == was:
		_set_progress(1.0 if frozen else 0.0)
		return
	_set_progress(0.0 if frozen else 1.0)
	if _tween != null:
		_tween.kill()
	_tween = create_tween()
	_tween.tween_method(_set_progress, 0.0 if frozen else 1.0, 1.0 if frozen else 0.0, freeze_time if frozen else thaw_time) \
		.set_ease(freeze_curve if frozen else Tween.EASE_IN).set_trans(Tween.TRANS_CUBIC)


func _set_progress(value: float) -> void:
	(material as ShaderMaterial).set_shader_parameter("progress", value)
	visible = value > 0.0
