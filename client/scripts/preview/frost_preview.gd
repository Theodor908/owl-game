## Preview of the shop ice: a row of stand-in cards that is reserved and
## released over and over. Run this scene on its own to tune `Frost`.
extends Control

## Seconds between reserving and releasing.
@export var hold := 1.6

var _frozen := false
var _ice: Frost


func _ready() -> void:
	var back := ColorRect.new()
	back.color = Color("141a33")
	back.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(back)
	var tray := PanelContainer.new()
	tray.add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	tray.custom_minimum_size = Vector2(1000, 240)
	tray.set_anchors_and_offsets_preset(Control.PRESET_CENTER, Control.PRESET_MODE_MINSIZE)
	add_child(tray)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 24)
	tray.add_child(row)
	var colours := [Color("c9a24a"), Color("3f8f7a"), Color("8a3b3b"), Color("4a5fa8"), Color("9a7bc4")]
	for colour in colours:
		var card := ColorRect.new()
		card.color = colour
		card.custom_minimum_size = Vector2(150, 200)
		row.add_child(card)
	_ice = Frost.new()
	tray.add_child(_ice)
	var timer := Timer.new()
	timer.wait_time = hold
	timer.autostart = true
	timer.timeout.connect(_toggle)
	add_child(timer)
	_toggle()


func _toggle() -> void:
	_ice.show_state(not _frozen, _frozen)
	_frozen = not _frozen
