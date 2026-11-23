## Damage drawn over a screen: a few coloured motes curve from the attacker's
## portrait onto the defender's, and the defender shows blood spots, a flash,
## a shake and the number. A reflection waits until the hit before it has
## landed, then a few silver rays shoot back and the attacker bleeds in turn.
##
## It covers its parent and takes no touches. Portraits are passed as
## callables that return the Control to aim at, read every frame, so a screen
## may rebuild its portraits while an effect is flying. Holds no rules and
## nothing waits on it: it shows damage the host has already dealt.
class_name DamageFx
extends Control

## How many motes fly on a hit.
@export_range(1, 8) var motes_min := 3
@export_range(1, 8) var motes_max := 4
## Seconds a mote takes to reach the defender.
@export var travel_time := 0.6
## The longest a mote waits before it leaves, in seconds.
@export var launch_spread := 0.14
## How far a mote's path bends sideways, as a share of the distance flown.
@export var bend := 0.55
## Radius of a mote's head, and how long its trail is, as a share of the path.
@export var mote_radius := 9.0
@export var trail_share := 0.22
@export var mote_colours: Array[Color] = [Color("e0a526"), Color("2fa58c"), Color("e4572e"), Color("9fc4f0")]

## How many rays a reflection shoots back, at most and at least.
@export_range(1, 6) var rays_min := 2
@export_range(1, 6) var rays_max := 3
@export var ray_time := 0.32
@export var ray_width := 5.0
## The length of the bright part of a ray, as a share of the distance.
@export var ray_share := 0.45
## How far apart the rays leave, in pixels.
@export var ray_spread := 40.0
@export var ray_core := Color("f4f8ff")
@export var ray_glow := Color("a9c8ee")

## Blood spots on the hit portrait: how many, how large as a share of the
## portrait's height, and how long they stay before they have faded.
@export_range(1, 10) var splats_min := 3
@export_range(1, 10) var splats_max := 5
@export var splat_share := Vector2(0.035, 0.065)
@export var splat_life := 1.4
@export var splat_colours: Array[Color] = [Color("7d1016"), Color("9a1a1c"), Color("5e0b10")]
## How far from the portrait's centre spots fall, as a share of its height.
@export var splat_reach := 0.36

## The defender's flash and shake.
@export var flash_tint := Color(1.5, 0.82, 0.78)
@export var flash_time := 0.3
@export var shake_pixels := 10.0
@export var shake_time := 0.3

## The number that rises from the hit portrait.
@export var number_size := 96
@export var number_rise := 90.0
@export var number_life := 1.2
@export var number_colour := Color("f2e9d8")
@export var blocked_colour := Color("c8d0dc")

## A pause between two hits that follow each other, in seconds.
@export var hit_gap := 0.12

## Effect time runs this many times faster than real time; 1 or more.
var speed := 1.0

var _clock := 0.0
## When the latest hit lands; the next hit or reflection starts from there.
var _last_impact := 0.0
## Things in flight: motes, rays, splats, numbers and sparks, each a Dictionary.
var _items: Array = []
## Impacts still to come: [time, Callable].
var _due: Array = []
var _font: Font


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


func _ready() -> void:
	_font = UI.display_font()
	if _font == null:
		_font = get_theme_default_font()


## True while something is still drawn or due.
func busy() -> bool:
	return not _items.is_empty() or not _due.is_empty()


