## Shared look and small builders for the landscape interface, so every
## screen uses the same palette, sizes and parts. See docs/ui-research.md,
## sections 2.0, 3 and 4. Everything drawn here is a placeholder for art.
class_name UI

# Palette (docs/ui-research.md 4.2).
const INK := Color("141b2d")
const INK_RAISED := Color("1f2a44")
const PAPER := Color("f2e9d8")
const PAPER_SHADE := Color("d9cdb4")
const SLATE := Color("9aa3b8")
const GILT := Color("e0a526")
const VERDIGRIS := Color("2fa58c")
const VERDIGRIS_DEEP := Color("17695a")
const VERMILION := Color("e4572e")
const VERMILION_DEEP := Color("b23a16")

# Text sizes in reference pixels (1080 high).
const SMALLEST := 32
const BODY := 48
const GLANCE := 64
const NUMERAL := 96

# Margins of the content frame inside the 1920x1080 viewport.
const MARGIN_SIDE := 48
const MARGIN_TOP := 30
const MARGIN_BOTTOM := 76

## Tint per category, in the order of `tunables.categories`.
const CATEGORY_TINTS: Array[Color] = [
	Color("c9973b"), Color("6fa85a"), Color("8c9bb0"), Color("9a6bb5"), Color("5aa9d6"),
	Color("6c78d8"), Color("d86c7e"), Color("e08a6a"), Color("3fa7a0"), Color("c9c94a"),
]
## Two-letter stand-ins for the category emblems.
const CATEGORY_MARKS: Array[String] = ["Hi", "Fl", "Ca", "Fi", "Ph", "Pl", "Th", "An", "Ge", "Sp"]

## Card kind -> [two-letter stand-in for the seal glyph, family tint].
const KINDS := {
	"category": ["Bk", VERDIGRIS], "weights": ["Wt", VERDIGRIS],
	"damage": ["Dm", VERMILION], "bet": ["Bt", VERMILION], "counter": ["Cn", VERMILION],
	"armor": ["Ar", SLATE], "defense": ["Df", SLATE], "reflect": ["Rf", SLATE],
	"time": ["Tm", PAPER_SHADE], "tie": ["Ti", PAPER_SHADE],
	"economy": ["Ec", GILT], "healing": ["He", GILT],
	"difficulty": ["Dc", PAPER_SHADE],
	"help": ["Hp", INK_RAISED], "endgame": ["Eg", INK_RAISED],
	"legendary": ["Lg", INK_RAISED], "token": ["Tk", INK_RAISED],
}

## Card kind -> [the family's name, what cards of it do], to explain the
## two letters on a card's seal when it is opened or held.
const KIND_NAMES := {
	"category": ["Book", "Counts toward its category. Three of one category give a damage bonus on every right answer in it."],
	"weights": ["Weight", "Changes the pull of your categories on the questions."],
	"damage": ["Damage", "Adds damage when you win a battle."],
	"bet": ["Bet", "A wager: more damage when you win, more taken when you lose."],
	"counter": ["Counter", "Works against what the opponent has: armor, reflect or a Healer."],
	"armor": ["Armor", "Takes damage before your HP does. It holds a place on the board."],
	"defense": ["Defense", "Caps or softens the damage you take."],
	"reflect": ["Reflect", "Sends part of the damage you take back at the opponent."],
	"time": ["Time", "More seconds to answer."],
	"tie": ["Tie", "Acts when a battle ends level."],
	"economy": ["Economy", "Brings you gold."],
	"healing": ["Healing", "Gives back HP, never above your starting HP."],
	"difficulty": ["Difficulty", "Makes the questions of your battle easier or harder, for both players."],
	"help": ["Help", "Helps you on a question, such as by removing a wrong answer."],
	"endgame": ["Endgame", "Works only in the final duel of the last two players."],
	"legendary": ["Legendary", "A rare card with a strong effect."],
	"token": ["Token", "Made by another effect, never bought from the pool."],
}


## A card with a note under it on the family its seal stands for: "Bk ·
## Book" and what such cards do. Used where a card is opened to read.
static func with_kind_note(card: Control, def: Dictionary) -> Control:
	var kind := String(def.get("kind", ""))
	if not KIND_NAMES.has(kind):
		return card
	var stack := column(12)
	stack.alignment = BoxContainer.ALIGNMENT_CENTER
	stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
	card.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	stack.add_child(card)
	var sheet := panel(INK_RAISED, 14, 18)
	sheet.mouse_filter = Control.MOUSE_FILTER_IGNORE
	sheet.custom_minimum_size.x = maxf(card.get_combined_minimum_size().x, t(420))
	sheet.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	var words := column(4)
	words.mouse_filter = Control.MOUSE_FILTER_IGNORE
	sheet.add_child(words)
	var tint: Color = KINDS[kind][1] if KINDS.has(kind) else SLATE
	words.add_child(label("%s  ·  %s" % [KINDS.get(kind, ["??"])[0], KIND_NAMES[kind][0]], t(30), tint.lightened(0.35), false))
	words.add_child(label(KIND_NAMES[kind][1], t(24), Color("e6eaf2")))
	stack.add_child(sheet)
	return stack

