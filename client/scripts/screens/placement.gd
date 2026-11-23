## Shown when the local player's match is over, by elimination or at the end:
## the hero in the painted crest (`skin/crest.png`) with the place on its
## plaque. A press brings up what to do next: watch the others, see the
## standings, or leave. Holds no rules.
extends Control

signal spectate
signal standings
signal leave

## Where things sit on the crest, measured on the 885 x 900 picture.
const CREST := Vector2(885, 900)
const NICHE := Rect2(311, 174, 265, 457)
const PLAQUE := Rect2(190, 650, 505, 100)
const SMALL_PLAQUE := Rect2(310, 776, 265, 62)
const PLACES: Array[String] = ["", "1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"]

@export var crest_height := 900

## The choices are showing.
var _asked := false


func _ready() -> void:
	refresh()


func refresh() -> void:
	UI.clear(self)
	var me := Store.me()
	var k := crest_height / CREST.y
	var holder := CenterContainer.new()
	holder.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(holder)
	var crest := Button.new()
	UI.bare(crest)
	crest.custom_minimum_size = CREST * k
	crest.pressed.connect(_ask)
	holder.add_child(crest)

	# The portrait fills the niche, behind the painted frame.
	var niche := Control.new()
	niche.mouse_filter = Control.MOUSE_FILTER_IGNORE
	niche.position = NICHE.position * k
	niche.size = NICHE.size * k
	var hero: Dictionary = {} if me["hero"] == null else Store.hero(me["hero"])
	var face := UI.Shape.new("arch", Color(0.05, 0.06, 0.1))
	var picture := UI.art("heroes/unknown.jpg") if hero.is_empty() else UI.hero_picture(hero)
	if picture == null:
		picture = UI.art("heroes/unknown.jpg")
		face.color = UI.UNKNOWN_TINT
	if picture != null:
		face.texture = picture
		if face.color != UI.UNKNOWN_TINT:
			face.color = Color.WHITE
	niche.add_child(face)
	var painted := UI.art("skin/crest.png")
	if painted != null:
		# The niche of the painting is black; cut it out by drawing the
		# portrait over the painting instead of under it.
		var image := TextureRect.new()
		image.texture = painted
		image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		image.stretch_mode = TextureRect.STRETCH_SCALE
		image.mouse_filter = Control.MOUSE_FILTER_IGNORE
		UI.fill(image)
		crest.add_child(image)
	crest.add_child(niche)

	var place := "" if me["placement"] == null else "%s place" % PLACES[clampi(int(me["placement"]), 0, PLACES.size() - 1)]
	if place == "":
		place = "Eliminated"
	var title := UI.centered(place, int(64 * k), UI.PAPER, false)
	title.position = PLAQUE.position * k
	title.size = PLAQUE.size * k
	crest.add_child(title)
	var who := UI.centered(Store.my_name, int(34 * k), UI.PAPER_SHADE, false)
	who.position = SMALL_PLAQUE.position * k
	who.size = SMALL_PLAQUE.size * k
	crest.add_child(who)

	var hint := UI.centered("Tap to continue", 30, UI.SLATE, false)
	hint.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	hint.offset_top = -44
	hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(hint)

	if _asked:
		_add_choices()


## Opens the choices. Also used by the development autoplayer.
func _ask() -> void:
	_asked = true
	refresh()


func _add_choices() -> void:
	var over := String(Store.view["phase"]) == "Finished"
	var dim := Button.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.offset_left = -300
	dim.offset_right = 300
	dim.offset_top = -300
	dim.offset_bottom = 300
	UI.face(dim, UI.box(Color(0.03, 0.03, 0.05, 0.78), 0, 0), 0.0)
	dim.pressed.connect(func() -> void:
		_asked = false
		refresh())
	add_child(dim)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	centre.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(centre)
	var sheet := UI.panel(UI.INK_RAISED, 16, 30)
	sheet.custom_minimum_size.x = 640
	centre.add_child(sheet)
	var stack := UI.column(16)
	sheet.add_child(stack)
	stack.add_child(UI.centered("The match is over" if over else "You are out", UI.GLANCE, UI.PAPER, false))
	if not over:
		stack.add_child(UI.stamp("Spectate", func() -> void: spectate.emit(), Vector2(0, 120), 42, UI.VERDIGRIS))
	stack.add_child(UI.stamp("Standings", func() -> void: standings.emit(), Vector2(0, 108), 40, UI.INK))
	stack.add_child(UI.stamp("Leave", func() -> void: leave.emit(), Vector2(0, 108), 40, UI.INK))
