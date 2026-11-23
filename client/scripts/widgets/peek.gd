## Press and hold for more: holding a card, a hero or a button shows its
## details beside it, and letting go hides them again. One instance lives in
## the scene (added by `main.gd`); anything can register with `Peek.attach`.
class_name Peek
extends CanvasLayer

static var instance: Peek

## How long a press must rest before the details appear, in milliseconds.
@export var hold_ms := 320
## How far the finger may wander and still count as resting, in pixels.
@export var slack := 16.0
## Gap between the held thing and its details.
@export var gap := 28.0
## The details stay this far from the top and bottom, clear of the painted border.
@export var edge := 64.0
## How much the rest of the screen darkens while details are up.
@export var dim_strength := 0.35
## Details shown in the middle of the screen are enlarged up to this much,
## as far as the screen has room.
@export var centre_grow := 1.4

var _source: Control
var _build: Callable
var _since := 0
var _at := Vector2.ZERO
var _shown: Control
var _dim: ColorRect
## The press that just ended showed details, so it must not count as a tap.
var _swallow := false


func _init() -> void:
	layer = 50
	instance = self


## Holding `control` shows what `build` returns (a Control) beside it, or
## large in the middle of the screen when `centred`.
static func attach(control: Control, build: Callable, centred := false) -> void:
	control.set_meta("peek", build)
	control.set_meta("peek_centred", centred)
	control.gui_input.connect(func(event: InputEvent) -> void:
		if instance != null and event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			instance._begin(control, build, event.global_position))


## Wraps a tap handler so the release that ends a hold does not also tap.
static func guard(action: Callable) -> Callable:
	return func() -> void:
		if not consume():
			action.call()


## True once after a hold showed details.
static func consume() -> bool:
	if instance == null:
		return false
	var was := instance._swallow
	instance._swallow = false
	return was


## A small sheet with a title and a paragraph, for things that are not cards.
static func note(title: String, text: String, width := 560) -> Control:
	var sheet := UI.panel(UI.INK_RAISED, 16, 32)
	sheet.custom_minimum_size.x = UI.t(width)
	var stack := UI.column(10)
	sheet.add_child(stack)
	stack.add_child(UI.label(title, UI.t(46), UI.PAPER))
	if text != "":
		stack.add_child(UI.label(text, UI.t(30), Color("e6eaf2")))
	return sheet


## Shows the details of the first thing under `root` that has any and for
## which `wanted` says yes. For development screenshots; a real hold needs a finger.
func demo(root: Node, wanted: Callable) -> bool:
	for node in root.find_children("*", "Control", true, false):
		if node.has_meta("peek") and wanted.call(node):
			_begin(node, node.get_meta("peek"), get_viewport().get_mouse_position())
			_show()
			return true
	return false


## Hides whatever `demo` showed.
func end_demo() -> void:
	_hide()
	_source = null


func _begin(control: Control, build: Callable, at: Vector2) -> void:
	_hide()
	_source = control
	_build = build
	_since = Time.get_ticks_msec()
	_at = at
	_swallow = false


func _process(_delta: float) -> void:
	if _source == null:
		return
	if not is_instance_valid(_source) or not _source.is_inside_tree():
		_hide()
		_source = null
		return
	var pointer := get_viewport().get_mouse_position()
	var moved := pointer.distance_to(_at) > slack
	if _shown != null and moved:
		_slide_to(pointer)
	if get_viewport().gui_is_dragging() or (_shown == null and moved):
		_hide()
		_source = null
	elif _shown == null and Time.get_ticks_msec() - _since >= hold_ms:
		_show()


## While details are up, sliding the finger onto a neighbour that also has
## details (another portrait of the player list, say) shows that one instead.
func _slide_to(pointer: Vector2) -> void:
	var row := _source.get_parent()
	if row == null:
		return
	for other in row.get_children():
		if other != _source and other is Control and other.has_meta("peek") and other.is_visible_in_tree() \
				and (other as Control).get_global_rect().has_point(pointer):
			_hide()
			_source = other
			_build = other.get_meta("peek")
			_at = pointer
			_show()
			return


func _input(event: InputEvent) -> void:
	var released := false
	if event is InputEventMouseButton:
		released = not event.pressed and event.button_index == MOUSE_BUTTON_LEFT
	elif event is InputEventScreenTouch:
		released = not event.pressed
	if not released:
		return
	if _shown != null:
		_swallow = true
	_hide()
	_source = null


func _show() -> void:
	var content: Control = _build.call()
	if content == null:
		_source = null
		return
	_dim = ColorRect.new()
	_dim.color = Color(0.02, 0.02, 0.04, dim_strength)
	_dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	_dim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_dim)
	_shown = content
	content.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(content)
	# Wrapped text only knows its height once it has a width, so the sheet is
	# laid out for two frames at its own width before it is placed and shown.
	content.modulate.a = 0.0
	for pass_number in 2:
		content.size = Vector2(content.get_combined_minimum_size().x, 0)
		await get_tree().process_frame
		if not is_instance_valid(content) or _shown != content or not is_instance_valid(_source):
			return
	content.reset_size()
	content.modulate.a = 1.0
	var view := get_viewport().get_visible_rect().size
	# Details taller or wider than the screen are shrunk to fit, never cut off.
	var room := Vector2(view.x - 2.0 * gap, view.y - 2.0 * edge)
	var centred := bool(_source.get_meta("peek_centred", false))
	var fit := minf(centre_grow if centred else 1.0, minf(room.x / content.size.x, room.y / content.size.y))
	content.scale = Vector2(fit, fit)
	var size := content.size * fit
	if centred:
		content.position = (view - size) * 0.5
		return
	var held := _source.get_global_rect()
	# Beside the held thing, on whichever side has room; never under the finger.
	var x := held.end.x + gap
	if x + size.x > view.x - gap:
		x = held.position.x - gap - size.x
	if x < gap:
		x = (view.x - size.x) * 0.5
	var y := clampf(held.get_center().y - size.y * 0.5, edge, maxf(edge, view.y - size.y - edge))
	content.position = Vector2(x, y)


func _hide() -> void:
	if _shown != null:
		_shown.queue_free()
		_shown = null
	if _dim != null:
		_dim.queue_free()
		_dim = null
