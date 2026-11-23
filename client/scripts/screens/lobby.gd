## The play menu, after "Play" on the home screen. The owl who runs the quiz
## is the painted backdrop (`backgrounds/lobby.jpg`, set by `main.gd`); over
## it sit a few words at the foot and, on the right, the player's name and
## the round "Play" button. "Play" asks which kind of match: a lobby with
## friends online, or offline against bots. Holds no rules.
extends Control

signal play
signal back
signal collection
## The player wants a lobby with friends.
signal friends
## The player wants the guided first match.
signal tutorial

@export var side_width := 460
@export var play_size := 270
@export var words_width := 980

## "" or "choose": whether the choice of match is showing.
var _sheet := ""


func _ready() -> void:
	_build()


func _build() -> void:
	UI.clear(self)
	var right := UI.panel(UI.INK_RAISED, 16, 22)
	right.set_anchors_preset(Control.PRESET_RIGHT_WIDE)
	right.offset_left = -side_width
	add_child(right)
	var stack := UI.column(12)
	right.add_child(stack)
	stack.add_child(UI.centered(Store.my_name, 46, UI.GILT, false))
	stack.add_child(UI.spacer(0, 0, true))
	stack.add_child(_play_button())
	stack.add_child(UI.spacer(0, 0, true))
	stack.add_child(UI.stamp("How to play", func() -> void: tutorial.emit(), Vector2(0, 88), 34, UI.INK))
	stack.add_child(UI.stamp("Collection", func() -> void: collection.emit(), Vector2(0, 88), 34, UI.INK))
	stack.add_child(UI.stamp("Back", func() -> void: back.emit(), Vector2(0, 88), 34, UI.INK))

	# Foot of the picture: one line on what the game is.
	var words := UI.panel(Color(UI.INK, 0.72), 16, 14)
	words.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	words.grow_vertical = Control.GROW_DIRECTION_BEGIN
	# Clear of the feathers in the border's corner.
	words.offset_left = 110
	words.offset_right = 110
	words.offset_top = -20
	words.offset_bottom = -20
	words.custom_minimum_size = Vector2(words_width, 0)
	words.add_child(UI.centered("Answer better than your opponent to hurt them. The last one standing wins.", 32, UI.PAPER))
	add_child(words)

	if _sheet != "":
		_add_sheet()


## The round painted "Play" button (`buttons/play.png`); a plain disc until
## the picture is there.
func _play_button() -> Button:
	var picture := UI.art("buttons/play.png")
	var ask := _show_sheet.bind("choose")
	var go: Button
	if picture == null:
		go = UI.medallion("Play", "", ask, play_size, UI.VERDIGRIS_DEEP, 72)
	else:
		go = UI.emblem(picture, "", "", ask, Vector2(play_size, play_size))
		var words := UI.centered("Play", 70, UI.PAPER, false)
		words.add_theme_color_override("font_outline_color", Color(0.02, 0.2, 0.16))
		words.add_theme_constant_override("outline_size", 14)
		UI.fill(words)
		go.add_child(words)
	go.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	return go


## Opens the match choice. Also used by the development autoplayer.
func choose_match() -> void:
	_show_sheet("choose")


func _show_sheet(sheet: String) -> void:
	_sheet = sheet
	_build()


## The sheet over the screen: the choice of match.
func _add_sheet() -> void:
	var dim := Button.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.offset_left = -200
	dim.offset_right = 200
	dim.offset_top = -200
	dim.offset_bottom = 200
	UI.face(dim, UI.box(Color(0.03, 0.03, 0.05, 0.78), 0, 0), 0.0)
	dim.pressed.connect(_show_sheet.bind(""))
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(centre)
	var sheet := UI.panel(UI.INK_RAISED, 16, 28)
	sheet.custom_minimum_size.x = 760
	centre.add_child(sheet)
	var stack := UI.column(18)
	sheet.add_child(stack)
	match _sheet:
		"choose":
			stack.add_child(UI.centered("Choose a match", UI.GLANCE, UI.GILT, false))
			# Until it has been played once, the guided match is offered first.
			if not Tutorial.done():
				stack.add_child(UI.stamp("Guided first match
learn as you play", func() -> void: tutorial.emit(), Vector2(0, 150), 38, UI.VERDIGRIS))
			stack.add_child(UI.stamp("Play with friends\nonline, by lobby code", func() -> void: friends.emit(), Vector2(0, 150), 38, UI.INK))
			stack.add_child(UI.stamp("Offline\nagainst five bots", func() -> void: play.emit(), Vector2(0, 150), 38, UI.VERDIGRIS))
			stack.add_child(UI.stamp("Cancel", _show_sheet.bind(""), Vector2(0, 96), 34, UI.INK))