## A hit from `attacker` onto `defender`, both callables that return a
## Control. `amount` is the damage, `hp_lost` what got past armor.
## Returns the seconds of effect time until it lands.
func strike(attacker: Callable, defender: Callable, amount: int, hp_lost: int) -> float:
	var start := maxf(_clock, _last_impact + (hit_gap if _last_impact > _clock else 0.0))
	var count := randi_range(motes_min, maxi(motes_min, motes_max))
	var landed := 0.0
	var colours := mote_colours.duplicate()
	colours.shuffle()
	for index in count:
		var delay := randf() * launch_spread
		var flight := travel_time * randf_range(0.88, 1.12)
		# Alternate sides, each mote bending its own amount, so the paths fan out.
		var side := -1.0 if index % 2 == 0 else 1.0
		var reach := (0.45 + 0.55 * float(index / 2 + 1) / ceilf(count / 2.0)) * randf_range(0.85, 1.1)
		_items.append({
			"kind": "mote", "from": attacker, "to": defender,
			"t0": start + delay, "life": flight,
			"colour": colours[index % colours.size()] if not colours.is_empty() else Color.WHITE,
			"bend_a": side * reach * bend,
			"bend_b": side * reach * randf_range(0.1, 0.6) * bend,
		})
		landed = maxf(landed, start + delay + flight)
		# Each mote is a burst of fire as it leaves.
		Sound.instance.cue("hit", start + delay - _clock)
	_landing(landed, defender, amount, hp_lost)
	return landed - _clock


## A reflection: once the hit before it has landed, rays shoot from
## `reflector` back at `attacker`, who then takes `amount`.
func reflect(reflector: Callable, attacker: Callable, amount: int, hp_lost: int) -> float:
	var start := maxf(_clock, _last_impact)
	var count := randi_range(rays_min, maxi(rays_min, rays_max))
	for index in count:
		var lane := (index - (count - 1) / 2.0) * ray_spread
		_items.append({
			"kind": "ray", "from": reflector, "to": attacker,
			"t0": start + index * 0.05, "life": ray_time, "lane": lane,
		})
		Sound.instance.cue("hit", start + index * 0.05 - _clock)
	var landed := start + (count - 1) * 0.05 + ray_time
	_landing(landed, attacker, amount, hp_lost)
	return landed - _clock


func _landing(at: float, target: Callable, amount: int, hp_lost: int) -> void:
	_last_impact = at
	_due.append([at, _impact.bind(target, amount, hp_lost)])


func _impact(target: Callable, amount: int, hp_lost: int) -> void:
	var portrait := _control(target)
	if portrait == null:
		return
	var height := maxf(60.0, portrait.size.y * portrait.get_global_transform().get_scale().y)
	if hp_lost > 0:
		var count := randi_range(splats_min, maxi(splats_min, splats_max))
		var turn := randf() * TAU
		for index in count:
			_items.append(_splat(target, height, turn + TAU * index / count))
	_items.append({"kind": "spark", "at": target, "t0": _clock, "life": 0.35, "reach": height * 0.42,
		"colour": Color("f4e2b8") if hp_lost > 0 else blocked_colour})
	var text := "-%d" % hp_lost if hp_lost > 0 else ("Blocked" if amount > 0 else "0")
	_items.append({"kind": "number", "at": target, "t0": _clock, "life": number_life, "text": text,
		"colour": number_colour if hp_lost > 0 else blocked_colour, "lean": randf_range(-0.2, 0.2) * height})
	_jolt(portrait)


## A blood spot: an uneven blob and a few drops beside it.
func _splat(target: Callable, height: float, heading: float) -> Dictionary:
	var radius := randf_range(splat_share.x, splat_share.y) * height
	var spot := Vector2.from_angle(heading + randf_range(-0.4, 0.4)) * randf_range(0.3, 1.0) * splat_reach * height
	var blob := PackedVector2Array()
	var steps := 14
	for step in steps:
		var angle := TAU * step / steps
		blob.append(Vector2.from_angle(angle) * radius * randf_range(0.72, 1.12))
	var drops := []
	for drop in randi_range(2, 4):
		var away := Vector2.from_angle(randf() * TAU) * radius * randf_range(1.25, 1.9)
		drops.append([away, radius * randf_range(0.12, 0.26)])
	return {"kind": "splat", "at": target, "t0": _clock + randf() * 0.08, "life": splat_life, "offset": spot,
		"blob": blob, "drops": drops, "colour": splat_colours[randi() % splat_colours.size()] if not splat_colours.is_empty() else Color("7d1016")}