## Corner rounding: panels multiply their radius by SOFTEN; stamps use STAMP_RADIUS.
const SOFTEN := 2.5
const STAMP_RADIUS := 36

## Health bars turn from green to red below this share of full health.
const LOW_HEALTH := 0.3

## Portraits narrower than this use the hero's close-up file.
const FACE_BELOW := 150

static var NOT_A_NUMBER := RegEx.create_from_string("[A-Za-z]{3,}")
## The display face (Grenze ExtraBold): titles, names, buttons and numbers.
## The body face (Libre Franklin) is the project's default font.
const HEADING_FONT := "res://assets/fonts/heading.ttf"
## Text at this size or larger is set in the display face.
const HEADING_FROM := 40
static var _display_font: Font

## How much the board and everything on it is enlarged on this screen. The
## layout is drawn for 16:9; a wider screen (most phones) has width to spare,
## and the cards grow into it. Set by `main.gd` from the shape of the screen.
static var zoom := 1.0
## How much small reading text (held details, tags, hints) is enlarged. Above
## 1 on phones, where the same layout is physically much smaller.
static var text_zoom := 1.0


## A length of the 16:9 layout, enlarged for this screen.
static func z(value: float) -> int:
	return int(round(value * zoom))


## A font size for small reading text on this device.
static func t(size: float) -> int:
	return int(round(size * text_zoom))
## The outline under light text: its colour, and its width as a share of the font size.
const OUTLINE_INK := Color("0b1020")
const OUTLINE_SHARE := 0.1
## Below this size the display face and the outline are not used: both muddy small text.
const DISPLAY_FLOOR := 30
## The one tint of the hooded figure that stands in for a missing portrait.
const UNKNOWN_TINT := Color(0.66, 0.72, 0.9)
## Numerals on the gold coin: dark brown with a pale rim.
const COIN_INK := Color("3a2408")
const COIN_RIM := Color("ffe9a8")


## The display face with lining, equal-width figures switched on; by default
## its numerals dip below the line like lowercase letters. Null until the
## font file is there.
static func display_font() -> Font:
	if _display_font == null and ResourceLoader.exists(HEADING_FONT):
		var face := FontVariation.new()
		face.base_font = load(HEADING_FONT)
		var server := TextServerManager.get_primary_interface()
		face.opentype_features = {server.name_to_tag("lnum"): 1, server.name_to_tag("tnum"): 1}
		_display_font = face
	return _display_font

const ROMAN: Array[String] = ["-", "I", "II", "III", "IV", "V", "VI", "VII", "VIII"]


static func label(text: String, size := BODY, color := PAPER, wrap := true) -> Label:
	var node := Label.new()
	node.text = text
	node.add_theme_font_size_override("font_size", size)
	node.add_theme_color_override("font_color", color)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	# Titles and numerals are set in the display face; the rest uses the
	# project's default font (project.godot, gui/theme/custom_font).
	if (size >= HEADING_FROM or (size >= DISPLAY_FLOOR and text != "" and not NOT_A_NUMBER.search(text))) and display_font() != null:
		node.add_theme_font_override("font", display_font())
	# Light text sits on the board or on painted art: a dark outline and a
	# soft shadow keep it readable. Dark text is on parchment and stays plain.
	# Small text gets the shadow only: an outline makes it heavy and blurred.
	if color.get_luminance() > 0.45 and color.a > 0.9:
		var outline := 0 if size < DISPLAY_FLOOR else maxi(3, int(size * OUTLINE_SHARE))
		if outline > 0:
			node.add_theme_color_override("font_outline_color", OUTLINE_INK)
			node.add_theme_constant_override("outline_size", outline)
		node.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.6))
		node.add_theme_constant_override("shadow_offset_x", 0)
		node.add_theme_constant_override("shadow_offset_y", 2 if outline == 0 else 3)
		node.add_theme_constant_override("shadow_outline_size", maxi(2, outline))
	if wrap:
		node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	return node


static func centered(text: String, size := BODY, color := PAPER, wrap := true) -> Label:
	var node := label(text, size, color, wrap)
	node.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	node.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	return node


static func box(color: Color, radius := 12, margin := 12, border := Color.TRANSPARENT, width := 0) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.set_corner_radius_all(radius)
	style.set_content_margin_all(margin)
	if width > 0:
		style.border_color = border
		style.set_border_width_all(width)
	return style


## A stretchable painted face from `assets/skin/`, or null when the file is
## missing. `side` and `top` are the parts of the picture that must not stretch.
static func skin(file: String, side: int, top: int, margin := 12) -> StyleBoxTexture:
	var picture := art("skin/%s.png" % file)
	if picture == null:
		return null
	var style := StyleBoxTexture.new()
	style.texture = picture
	style.texture_margin_left = side
	style.texture_margin_right = side
	style.texture_margin_top = top
	style.texture_margin_bottom = top
	style.set_content_margin_all(margin)
	return style


