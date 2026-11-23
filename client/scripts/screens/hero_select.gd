## Hero select: each player is dealt two heroes and picks one. The offers
## stand side by side in the painted hero frame with their names underneath;
## a tap marks one, "Confirm" picks it, and holding one shows what it does.
## The offer comes from the host (`me.hero_offer`); this screen holds no rule.
extends Control

## Height of the painted frame around each offered hero.
@export var frame_height := 540
## How much the marked hero grows.
@export var marked_scale := 1.06
@export var glow := Color("52e08a")

var _timer: Label
var _error := ""
## The offer the player has tapped, or -1.
var _marked := -1


func _ready() -> void:
	refresh()


func _process(_delta: float) -> void:
	if _timer != null:
		_timer.text = str(UI.seconds_left(Store.view.get("phase_deadline")))


func refresh() -> void:
	UI.clear(self)
	var me := Store.me()
	var mine = me["hero"]

	var page := UI.column(12)
	page.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(page)

	# The banner, with the timer on a plate at the right edge.
	var top := Control.new()
	top.custom_minimum_size.y = 120
	var banner := UI.panel(UI.INK_RAISED, 16, 14)
	banner.custom_minimum_size = Vector2(760, 104)
	banner.set_anchors_preset(Control.PRESET_CENTER)
	banner.grow_horizontal = Control.GROW_DIRECTION_BOTH
	banner.grow_vertical = Control.GROW_DIRECTION_BOTH
	banner.add_child(UI.centered("Your hero" if mine != null else "Choose your hero", UI.GLANCE, UI.PAPER, false))
	top.add_child(banner)
	var clock := UI.panel(UI.INK_RAISED, 16, 8)
	clock.custom_minimum_size = Vector2(170, 104)
	clock.set_anchors_preset(Control.PRESET_CENTER_RIGHT)
	clock.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	clock.grow_vertical = Control.GROW_DIRECTION_BOTH
	_timer = UI.centered("", UI.GLANCE, UI.PAPER, false)
	clock.add_child(_timer)
	top.add_child(clock)
	page.add_child(top)

	var body := UI.row(90)
	body.alignment = BoxContainer.ALIGNMENT_CENTER
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	if mine != null:
		body.add_child(_option(int(mine), true))
	else:
		for id in me.get("hero_offer", []):
			body.add_child(_option(int(id), false))
	Tutorial.mark(body, "hero_offer")
	page.add_child(body)

	var foot := Control.new()
	foot.custom_minimum_size.y = 140
	var line: Control
	if mine != null:
		line = UI.centered("Chosen. Waiting for the others.", 40, UI.GILT, false)
	elif _error != "":
		line = UI.centered(_error, 36, UI.VERMILION, false)
	elif _marked < 0:
		line = UI.centered("Tap a hero to choose. Hold one to read what they do.", 34, UI.PAPER_SHADE, false)
	else:
		line = UI.stamp("Confirm", _pick, Vector2(420, 130), UI.GLANCE, UI.VERDIGRIS)
	line.set_anchors_preset(Control.PRESET_CENTER)
	line.grow_horizontal = Control.GROW_DIRECTION_BOTH
	line.grow_vertical = Control.GROW_DIRECTION_BOTH
	foot.add_child(line)
	page.add_child(foot)


## One offered hero: the framed portrait with the name under it.
func _option(index: int, chosen: bool) -> Control:
	var hero: Dictionary = Store.heroes[index]
	var marked := chosen or index == _marked
	var stack := UI.column(6)
	stack.alignment = BoxContainer.ALIGNMENT_CENTER
	stack.size_flags_vertical = Control.SIZE_SHRINK_CENTER

	var button := Button.new()
	UI.bare(button)
	var size := HeroCard.FRAME * (frame_height / HeroCard.FRAME.y)
	button.custom_minimum_size = size
	button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	if marked:
		# A soft light behind the frame: wide and faint, then narrow and bright.
		for step in 4:
			var halo := UI.Shape.new("arch", Color.TRANSPARENT, Color(glow, 0.16 + 0.14 * step), 30.0 - 7.0 * step)
			UI.fill(halo, 10)
			button.add_child(halo)
	var picture := HeroCard.framed(hero, frame_height)
	button.add_child(picture)
	if marked:
		button.pivot_offset = size * 0.5
		button.scale = Vector2(marked_scale, marked_scale)
	UI.quiet(button)
	if not chosen:
		button.pressed.connect(Peek.guard(_mark.bind(index)))
	Peek.attach(button, HeroCard.facts.bind(hero, "", 520))
	stack.add_child(button)

	var name_label := UI.centered(String(hero["name"]), 44, UI.PAPER, false)
	name_label.add_theme_color_override("font_outline_color", Color(0.05, 0.04, 0.03))
	name_label.add_theme_constant_override("outline_size", 10)
	stack.add_child(name_label)
	stack.add_child(UI.centered(String(hero["title"]), 28, UI.PAPER_SHADE, false))
	return stack


func _mark(index: int) -> void:
	_marked = index
	_error = ""
	refresh()


func _pick() -> void:
	_error = Transport.send({"PickHero": {"player": Store.my_id(), "hero": _marked}}).capitalize()
	refresh()
