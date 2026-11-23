## The first screen of the app: the lid of the box, with the menu on the
## round plate in its middle. Holds no rules and starts nothing by itself;
## it only says where the player wants to go.
extends Control

## "play", "collection" or "profile": the screen to open.
signal chosen(where: String)

@export var button_height := 116
## The owl emblem at the top, and how far the menu sits below the middle to leave it room.
@export var logo_size := 330
@export var logo_top := -34
@export var menu_drop := 170
## Widths of the menu plates from top to bottom: the outer ones are
## narrower, so the stack follows the round plate behind it.
@export var button_widths: Array[int] = [540, 540, 540, 540]

var _sheet := ""


func _ready() -> void:
	_build()


func _build() -> void:
	UI.clear(self)
	# The game's emblem stands in for a written title; the name shows until the picture is there.
	var emblem := UI.art("skin/logo.png")
	if emblem != null:
		var crest := TextureRect.new()
		crest.texture = emblem
		crest.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		crest.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		crest.mouse_filter = Control.MOUSE_FILTER_IGNORE
		crest.set_anchors_preset(Control.PRESET_CENTER_TOP)
		crest.offset_left = -logo_size / 2.0
		crest.offset_right = logo_size / 2.0
		crest.offset_top = logo_top
		crest.offset_bottom = logo_top + logo_size
		add_child(crest)
	else:
		var title := UI.centered("OWL", 150, UI.GILT, false)
		title.set_anchors_preset(Control.PRESET_TOP_WIDE)
		title.offset_top = -10
		title.offset_bottom = 170
		add_child(title)

	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.offset_top = menu_drop
	add_child(centre)
	var menu := UI.column(12)
	centre.add_child(menu)
	var entries := [
		["Play", func() -> void: chosen.emit("play")],
		["Collection", func() -> void: chosen.emit("collection")],
		["Profile", func() -> void: chosen.emit("profile")],
		["Settings", _open.bind("settings")],
	]
	for index in entries.size():
		var plate := _plate(String(entries[index][0]), entries[index][1], button_widths[index])
		plate.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		menu.add_child(plate)

	if _sheet != "":
		_add_sheet()


## A menu plate: the same slate and silver as every other button.
func _plate(text: String, on_press: Callable, width: int) -> Button:
	return UI.stamp(text, on_press, Vector2(width, button_height), 60, UI.INK)


func _open(sheet: String) -> void:
	_sheet = sheet
	_build()


func _close() -> void:
	_sheet = ""
	_build()


func _add_sheet() -> void:
	var dim := Button.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	UI.face(dim, UI.box(Color(0.03, 0.03, 0.05, 0.75), 0, 0), 0.0)
	dim.pressed.connect(_close)
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(centre)
	var sheet := UI.panel(UI.INK_RAISED, 16, 28)
	sheet.custom_minimum_size.x = 820
	centre.add_child(sheet)
	var stack := UI.column(18)
	sheet.add_child(stack)
	if _sheet == "settings":
		stack.add_child(UI.label("Settings", UI.GLANCE, UI.GILT))
		stack.add_child(UI.volume("Music", Sound.instance.music_volume, Sound.instance.set_music_volume))
		stack.add_child(UI.volume("Effects", Sound.instance.effects_volume, Sound.instance.set_effects_volume))
		stack.add_child(UI.stamp("Sound: %s" % ("off" if Sound.instance.muted else "on"), _toggle_mute, Vector2(0, 110), 40, UI.INK))
	else:
		stack.add_child(UI.label("Profile", UI.GLANCE, UI.GILT))
		stack.add_child(UI.label("Your name, as the other players see it:", 32))
		var field := LineEdit.new()
		field.text = Store.my_name
		field.max_length = Store.NAME_LENGTH
		field.custom_minimum_size.y = 96
		field.add_theme_font_size_override("font_size", 44)
		field.text_changed.connect(Store.set_my_name)
		stack.add_child(field)
		stack.add_child(UI.label("Ratings and match history come with accounts, later in the build.", 28, UI.SLATE))
	stack.add_child(UI.stamp("Close", _close, Vector2(0, 116), 36, UI.INK))


func _toggle_mute() -> void:
	Sound.instance.set_muted(not Sound.instance.muted)
	_build()

