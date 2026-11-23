## A small shield on the lower corner of a hero portrait with the player's
## armor on it. It shows only while the armor is above 0, and follows the
## host's view on its own: `UI.portrait` adds one to every portrait large
## enough to read, and the badge finds the seat by the hero, since a hero
## sits at most once in a lobby. Holds no rules: the number is the view's
## `armor` (board shields and temporary armor together).
class_name ArmorBadge
extends Control

## The badge's height as a share of the portrait's height.
@export var share := 0.34
## Never drawn smaller than this, in pixels.
@export var min_size := 40.0
## Where the badge's centre sits in the portrait's square, as shares of its side.
@export var anchor := Vector2(0.17, 0.8)
## The number's size as a share of the badge's height.
@export var number_share := 0.42
## How much larger the badge grows for a moment when its armor goes down.
@export var pulse_scale := 1.3
@export var pulse_time := 0.35
## Tint of the flash when armor is lost.
@export var pulse_tint := Color(1.6, 1.5, 1.2)
## How long the badge takes to pop in when armor first appears.
@export var appear_time := 0.22

## Portraits smaller than this get no badge: the number would not be read.
const MIN_PORTRAIT := 90

const SHIELD := "skin/armor.png"
## The shield outline used when the painted shield is missing, on a 1x1 box.
const OUTLINE := [Vector2(0.06, 0.08), Vector2(0.5, 0.0), Vector2(0.94, 0.08), Vector2(0.9, 0.55),
	Vector2(0.5, 1.0), Vector2(0.1, 0.55)]

## The hero whose player is followed; empty when `follow_store` is off.
var hero_id := ""
## Off in previews and anywhere the value is set by hand with `show_value`.
var follow_store := true

var _value := 0
var _words: Label
var _picture: Texture2D
var _tween: Tween


## Puts a badge on `portrait`, a `px` square, for the player who has `hero`.
## Returns it, or null when the portrait is too small or has no hero.
static func attach(portrait: Control, hero: Dictionary, px: int) -> ArmorBadge:
	if px < MIN_PORTRAIT or hero.is_empty():
		return null
	var badge := ArmorBadge.new()
	badge.name = "ArmorBadge"
	badge.hero_id = String(hero.get("id", ""))
	badge.place(px)
	portrait.add_child(badge)
	return badge


## The badge on a portrait made by `UI.portrait`, or null.
static func of(portrait: Control) -> ArmorBadge:
	return portrait.get_node_or_null("ArmorBadge") as ArmorBadge


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_picture = UI.art(SHIELD)


## Sizes the badge for a `px` portrait, centred on `anchor`.
func place(px: int, at := Vector2(-1, -1)) -> void:
	var spot := anchor if at.x < 0.0 else at
	var height := maxf(min_size, px * share)
	var width := height * (170.0 / 192.0 if _picture == null else float(_picture.get_width()) / _picture.get_height())
	size = Vector2(width, height)
	position = spot * px - size / 2.0
	pivot_offset = size / 2.0
	if _words != null:
		_style_words()


func _ready() -> void:
	_words = Label.new()
	_words.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_words.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_words.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_words.set_anchors_preset(Control.PRESET_FULL_RECT)
	# The shield's boss sits a little above the middle.
	_words.offset_bottom = -size.y * 0.08
	_words.add_theme_font_override("font", UI.display_font())
	_words.add_theme_color_override("font_color", UI.INK)
	_words.add_theme_color_override("font_outline_color", Color(1, 1, 1, 0.6))
	add_child(_words)
	_style_words()
	if follow_store:
		Store.changed.connect(_follow)
		_follow()


func _style_words() -> void:
	_words.add_theme_font_size_override("font_size", maxi(14, int(size.y * number_share)))
	_words.add_theme_constant_override("outline_size", maxi(2, int(size.y * 0.06)))


## Shows `armor`. A drop pulses the badge; going to 0 hides it.
func show_value(armor: int) -> void:
	var was := _value
	_value = maxi(0, armor)
	if _words != null:
		_words.text = str(_value)
	if _value == was and visible == (_value > 0):
		return
	if _tween != null and _tween.is_valid():
		_tween.kill()
	scale = Vector2.ONE
	modulate = Color.WHITE
	if _value <= 0:
		visible = false
		return
	var popping := not visible
	visible = true
	if not is_inside_tree():
		return
	_tween = create_tween()
	if popping or _value > was:
		scale = Vector2(0.4, 0.4) if popping else Vector2(1.15, 1.15)
		_tween.tween_property(self, "scale", Vector2.ONE, appear_time).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	else:
		scale = Vector2(pulse_scale, pulse_scale)
		modulate = pulse_tint
		_tween.set_parallel()
		_tween.tween_property(self, "scale", Vector2.ONE, pulse_time).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
		_tween.tween_property(self, "modulate", Color.WHITE, pulse_time)


## Reads the armor of the player who has this hero from the view.
func _follow() -> void:
	show_value(_armor_in_view())


func _armor_in_view() -> int:
	if hero_id == "" or not Store.has_match():
		return 0
	for player in Store.view.get("players", []):
		if player.get("hero") == null:
			continue
		if String(Store.hero(player["hero"]).get("id", "")) == hero_id:
			return int(player.get("armor", 0)) if player.get("alive", true) else 0
	return 0


func _draw() -> void:
	if _picture != null:
		draw_texture_rect(_picture, Rect2(Vector2.ZERO, size), false)
		return
	var points := PackedVector2Array()
	for point in OUTLINE:
		points.append(point * size)
	draw_colored_polygon(points, Color("c8d0dc"))
	points.append(points[0])
	draw_polyline(points, UI.GILT, maxf(2.0, size.y * 0.06), true)