## The flash and shake on the portrait that was hit.
func _jolt(portrait: Control) -> void:
	var pace := maxf(1.0, speed)
	var rest := portrait.position
	var tween := portrait.create_tween()
	portrait.modulate = flash_tint
	tween.tween_property(portrait, "modulate", Color.WHITE, flash_time / pace)
	var shake := portrait.create_tween()
	var steps := 5
	for step in steps:
		var strength := shake_pixels * (1.0 - float(step) / steps)
		shake.tween_property(portrait, "position", rest + Vector2(strength * (-1.0 if step % 2 else 1.0), randf_range(-0.4, 0.4) * strength),
			shake_time / steps / pace)
	shake.tween_property(portrait, "position", rest, shake_time / steps / pace)


func _process(delta: float) -> void:
	if _items.is_empty() and _due.is_empty():
		return
	_clock += delta * maxf(1.0, speed)
	while not _due.is_empty():
		var next := -1
		for index in _due.size():
			if _clock >= float(_due[index][0]) and (next < 0 or float(_due[index][0]) < float(_due[next][0])):
				next = index
		if next < 0:
			break
		var call: Callable = _due[next][1]
		_due.remove_at(next)
		call.call()
	_items = _items.filter(func(item: Dictionary) -> bool: return _clock < float(item["t0"]) + float(item["life"]))
	queue_redraw()


func _control(target: Callable) -> Control:
	if not target.is_valid():
		return null
	var node = target.call()
	return node as Control if is_instance_valid(node) and node is Control and (node as Control).is_visible_in_tree() else null


## The centre of a portrait in this layer's coordinates, or null.
func _centre(target: Callable) -> Variant:
	var portrait := _control(target)
	if portrait == null:
		return null
	var global := portrait.get_global_rect().get_center()
	return get_global_transform().affine_inverse() * global


func _draw() -> void:
	for item in _items:
		var age := (_clock - float(item["t0"])) / float(item["life"])
		if age < 0.0:
			continue
		match String(item["kind"]):
			"splat":
				_draw_splat(item, age)
			"mote":
				_draw_mote(item, age)
			"ray":
				_draw_ray(item, age)
			"spark":
				_draw_spark(item, age)
	# Numbers above everything else.
	for item in _items:
		var age := (_clock - float(item["t0"])) / float(item["life"])
		if age >= 0.0 and String(item["kind"]) == "number":
			_draw_number(item, age)


func _path(item: Dictionary) -> Array:
	var from = _centre(item["from"])
	var to = _centre(item["to"])
	if from == null or to == null:
		return []
	var line: Vector2 = to - from
	var normal := Vector2(-line.y, line.x)
	var b1: Vector2 = from + line * 0.3 + normal * float(item["bend_a"])
	var b2: Vector2 = from + line * 0.72 + normal * float(item["bend_b"])
	return [from, b1, b2, to]


static func _bezier(points: Array, t: float) -> Vector2:
	var u := 1.0 - t
	return points[0] * u * u * u + points[1] * 3.0 * u * u * t + points[2] * 3.0 * u * t * t + points[3] * t * t * t


func _draw_mote(item: Dictionary, age: float) -> void:
	var points := _path(item)
	if points.is_empty():
		return
	var head := ease(age, -1.8)
	var colour: Color = item["colour"]
	var tail := maxf(0.0, head - trail_share)
	var steps := 14
	var previous := _bezier(points, tail)
	for step in range(1, steps + 1):
		var t := lerpf(tail, head, float(step) / steps)
		var point := _bezier(points, t)
		var share := float(step) / steps
		draw_line(previous, point, Color(colour, 0.15 * share), mote_radius * 2.2 * share, true)
		draw_line(previous, point, Color(colour.lightened(0.3), 0.75 * share), mote_radius * 0.9 * share + 1.0, true)
		previous = point
	var at := _bezier(points, head)
	draw_circle(at, mote_radius * 2.6, Color(colour, 0.14))
	draw_circle(at, mote_radius * 1.6, Color(colour, 0.35))
	draw_circle(at, mote_radius, colour.lightened(0.25))
	draw_circle(at, mote_radius * 0.45, Color(1, 1, 0.95))