## The dark walnut plate with a brass edge: name plates, rows, bars. Falls
## back to a flat box.
static func plate(radius := 12, margin := 12) -> StyleBox:
	var painted := skin("plate", 40, 34, margin)
	return painted if painted != null else box(INK_RAISED, radius, margin)


## Gives a button a painted face for every state.
static func face_skin(button: Button, style: StyleBoxTexture) -> void:
	var down: StyleBoxTexture = style.duplicate()
	down.modulate_color = style.modulate_color.darkened(0.2)
	var off: StyleBoxTexture = style.duplicate()
	off.modulate_color = Color(style.modulate_color, 0.45)
	button.add_theme_stylebox_override("normal", style)
	button.add_theme_stylebox_override("hover", style)
	button.add_theme_stylebox_override("pressed", down)
	button.add_theme_stylebox_override("disabled", off)
	button.add_theme_stylebox_override("focus", StyleBoxEmpty.new())
	button.focus_mode = Control.FOCUS_NONE


## A small painted object from `assets/skin/` with a number printed on it:
## "coin", "health", "armor", or a wax seal with no text. Falls back to a disc.
static func token(file: String, text: String, px: int, size: int, ink := INK, fallback := GILT) -> Control:
	var picture := art("skin/%s.png" % file)
	if picture == null:
		return badge(text, px, fallback, ink, size, INK)
	var node := Control.new()
	node.custom_minimum_size = Vector2(px, px)
	node.size = Vector2(px, px)
	node.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var image := TextureRect.new()
	image.texture = picture
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.mouse_filter = Control.MOUSE_FILTER_IGNORE
	fill(image)
	node.add_child(image)
	if text != "":
		var coin := file == "coin"
		var words := centered(text, size, COIN_INK if coin else ink, false)
		words.add_theme_color_override("font_outline_color", COIN_RIM if coin else (Color(1, 1, 1, 0.55) if ink == INK else Color(0, 0, 0, 0.7)))
		words.add_theme_constant_override("outline_size", 6)
		fill(words)
		node.add_child(words)
	return node


## A brass medallion with a letter or number on it.
static func seal(text: String, px: int, size: int) -> Control:
	var picture := art("skin/seal.png")
	if picture == null:
		return badge(text, px, INK, PAPER, size)
	var node := Control.new()
	node.custom_minimum_size = Vector2(px, px)
	node.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var image := TextureRect.new()
	image.texture = picture
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.mouse_filter = Control.MOUSE_FILTER_IGNORE
	fill(image)
	node.add_child(image)
	var words := centered(text, size, PAPER, false)
	fill(words)
	node.add_child(words)
	return node


## Gives a button one face for every state, so its look is set by the caller
## and not by Godot's default theme.
static func face(button: Button, style: StyleBoxFlat, pressed_darken := 0.12) -> void:
	var down: StyleBoxFlat = style.duplicate()
	down.bg_color = style.bg_color.darkened(pressed_darken)
	button.add_theme_stylebox_override("normal", style)
	button.add_theme_stylebox_override("hover", style)
	button.add_theme_stylebox_override("pressed", down)
	button.add_theme_stylebox_override("disabled", style)
	button.add_theme_stylebox_override("focus", StyleBoxEmpty.new())
	button.focus_mode = Control.FOCUS_NONE


## A text button. Light faces get ink text, dark faces get paper text.
static func stamp(text: String, on_press: Callable, min_size: Vector2, size := SMALLEST, color := PAPER) -> Button:
	var node := Button.new()
	node.text = text
	node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	node.custom_minimum_size = min_size
	node.add_theme_font_size_override("font_size", size)
	# Button captions are set in the display face.
	if display_font() != null:
		node.add_theme_font_override("font", display_font())
	var ink := INK if color.get_luminance() > 0.45 else PAPER
	for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		node.add_theme_color_override(state, ink)
	node.add_theme_color_override("font_disabled_color", Color(ink, 0.4))
	# Confirm buttons are green enamel, quiet ones walnut; the rest stay flat.
	var painted: StyleBoxTexture = null
	if color == VERDIGRIS and min_size.y >= 100:
		painted = skin("primary", 66, 50, 10)
	elif (color == INK or color == INK_RAISED) and min_size.y >= 70:
		painted = skin("plate", 40, 34, 10)
	if painted != null:
		for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
			node.add_theme_color_override(state, PAPER)
		node.add_theme_color_override("font_disabled_color", Color(PAPER, 0.5))
		node.add_theme_color_override("font_outline_color", Color(0.05, 0.08, 0.05))
		node.add_theme_constant_override("outline_size", 8)
		face_skin(node, painted)
	else:
		face(node, box(color, STAMP_RADIUS, 10))
		node.add_theme_stylebox_override("disabled", box(Color(color, 0.45), STAMP_RADIUS, 10))
	node.pressed.connect(on_press)
	return node


