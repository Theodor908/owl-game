## The menu opened from inside a match: resume, concede, settings, quit.
## It covers the screen until it is closed and frees itself. Holds no rules:
## "concede" only tells `main.gd` that the player wants to leave the match.
extends Control

signal concede

## "menu", "concede" (asking to confirm) or "settings".
var _page := "menu"


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_build()


func _build() -> void:
	UI.clear(self)
	var dim := Button.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	UI.face(dim, UI.box(Color(0.03, 0.03, 0.05, 0.78), 0, 0), 0.0)
	dim.pressed.connect(queue_free)
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(centre)
	var sheet := UI.panel(UI.INK_RAISED, 16, 30)
	sheet.custom_minimum_size.x = 760
	centre.add_child(sheet)
	var stack := UI.column(16)
	sheet.add_child(stack)
	match _page:
		"concede":
			stack.add_child(UI.centered("Concede?", UI.GLANCE, UI.GILT, false))
			stack.add_child(UI.centered("You leave this match and return to the main menu. The match is not saved.", 30))
			stack.add_child(_danger("Concede", _concede))
			stack.add_child(UI.stamp("Keep playing", _show.bind("menu"), Vector2(0, 108), 40, UI.INK))
		"settings":
			stack.add_child(UI.centered("Settings", UI.GLANCE, UI.GILT, false))
			stack.add_child(UI.volume("Music", Sound.instance.music_volume, Sound.instance.set_music_volume))
			stack.add_child(UI.volume("Effects", Sound.instance.effects_volume, Sound.instance.set_effects_volume))
			stack.add_child(UI.stamp("Sound: %s" % ("off" if Sound.instance.muted else "on"), _toggle_mute, Vector2(0, 108), 40, UI.INK))
			stack.add_child(UI.stamp("Back", _show.bind("menu"), Vector2(0, 108), 40, UI.INK))
		_:
			stack.add_child(UI.centered("Game menu", UI.GLANCE, UI.GILT, false))
			stack.add_child(UI.stamp("Resume", queue_free, Vector2(0, 108), 40, UI.INK))
			stack.add_child(_danger("Concede", _show.bind("concede")))
			stack.add_child(UI.stamp("Settings", _show.bind("settings"), Vector2(0, 108), 40, UI.INK))
			stack.add_child(UI.stamp("Quit game", func() -> void: get_tree().quit(), Vector2(0, 108), 40, UI.INK))


## A plate like the others, in deep red: for the one choice that ends the match.
func _danger(text: String, on_press: Callable) -> Button:
	var plate := UI.stamp(text, on_press, Vector2(0, 108), 40, UI.INK)
	var red := UI.skin("plate_danger", 40, 34, 10)
	if red != null:
		UI.face_skin(plate, red)
	return plate


func _show(page: String) -> void:
	_page = page
	_build()


func _concede() -> void:
	concede.emit()
	queue_free()


func _toggle_mute() -> void:
	Sound.instance.set_muted(not Sound.instance.muted)
	_build()