func _draw_ray(item: Dictionary, age: float) -> void:
	var from = _centre(item["from"])
	var to = _centre(item["to"])
	if from == null or to == null:
		return
	var line: Vector2 = to - from
	var across := Vector2(-line.y, line.x).normalized() * float(item["lane"])
	# The lanes meet at the target, so the rays converge on it.
	var start: Vector2 = from + across
	var head_t := clampf(age * (1.0 + ray_share), 0.0, 1.0)
	var tail_t := clampf(age * (1.0 + ray_share) - ray_share, 0.0, 1.0)
	var head := start.lerp(to, head_t)
	var tail := start.lerp(to, tail_t)
	var fade := 1.0 - clampf((age - 0.8) / 0.2, 0.0, 1.0)
	draw_line(tail, head, Color(ray_glow, 0.22 * fade), ray_width * 4.0, true)
	draw_line(tail, head, Color(ray_glow, 0.6 * fade), ray_width * 1.8, true)
	draw_line(tail, head, Color(ray_core, fade), ray_width * 0.7, true)
	draw_circle(head, ray_width * 1.6, Color(ray_core, 0.9 * fade))


func _draw_splat(item: Dictionary, age: float) -> void:
	var centre = _centre(item["at"])
	if centre == null:
		return
	# Splashes in quickly, holds, then fades.
	var grow := clampf(age / 0.08, 0.0, 1.0)
	var size := lerpf(0.55, 1.0, ease(grow, 0.4))
	var alpha := 0.9 * (1.0 - clampf((age - 0.55) / 0.45, 0.0, 1.0))
	var colour: Color = item["colour"]
	var spot: Vector2 = centre + item["offset"]
	var blob := PackedVector2Array()
	for point in item["blob"]:
		blob.append(spot + point * size)
	draw_colored_polygon(blob, Color(colour, alpha))
	draw_circle(spot - (item["blob"][3] as Vector2) * 0.25 * size, (item["blob"][0] as Vector2).length() * 0.3 * size,
		Color(colour.lightened(0.25), alpha * 0.5))
	for drop in item["drops"]:
		draw_circle(spot + (drop[0] as Vector2) * size, float(drop[1]) * size, Color(colour, alpha))


func _draw_spark(item: Dictionary, age: float) -> void:
	var centre = _centre(item["at"])
	if centre == null:
		return
	var colour: Color = item["colour"]
	var radius := float(item["reach"]) * ease(age, 0.4)
	draw_arc(centre, radius, 0.0, TAU, 40, Color(colour, 0.7 * (1.0 - age)), 6.0 * (1.0 - age) + 1.0, true)


func _draw_number(item: Dictionary, age: float) -> void:
	var centre = _centre(item["at"])
	if centre == null or _font == null:
		return
	var text := String(item["text"])
	var size := number_size if text.begins_with("-") else int(number_size * 0.6)
	var pop := 1.0 + 0.35 * (1.0 - clampf(age / 0.15, 0.0, 1.0))
	size = int(size * pop)
	var width := _font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
	var at: Vector2 = centre + Vector2(float(item["lean"]) - width / 2.0, -number_rise * ease(age, 0.5) + size * 0.35)
	var alpha := 1.0 - clampf((age - 0.7) / 0.3, 0.0, 1.0)
	draw_string_outline(_font, at, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, maxi(4, size / 8), Color(UI.OUTLINE_INK, alpha))
	draw_string(_font, at, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, Color(item["colour"], alpha))
