# Headless check that the Rust extension loads and a whole offline match runs.
# Run: godot --headless --path client --script res://tests/smoke.gd
extends SceneTree

func _init() -> void:
	var lobby = ClassDB.instantiate("LocalLobby")
	if lobby == null:
		push_error("LocalLobby is not registered: the extension did not load")
		quit(1)
		return
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data")
	var error: String = lobby.load_content(data_dir)
	if error != "":
		push_error(error)
		quit(1)
		return
	var content = JSON.parse_string(lobby.content_json())
	print("cards: ", content["cards"].size(), ", heroes: ", content["heroes"].size())

	lobby.start(7, 0)
	var rejected: String = lobby.send(0, JSON.stringify({"Reroll": {"player": 3}}))
	print("acting for another seat -> ", rejected)
	# A player may pick only from the two heroes they were offered.
	var offer: Array = JSON.parse_string(lobby.view())["me"]["hero_offer"]
	print("hero offer: ", offer)
	print("pick hero -> '", lobby.send(0, JSON.stringify({"PickHero": {"player": 0, "hero": int(offer[0])}})), "'")

	var now := 0
	var events := 0
	var first_question := ""
	while not lobby.is_finished() and now < 3 * 60 * 60 * 1000:
		now += 250
		lobby.tick(now)
		for item in JSON.parse_string(lobby.drain()):
			events += 1
			if first_question == "" and item.has("question"):
				first_question = JSON.stringify(item["question"])
	var view = JSON.parse_string(lobby.view())
	print("first question: ", first_question)
	print("finished: ", lobby.is_finished(), " after ", now / 60000.0, " game minutes, ", events, " events")
	print("my placement: ", view["me"]["placement"], ", phase: ", view["phase"])
	quit(0 if lobby.is_finished() else 1)
