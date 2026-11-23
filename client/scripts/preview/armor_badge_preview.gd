## Preview of the armor shield on a hero portrait: the armor goes 0, 3, 12
## and back to 0, over and over, on a large portrait, a shop-sized one and a
## player-list icon. Run this scene on its own to tune `ArmorBadge`.
extends Control

## Seconds each value stays up.
@export var hold := 1.0
@export var values: Array[int] = [0, 3, 12, 0]

var _badges: Array = []


func _ready() -> void:
	var back := ColorRect.new()
	back.color = Color("141a33")
	back.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(back)
	var heroes: Array = Store.heroes
	var hero: Dictionary = heroes[2] if heroes.size() > 2 else {"id": "preview", "kind": "Preview", "category": 0, "name": "Preview"}
	var spots := [[400, Vector2(300, 340), "portrait"], [220, Vector2(900, 430), "portrait"], [150, Vector2(1400, 465), "icon"]]
	for spot in spots:
		var px: int = spot[0]
		var picture := UI.icon(hero, px, UI.GILT, 4, 0.6) if spot[2] == "icon" else UI.portrait(hero, px, UI.GILT, 6, 16)
		picture.position = spot[1]
		var badge := ArmorBadge.of(picture)
		if badge != null:
			badge.follow_store = false
			_badges.append(badge)
		add_child(picture)
	_loop()


func _loop() -> void:
	var index := 0
	while is_inside_tree():
		for badge in _badges:
			(badge as ArmorBadge).show_value(values[index % values.size()])
		index += 1
		await get_tree().create_timer(hold).timeout
