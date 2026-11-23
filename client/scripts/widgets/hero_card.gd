## A hero drawn large: the portrait in the painted hero frame, and the sheet
## of facts shown when a hero is held.
class_name HeroCard

## Where things sit on `assets/frames/hero.png`, measured on the 900 x 1181 picture.
const FRAME := Vector2(900, 1181)
const WINDOW := Rect2(200, 205, 500, 800)
## Centres of the two round sockets at the foot of the frame: health, category bonus.
const SOCKETS: Array[Vector2] = [Vector2(117, 1085), Vector2(783, 1085)]


## The hero's picture behind the painted frame, `height` pixels tall, with
## health and category bonus on the two sockets at its foot.
static func framed(hero: Dictionary, height: float) -> Control:
	var k := height / FRAME.y
	var box := Control.new()
	box.custom_minimum_size = FRAME * k
	box.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	box.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE

	var window := Control.new()
	window.mouse_filter = Control.MOUSE_FILTER_IGNORE
	window.position = WINDOW.position * k
	window.size = WINDOW.size * k
	var tint: Color = UI.CATEGORY_TINTS[int(hero["category"])].darkened(0.35)
	var face := UI.Shape.new("arch", tint)
	var picture := UI.hero_picture(hero)
	if picture != null:
		face.texture = picture
		face.color = Color.WHITE
	var unknown := UI.art("heroes/unknown.jpg") if picture == null else null
	if unknown != null:
		face.texture = unknown
		face.color = UI.UNKNOWN_TINT
	window.add_child(face)
	if picture == null and unknown == null:
		var initials := UI.centered(UI.short_name(hero).left(2), int(220 * k), UI.PAPER, false)
		UI.fill(initials)
		window.add_child(initials)
	box.add_child(window)

	var frame := UI.art("frames/hero.png")
	if frame != null:
		var ring := TextureRect.new()
		ring.texture = frame
		ring.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		ring.stretch_mode = TextureRect.STRETCH_SCALE
		ring.mouse_filter = Control.MOUSE_FILTER_IGNORE
		UI.fill(ring)
		box.add_child(ring)

	var marks: Array[String] = [str(int(hero["hp"])), "+%d%%" % int(hero["category_bonus"])]
	for index in marks.size():
		var words := UI.centered(marks[index], int((64 if index == 0 else 46) * k), UI.PAPER, false)
		words.add_theme_color_override("font_outline_color", Color(0.1, 0.07, 0.04))
		words.add_theme_constant_override("outline_size", 8)
		words.position = (SOCKETS[index] - Vector2(70, 50)) * k
		words.size = Vector2(140, 100) * k
		box.add_child(words)
	return box


## Who the hero is and what they do. `state` holds extra lines, such as a
## player's name and health in a running match. Three text styles only: the
## name, the gilt lines of numbers, and the body.
static func facts(hero: Dictionary, state := "", width := 460) -> Control:
	var sheet := UI.panel(UI.INK_RAISED, 16, 32)
	sheet.custom_minimum_size.x = UI.t(width)
	sheet.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var stack := UI.column(8)
	sheet.add_child(stack)
	var name_label := UI.label(String(hero["name"]), UI.t(44 if String(hero["name"]).length() <= 16 else 36))
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD
	stack.add_child(name_label)
	var numbers := "+%d%% %s" % [int(hero["category_bonus"]), UI.category_label(hero["category"])]
	if state != "":
		numbers = state + "\n" + numbers
	stack.add_child(UI.label(numbers, UI.t(28), UI.GILT))
	var ability := Ability.of(hero)
	# A hero with no ability shows no ability lines.
	if ability != null:
		stack.add_child(UI.label("%s. %s" % [ability.name, ability.text], UI.t(30), Color("e6eaf2")))
		stack.add_child(UI.label(ability.terms(), UI.t(28), UI.GILT))
	return sheet


## The framed hero with the facts beside it: what a held hero shows.
static func info(hero: Dictionary, state := "") -> Control:
	var line := UI.row(20)
	line.add_child(framed(hero, 520))
	line.add_child(facts(hero, state, 600))
	return line