## A line of text the player types: a name, an address, a password.
static func field(placeholder: String, text := "", secret := false) -> LineEdit:
	var node := LineEdit.new()
	node.placeholder_text = placeholder
	node.text = text
	node.secret = secret
	node.custom_minimum_size.y = 84
	node.add_theme_font_size_override("font_size", 34)
	node.add_theme_color_override("font_color", PAPER)
	node.add_theme_color_override("font_placeholder_color", Color(SLATE, 0.8))
	node.add_theme_color_override("caret_color", PAPER)
	node.add_theme_stylebox_override("normal", box(Color(0.04, 0.05, 0.1), 14, 16, Color("b9c6dc"), 2))
	node.add_theme_stylebox_override("focus", box(Color(0.04, 0.05, 0.1), 14, 16, Color("e8eef8"), 3))
	return node


static func panel(color := INK_RAISED, radius := 12, margin := 12) -> PanelContainer:
	var node := PanelContainer.new()
	var painted: StyleBox = null
	if color == INK_RAISED:
		painted = skin("plate", 40, 34, margin + 8)
	elif color == PAPER and radius >= 12:
		painted = skin("parchment", 60, 60, margin + 16)
	node.add_theme_stylebox_override("panel", painted if painted != null else box(color, int(radius * SOFTEN), margin))
	return node


static func column(separation := 16) -> VBoxContainer:
	var node := VBoxContainer.new()
	node.add_theme_constant_override("separation", separation)
	return node


static func row(separation := 16) -> HBoxContainer:
	var node := HBoxContainer.new()
	node.add_theme_constant_override("separation", separation)
	return node


static func spacer(width := 0, height := 0, expand := false) -> Control:
	var node := Control.new()
	node.custom_minimum_size = Vector2(width, height)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if expand:
		node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		node.size_flags_vertical = Control.SIZE_EXPAND_FILL
	return node


## Makes `node` fill its parent, `margin` pixels in from every edge. For
## content laid over a button, which is not a container.
static func fill(node: Control, margin := 0) -> void:
	node.set_anchors_preset(Control.PRESET_FULL_RECT)
	node.offset_left = margin
	node.offset_top = margin
	node.offset_right = -margin
	node.offset_bottom = -margin


## Removes every child at once, so a screen can be rebuilt from fresh state.
static func clear(node: Node) -> void:
	for child in node.get_children():
		node.remove_child(child)
		child.queue_free()


