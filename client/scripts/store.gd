## The latest state the host has sent, plus the static content (cards,
## heroes, categories) used to display it. Holds no rules: screens read from
## here and send commands through Transport.
extends Node

signal changed

## Everything the local player may see right now. Empty before a match starts.
var view: Dictionary = {}
var cards: Array = []
var heroes: Array = []
var categories: Array = []
var tunables: Dictionary = {}

## Names shown for the other seats until real players sit there.
const BOT_NAMES: Array[String] = ["Minerva", "Archie", "Tawny", "Sage", "Strix", "Quill"]
const PROFILE_FILE := "user://profile.cfg"
const NAME_LENGTH := 14
## The local player's display name, set on the Profile sheet.
var my_name := "You"
## Names by seat in an online match, as the server gave them.
var seat_names: Array = []


func _ready() -> void:
	var file := ConfigFile.new()
	if file.load(PROFILE_FILE) == OK:
		my_name = String(file.get_value("profile", "name", my_name))


func set_my_name(text: String) -> void:
	var cleaned := text.strip_edges().left(NAME_LENGTH)
	my_name = "You" if cleaned == "" else cleaned
	var file := ConfigFile.new()
	file.set_value("profile", "name", my_name)
	file.save(PROFILE_FILE)


## The name of the player in a seat: the local player's own, or a bot's.
func player_name(seat: int) -> String:
	if Transport.online and seat >= 0 and seat < seat_names.size():
		return String(seat_names[seat]).trim_suffix(" (bot)")
	if has_match() and seat == my_id():
		return my_name
	return BOT_NAMES[seat % BOT_NAMES.size()]


func set_content(content: Dictionary) -> void:
	cards = content["cards"]
	heroes = content["heroes"]
	tunables = content["tunables"]
	categories = tunables["categories"]


func set_view(new_view: Variant) -> void:
	if new_view == null or new_view == view:
		return
	view = new_view
	changed.emit()


func has_match() -> bool:
	return not view.is_empty()


func me() -> Dictionary:
	return view["me"]


func my_id() -> int:
	return int(view["you"])


func card(def: Variant) -> Dictionary:
	return cards[int(def)]


func hero(id: Variant) -> Dictionary:
	return heroes[int(id)]


func category_name(id: Variant) -> String:
	return String(categories[int(id)]).replace("_", " ").capitalize()


## The hero of a player by seat, or an empty dictionary before they have one.
func hero_of(seat: int) -> Dictionary:
	var id = view["players"][seat]["hero"]
	return {} if id == null else hero(id)


func hero_name(seat: int) -> String:
	var found := hero_of(seat)
	return "Player %d" % (seat + 1) if found.is_empty() else String(found["name"])


## A short display name for a card instance: its parts joined, golden marked.
func instance_name(instance: Dictionary) -> String:
	var names: Array[String] = []
	for part in instance["parts"]:
		var text := String(card(part["def"])["name"])
		names.append("★ " + text if part["golden"] else text)
	return " + ".join(names)


func instance_text(instance: Dictionary) -> String:
	var lines: Array[String] = []
	for part in instance["parts"]:
		var def := card(part["def"])
		var text := String(def["text"])
		lines.append("%s%s: %s" % [def["name"], " (golden)" if part["golden"] else "", text])
	if int(instance["armor_left"]) > 0:
		lines.append("Armor left: %d" % int(instance["armor_left"]))
	return "\n".join(lines)