## Makes a subtree invisible to touch, so the control that owns it gets the tap.
static func quiet(node: Node) -> void:
	for child in node.get_children():
		if child is Control:
			(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
		quiet(child)


## Milliseconds left until a host deadline, never below zero.
static func ms_left(deadline: Variant) -> int:
	if deadline == null:
		return 0
	return maxi(0, int(deadline) - Transport.now())


static func seconds_left(deadline: Variant) -> int:
	return ceili(ms_left(deadline) / 1000.0)


static func roman(level: int) -> String:
	return ROMAN[clampi(level, 0, ROMAN.size() - 1)]


## "flora_and_fauna" -> "Flora and Fauna".
static func category_label(id: Variant) -> String:
	var words: Array[String] = []
	for word in String(Store.categories[int(id)]).split("_"):
		words.append(word if word == "and" else word.capitalize())
	return " ".join(words)


## A question's category with its difficulty: "History · Hard". The
## difficulty is the host's word for it ("Easy", "Medium", "Hard"); a view
## without one gives the category alone.
static func category_line(id: Variant, difficulty: Variant) -> String:
	var text := category_label(id)
	return text if difficulty == null or String(difficulty) == "" else "%s  ·  %s" % [text, String(difficulty)]


## A card's display name. Per-category cards carry the raw slug in their
## name ("Apprentice (history)"); show "History Apprentice" instead.
static func card_name(def: Dictionary) -> String:
	var text := String(def["name"])
	if def.get("category") != null and text.contains(" ("):
		return "%s %s" % [category_label(def["category"]), text.get_slice(" (", 0)]
	return text


## The first number in a card's text ("+1", "15%", "+2 s"), copied, never computed.
## What a card does, in the words of its data file; the golden version's
## words when `golden` and the card has one.
static func card_text(def: Dictionary, golden := false) -> String:
	if golden and def.get("golden_text") != null:
		return String(def["golden_text"])
	return String(def["text"])


static func card_value(def: Dictionary, golden := false) -> String:
	var pattern := RegEx.create_from_string("[+-]?\\d+( ?%| s\\b)?")
	var found := pattern.search(card_text(def, golden))
	return "" if found == null else found.get_string()


## A short name for a hero: "Newton", "Herophilus", "Lumiere".
static func short_name(hero: Dictionary) -> String:
	var words := String(hero["name"]).split(" ")
	if words.size() > 1 and (words[1] == "of" or words[-1] == "brothers"):
		return words[0]
	return words[-1]


## The small painted frame around every hero portrait
## (`assets/frames/hero_small.png`): its size, and the opening in it.
const HERO_FRAME := Vector2(394, 480)
const HERO_FRAME_WINDOW := Rect2(36, 43, 322, 406)
## How far the picture reaches under the frame's inner edge, in frame pixels.
const HERO_FRAME_TUCK := 8.0
## Tint of the frame around someone who is not the local player.
const HERO_FRAME_OTHER := Color(0.66, 0.7, 0.8)


## A hero portrait `px` tall in the small arched frame; the frame is narrower
## than tall and sits centred in a `px` square. `ring` picks the frame's
## tint: gilt for the local player, anything else for the others. Until a
## hero has a picture, a tinted plate with initials stands in. Without the
## frame file the old drawn shapes show: `radius` below zero a rounded
## square, anything else an arched niche. A portrait large enough to read
## carries the player's armor on a shield (`ArmorBadge`).
static func portrait(hero: Dictionary, px: int, ring := SLATE, ring_width := 4, radius := -1) -> Control:
	var node := Control.new()
	node.custom_minimum_size = Vector2(px, px)
	node.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	node.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var frame := art("frames/hero_small.png")
	var kind := "arch" if frame != null or radius >= 0 else "tile"
	var tint := SLATE.darkened(0.5) if hero.is_empty() else CATEGORY_TINTS[int(hero["category"])].darkened(0.3)
	var plate := Shape.new(kind, tint)
	var k := px / HERO_FRAME.y
	var left := (px - HERO_FRAME.x * k) / 2.0
	var holder: Control = node
	if frame != null:
		# The picture fills the opening and tucks a little under the frame.
		holder = Control.new()
		holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
		var window := HERO_FRAME_WINDOW.grow(HERO_FRAME_TUCK)
		holder.position = Vector2(left, 0) + window.position * k
		holder.size = window.size * k
		node.add_child(holder)
	holder.add_child(plate)
	var picture: Texture2D = null
	if not hero.is_empty():
		# Small portraits show the face; large ones the whole illustration.
		picture = hero_picture(hero, px < FACE_BELOW)
	var unknown := art("heroes/unknown.jpg") if picture == null else null
	if picture != null:
		plate.texture = picture
		plate.color = Color.WHITE
	elif unknown != null:
		# No portrait yet: a hooded figure in the hero's category colour.
		plate.texture = unknown
		plate.color = UNKNOWN_TINT
	else:
		var text := "?" if hero.is_empty() else short_name(hero).left(2)
		var initials := centered(text, maxi(20, int(px * 0.34)), PAPER, false)
		fill(initials)
		holder.add_child(initials)
	if frame == null:
		node.add_child(Shape.new(kind, Color.TRANSPARENT, ring, ring_width))
		ArmorBadge.attach(node, hero, px)
		return node
	var border := TextureRect.new()
	border.texture = frame
	border.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	border.stretch_mode = TextureRect.STRETCH_SCALE
	border.mouse_filter = Control.MOUSE_FILTER_IGNORE
	border.position = Vector2(left, 0)
	border.size = HERO_FRAME * k
	border.modulate = Color.WHITE if ring == GILT else HERO_FRAME_OTHER
	node.add_child(border)
	# The player's armor on a small shield in the lower corner, while above 0.
	ArmorBadge.attach(node, hero, px)
	return node


## A hero's painting: `heroes/<id>.jpg`, or `<id>_face.jpg` for the close
## crop used in small portraits. Null until it is painted.
static func hero_picture(hero: Dictionary, face := false) -> Texture2D:
	var suffix := "_face" if face else ""
	var own := art("heroes/%s%s.jpg" % [String(hero.get("id", "")), suffix])
	# Older files are named after the hero's kind, which is the same word in lower case.
	return own if own != null else art("heroes/%s%s.jpg" % [String(hero["kind"]).to_lower(), suffix])


## A square hero icon with a health bar set into its lower edge. `health`
## runs from 0 to 1; the bar's length is the reading, there is no number.
static func icon(hero: Dictionary, px: int, ring: Color, ring_width: int, health: float) -> Control:
	var node := portrait(hero, px, ring, ring_width)
	var bar := Panel.new()
	bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bar.position = Vector2(px * 0.12, px * 0.76)
	bar.size = Vector2(px * 0.76, px * 0.13)
	bar.add_theme_stylebox_override("panel", box(Color(INK, 0.9), 99, 0, PAPER_SHADE, 2))
	var amount := clampf(health, 0.0, 1.0)
	if amount > 0.0:
		var level := Panel.new()
		level.mouse_filter = Control.MOUSE_FILTER_IGNORE
		level.position = Vector2(4, 4)
		level.size = Vector2(maxf(bar.size.y - 8, (bar.size.x - 8) * amount), bar.size.y - 8)
		level.add_theme_stylebox_override("panel", box(VERMILION if amount < LOW_HEALTH else VERDIGRIS, 99, 0))
		bar.add_child(level)
	node.add_child(bar)
	# The armor shield stands above the health bar, not over it.
	var shield := ArmorBadge.of(node)
	if shield != null:
		shield.share = 0.4
		shield.place(px, Vector2(shield.anchor.x, 0.55))
	return node


## An image from `client/assets/`, or null when the file is not there yet,
## so every picture can be missing and the placeholder shows instead.
static func art(path: String) -> Texture2D:
	var full := "res://assets/" + path
	return load(full) as Texture2D if ResourceLoader.exists(full) else null


## Removes the face of a button, so a drawn Shape behind its content is its look.
static func bare(button: Button) -> void:
	for state in ["normal", "hover", "pressed", "disabled", "focus"]:
		button.add_theme_stylebox_override(state, StyleBoxEmpty.new())
	button.focus_mode = Control.FOCUS_NONE


## A round seal with a number or a short word in it.
static func badge(text: String, px: int, color: Color, ink: Color, size: int, ring := Color.TRANSPARENT) -> Control:
	var node := Control.new()
	node.custom_minimum_size = Vector2(px, px)
	node.size = Vector2(px, px)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.add_child(Shape.new("disc", color, ring, 0.0 if ring.a == 0.0 else 3.0))
	var words := centered(text, size, ink, false)
	fill(words)
	node.add_child(words)
	return node


## A round button: a word in a disc, with an optional coin on its rim.
static func medallion(text: String, coin: String, on_press: Callable, px: int, color := PAPER, size := 27) -> Button:
	var node := Button.new()
	node.custom_minimum_size = Vector2(px, px)
	node.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	bare(node)
	var ink := INK if color.get_luminance() > 0.45 else PAPER
	var disc := Shape.new("disc", color, GILT, 5.0)
	node.add_child(disc)
	var words := centered(text, size, ink)
	words.autowrap_mode = TextServer.AUTOWRAP_WORD
	fill(words, 10)
	node.add_child(words)
	if coin != "":
		var cost := badge(coin, 56, GILT, INK, 32, INK)
		cost.position = Vector2(px - 44, -12)
		node.add_child(cost)
	node.button_down.connect(func(): disc.modulate = Color(0.82, 0.82, 0.82))
	node.button_up.connect(func(): disc.modulate = Color.WHITE)
	node.pressed.connect(on_press)
	return node


## A button that is a painted picture. `caption` is a small plate over its
## lower edge (it covers anything painted there), `coin` a cost on its rim.
static func emblem(picture: Texture2D, caption: String, coin: String, on_press: Callable, size: Vector2, plate := PAPER_SHADE, coin_beside := false) -> Button:
	var node := Button.new()
	node.custom_minimum_size = size
	node.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	bare(node)
	var image := TextureRect.new()
	image.texture = picture
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.mouse_filter = Control.MOUSE_FILTER_IGNORE
	fill(image)
	node.add_child(image)
	if caption != "":
		var tag := PanelContainer.new()
		tag.mouse_filter = Control.MOUSE_FILTER_IGNORE
		tag.add_theme_stylebox_override("panel", box(plate, 18, 4, INK, 2))
		tag.get_theme_stylebox("panel").content_margin_left = 14
		tag.get_theme_stylebox("panel").content_margin_right = 14
		tag.add_child(centered(caption, 24, INK, false))
		tag.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
		tag.grow_horizontal = Control.GROW_DIRECTION_BOTH
		tag.grow_vertical = Control.GROW_DIRECTION_BEGIN
		tag.offset_bottom = -size.y * 0.1
		tag.offset_top = tag.offset_bottom
		node.add_child(tag)
	if coin != "":
		var cost := token("coin", coin, 60, 32)
		# Beside the picture when its top carries something that must stay visible.
		cost.position = Vector2(-66, size.y * 0.5 - 30) if coin_beside else Vector2(size.x - 60, -8)
		node.add_child(cost)
	node.button_down.connect(func(): image.modulate = Color(0.75, 0.75, 0.75))
	node.button_up.connect(func(): image.modulate = Color.WHITE)
	node.pressed.connect(on_press)
	return node


## The points of a silhouette inside `rect`: "oval", "disc", "tile" or "arch".
static func outline(kind: String, rect: Rect2) -> PackedVector2Array:
	var points := PackedVector2Array()
	var centre := rect.get_center()
	match kind:
		"oval":
			_arc(points, centre, rect.size.x / 2.0, rect.size.y / 2.0, 0.0, TAU, 56, false)
		"disc":
			var r := minf(rect.size.x, rect.size.y) / 2.0
			_arc(points, centre, r, r, 0.0, TAU, 48, false)
		"tile":
			var c := minf(rect.size.x, rect.size.y) * 0.18
			_arc(points, rect.position + Vector2(c, c), c, c, PI, PI * 1.5, 6, true)
			_arc(points, Vector2(rect.end.x - c, rect.position.y + c), c, c, PI * 1.5, TAU, 6, true)
			_arc(points, rect.end - Vector2(c, c), c, c, 0.0, PI / 2.0, 6, true)
			_arc(points, Vector2(rect.position.x + c, rect.end.y - c), c, c, PI / 2.0, PI, 6, true)
		_:
			var half := rect.size.x / 2.0
			var rise := minf(half, rect.size.y * 0.6)
			var corner := minf(rect.size.x, rect.size.y) * 0.1
			_arc(points, Vector2(centre.x, rect.position.y + rise), half, rise, PI, TAU, 28, true)
			_arc(points, rect.end - Vector2(corner, corner), corner, corner, 0.0, PI / 2.0, 6, true)
			_arc(points, Vector2(rect.position.x + corner, rect.end.y - corner), corner, corner, PI / 2.0, PI, 6, true)
	return points


static func _arc(points: PackedVector2Array, centre: Vector2, rx: float, ry: float, from: float, to: float, steps: int, closed: bool) -> void:
	var count := steps + 1 if closed else steps
	for i in count:
		var angle := lerpf(from, to, float(i) / steps)
		points.append(centre + Vector2(cos(angle) * rx, sin(angle) * ry))


## A drawn silhouette: filled, outlined or both. With `clip`, its children
## are cut to the silhouette (a hatch now, the illustration later).
class Shape extends Control:
	var kind := "oval"
	var color := Color.TRANSPARENT
	var border := Color.TRANSPARENT
	var width := 0.0
	## Painted across the silhouette when set; `color` then tints it.
	var texture: Texture2D

	func _init(shape_kind: String, fill_color: Color, border_color := Color.TRANSPARENT, border_width := 0.0, clip := false) -> void:
		kind = shape_kind
		color = fill_color
		border = border_color
		width = border_width
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		set_anchors_preset(Control.PRESET_FULL_RECT)
		if clip:
			clip_children = CanvasItem.CLIP_CHILDREN_AND_DRAW

	func _draw() -> void:
		if size.x < 4.0 or size.y < 4.0:
			return
		var inset := Vector2(width, width) / 2.0
		var points := UI.outline(kind, Rect2(inset, size - inset * 2.0))
		if texture != null:
			# Centre-crops the picture to the shape's own proportions.
			var scale := maxf(size.x / texture.get_width(), size.y / texture.get_height())
			var shown := size / (texture.get_size() * scale)
			var uvs := PackedVector2Array()
			for point in points:
				uvs.append((Vector2.ONE - shown) / 2.0 + point / size * shown)
			draw_colored_polygon(points, color, uvs, texture)
		elif color.a > 0.0:
			draw_colored_polygon(points, color)
		if width > 0.0:
			points.append(points[0])
			draw_polyline(points, border, width, true)


## An arrowhead pointing left (-1) or right (1), drawn so it needs no picture:
## turning the page.
class Arrow extends Control:
	var color := Color.WHITE
	var way := 1

	func _init(direction: int, tint: Color) -> void:
		way = direction
		color = tint
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func _draw() -> void:
		var centre := size / 2.0
		var half := minf(size.x, size.y * 0.6) / 2.0
		var tip := centre + Vector2(half * way, 0)
		var back := centre.x - half * way
		var notch := centre.x - half * 0.35 * way
		var points := PackedVector2Array([tip, Vector2(back, centre.y - half * 1.5), Vector2(notch, centre.y), Vector2(back, centre.y + half * 1.5)])
		draw_colored_polygon(points, color)
		points.append(points[0])
		draw_polyline(points, UI.OUTLINE_INK, 3.0, true)


## A cog wheel, drawn so it needs no picture: the sign for the game menu.
class Gear extends Control:
	var color := Color.WHITE
	var teeth := 8

	func _init(tint: Color) -> void:
		color = tint
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		set_anchors_preset(Control.PRESET_FULL_RECT)

	func _draw() -> void:
		var centre := size / 2.0
		var outer := minf(size.x, size.y) / 2.0
		var body := outer * 0.74
		var points := PackedVector2Array()
		# Each tooth: up its flank, across its top, down again, along the body.
		for tooth in teeth:
			var at := TAU * tooth / teeth
			var half := TAU / teeth / 2.0
			for corner in [[-0.46, body], [-0.3, outer], [0.3, outer], [0.46, body]]:
				points.append(centre + Vector2.from_angle(at + half * float(corner[0])) * float(corner[1]))
		draw_colored_polygon(points, color)
		draw_circle(centre, outer * 0.32, Color(0.07, 0.08, 0.14))


## The round button with a gear on it that opens the game menu.
static func gear_button(on_press: Callable, px: int) -> Button:
	var node := Button.new()
	node.custom_minimum_size = Vector2(px, px)
	bare(node)
	var disc := Shape.new("disc", Color(0.07, 0.08, 0.14, 0.92), Color("b9c6dc"), 5.0)
	node.add_child(disc)
	var gear := Gear.new(Color("e8eef8"))
	fill(gear, int(px * 0.2))
	node.add_child(gear)
	node.button_down.connect(func(): disc.modulate = Color(0.75, 0.75, 0.75))
	node.button_up.connect(func(): disc.modulate = Color.WHITE)
	node.pressed.connect(on_press)
	return node


## Diagonal engraved hatch: the sign for disabled, unavailable, removed or wrong.
class Hatch extends Control:
	var color := Color(0.08, 0.1, 0.18, 0.4)
	var step := 18.0

	func _init(tint := Color(0.08, 0.1, 0.18, 0.4)) -> void:
		color = tint
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		clip_contents = true
		set_anchors_preset(Control.PRESET_FULL_RECT)

	func _draw() -> void:
		var x := -size.y
		while x < size.x:
			draw_line(Vector2(x, size.y), Vector2(x + size.y, 0), color, 3.0)
			x += step


## A drawn mark, so right and wrong never depend on colour or on a font glyph.
class Mark extends Control:
	var kind := "check"
	var color := Color.WHITE
	var _painted := false

	func _init(mark_kind: String, tint: Color, px: int) -> void:
		kind = mark_kind
		color = tint
		custom_minimum_size = Vector2(px, px)
		size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		size_flags_vertical = Control.SIZE_SHRINK_CENTER
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		# The painted symbol (`skin/mark_<kind>.png`) when there is one; the
		# drawn strokes below are the fallback, and the only form of the
		# small "ring" and "dot" marks.
		var painted := UI.art("skin/mark_%s.png" % kind)
		if painted != null:
			_painted = true
			var image := TextureRect.new()
			image.texture = painted
			image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			image.mouse_filter = Control.MOUSE_FILTER_IGNORE
			image.set_anchors_preset(Control.PRESET_FULL_RECT)
			add_child(image)

	func _draw() -> void:
		if _painted:
			return
		var s := minf(size.x, size.y)
		var o := (size - Vector2(s, s)) / 2.0
		var w := maxf(4.0, s * 0.14)
		match kind:
			"check":
				draw_polyline([o + Vector2(0.15, 0.55) * s, o + Vector2(0.4, 0.8) * s, o + Vector2(0.86, 0.2) * s], color, w)
			"cross":
				draw_line(o + Vector2(0.2, 0.2) * s, o + Vector2(0.8, 0.8) * s, color, w)
				draw_line(o + Vector2(0.8, 0.2) * s, o + Vector2(0.2, 0.8) * s, color, w)
			"lock":
				draw_rect(Rect2(o + Vector2(0.22, 0.48) * s, Vector2(0.56, 0.42) * s), color)
				draw_arc(o + Vector2(0.5, 0.48) * s, 0.2 * s, PI, TAU, 16, color, w * 0.7)
			"ring":
				draw_arc(o + Vector2(0.5, 0.5) * s, 0.3 * s, 0.0, TAU, 24, color, w * 0.6)
			_:
				draw_circle(o + Vector2(0.5, 0.5) * s, s * 0.1, color)


## A region that accepts a dragged card. The callbacks decide what a drop means.
class DropZone extends PanelContainer:
	var accepts: Callable
	var dropped: Callable

	func _can_drop_data(_at: Vector2, data: Variant) -> bool:
		return accepts.is_valid() and bool(accepts.call(data))

	func _drop_data(_at: Vector2, data: Variant) -> void:
		dropped.call(data)


## A labelled slider from 0 to 100% that sets a volume as it moves.
static func volume(title: String, value: float, set_to: Callable) -> Control:
	var line := row(24)
	var name_label := label(title, 40, PAPER, false)
	name_label.custom_minimum_size.x = 200
	name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	line.add_child(name_label)
	var slider := HSlider.new()
	slider.min_value = 0.0
	slider.max_value = 1.0
	slider.step = 0.05
	slider.value = value
	slider.custom_minimum_size = Vector2(0, 80)
	slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	slider.add_theme_icon_override("grabber", knob(GILT))
	slider.add_theme_icon_override("grabber_highlight", knob(GILT.lightened(0.25)))
	var groove := box(Color(0.06, 0.07, 0.12), 10, 0, Color("b9c6dc"), 2)
	groove.content_margin_top = 10
	groove.content_margin_bottom = 10
	slider.add_theme_stylebox_override("slider", groove)
	var filled := box(VERDIGRIS, 10, 0)
	filled.content_margin_top = 10
	filled.content_margin_bottom = 10
	slider.add_theme_stylebox_override("grabber_area", filled)
	slider.add_theme_stylebox_override("grabber_area_highlight", filled)
	var percent := label("%d%%" % roundi(value * 100), 34, SLATE, false)
	percent.custom_minimum_size.x = 110
	percent.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	percent.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	slider.value_changed.connect(func(new_value: float) -> void:
		set_to.call(new_value)
		percent.text = "%d%%" % roundi(new_value * 100))
	# A click when the finger lets go, so the new effects level can be heard.
	slider.drag_ended.connect(func(_changed: bool) -> void: Sound.instance.play("button"))
	line.add_child(slider)
	line.add_child(percent)
	return line


## A round slider knob in `colour`.
static func knob(colour: Color) -> Texture2D:
	var size := 44
	var image := Image.create(size, size, false, Image.FORMAT_RGBA8)
	var centre := Vector2(size, size) / 2.0
	for y in size:
		for x in size:
			var distance := Vector2(x + 0.5, y + 0.5).distance_to(centre)
			var alpha := clampf(size / 2.0 - distance, 0.0, 1.0)
			var shade := colour.darkened(0.35) if distance > size / 2.0 - 4.0 else colour
			image.set_pixel(x, y, Color(shade, alpha))
	return ImageTexture.create_from_image(image)
