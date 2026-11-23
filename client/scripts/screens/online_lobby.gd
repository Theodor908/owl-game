## Friends and the lobby. On the left the player's friends, the requests
## waiting for them and a field to add a friend by exact username; on the
## right the lobby: open one or join one by its code, invite friends who are
## online, seat bots, start the match.
## Holds no rules: the server decides who may join and when a match starts.
extends Control

signal back

## How often the friends list is fetched again, in seconds.
@export var refresh_seconds := 4.0

var _message := ""
var _add: LineEdit
var _code: LineEdit
var _since_refresh := 0.0


func _ready() -> void:
	Online.changed.connect(_refresh)
	Online.failed.connect(_fail)
	Online.refresh_friends()
	_refresh()


func _exit_tree() -> void:
	Online.changed.disconnect(_refresh)
	Online.failed.disconnect(_fail)


func _process(delta: float) -> void:
	_since_refresh += delta
	# Not while a name is being typed: a rebuild would drop the keyboard.
	var typing := (is_instance_valid(_add) and _add.has_focus()) or (is_instance_valid(_code) and _code.has_focus())
	if _since_refresh >= refresh_seconds and not typing:
		_since_refresh = 0.0
		Online.refresh_friends()


func _fail(text: String) -> void:
	_message = text
	_refresh()


func _refresh() -> void:
	var typed := _add.text if is_instance_valid(_add) else ""
	UI.clear(self)
	var top := UI.row(24)
	top.set_anchors_preset(Control.PRESET_TOP_WIDE)
	top.custom_minimum_size.y = 96
	top.add_child(UI.stamp("Back", func() -> void: back.emit(), Vector2(200, 92), 36, UI.INK))
	var title := UI.label("Play with friends", UI.GLANCE, UI.PAPER, false)
	title.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(title)
	# The player's own name, to tell a friend who wants to add them.
	var me := UI.label("You are %s%s" % [Online.username, "  (guest)" if Online.guest else ""], 34, UI.GILT, false)
	me.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	top.add_child(me)
	if _message != "":
		var note := UI.label(_message, 28, UI.VERMILION, false)
		note.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		top.add_child(note)
	add_child(top)

	var body := UI.row(24)
	body.set_anchors_preset(Control.PRESET_FULL_RECT)
	body.offset_top = 112
	add_child(body)
	body.add_child(_friends_panel(typed))
	body.add_child(_lobby_panel())


func _friends_panel(typed: String) -> Control:
	var panel := UI.panel(UI.INK_RAISED, 16, 24)
	panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var stack := UI.column(10)
	panel.add_child(stack)
	stack.add_child(UI.label("Friends", 44, UI.PAPER, false))

	var add_line := UI.row(12)
	_add = UI.field("A friend's exact username", typed)
	_add.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_line.add_child(_add)
	add_line.add_child(UI.stamp("Add", func() -> void:
		_message = ""
		if _add.text.strip_edges() != "":
			Online.add_friend(_add.text.strip_edges())
			_add.text = "", Vector2(180, 84), 32, UI.INK))
	stack.add_child(add_line)

	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var list := UI.column(8)
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list)
	stack.add_child(scroll)

	for friend in Online.friends.get("incoming", []):
		list.add_child(_row("%s wants to be friends" % String(friend), UI.GILT,
			[["Accept", Online.accept_friend.bind(String(friend))], ["Decline", Online.remove_friend.bind(String(friend))]]))
	var in_lobby: Array = Online.lobby.get("members", [])
	var asked: Array = Online.lobby.get("invited", [])
	for friend in Online.friends.get("friends", []):
		var name_text := String(friend["username"])
		var online: bool = friend["online"]
		var buttons: Array = []
		var state := "online" if online else "offline"
		if in_lobby.has(name_text):
			state = "in your lobby"
		elif asked.has(name_text):
			state = "invited"
		elif online and Online.is_owner():
			buttons.append(["Invite", Online.invite.bind(name_text)])
		buttons.append(["Remove", Online.remove_friend.bind(name_text)])
		list.add_child(_row("%s  ·  %s" % [name_text, state], UI.PAPER if online else UI.SLATE, buttons))
	for friend in Online.friends.get("outgoing", []):
		list.add_child(_row("%s  ·  asked" % String(friend), UI.SLATE, [["Withdraw", Online.remove_friend.bind(String(friend))]]))
	if list.get_child_count() == 0:
		list.add_child(UI.label("No friends yet. Add one by their username; they have to accept.", 28, UI.SLATE))
	return panel


## One line of the list: words, then small buttons ([[label, callable], ...]).
func _row(words: String, color: Color, buttons: Array) -> Control:
	var line := UI.row(10)
	var label := UI.label(words, 32, color, false)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.clip_text = true
	line.add_child(label)
	for button in buttons:
		var action: Callable = button[1]
		line.add_child(UI.stamp(String(button[0]), func() -> void:
			_message = ""
			action.call(), Vector2(170, 76), 28, UI.INK))
	return line


func _lobby_panel() -> Control:
	var typed := _code.text if is_instance_valid(_code) else ""
	var panel := UI.panel(UI.INK_RAISED, 16, 24)
	panel.custom_minimum_size.x = 720
	var stack := UI.column(10)
	panel.add_child(stack)
	stack.add_child(UI.label("Lobby", 44, UI.PAPER, false))
	if not Online.connected:
		stack.add_child(UI.label("Connecting to the server…", 30, UI.SLATE))
		return panel
	if Online.lobby.is_empty():
		stack.add_child(UI.label("Open a lobby and invite friends, or join one with the code its owner tells you.", 30, Color("e6eaf2")))
		stack.add_child(UI.stamp("Open a lobby", func() -> void:
			_message = ""
			Online.create_lobby(), Vector2(0, 116), 42, UI.VERDIGRIS))
		stack.add_child(UI.spacer(0, 0, true))
		var join_line := UI.row(12)
		_code = UI.field("Lobby code", typed)
		_code.max_length = 6
		_code.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		join_line.add_child(_code)
		join_line.add_child(UI.stamp("Join", func() -> void:
			_message = ""
			if _code.text.strip_edges() != "":
				Online.join_lobby(_code.text.strip_edges()), Vector2(200, 84), 32, UI.INK))
		stack.add_child(join_line)
		return panel

	var members: Array = Online.lobby["members"]
	var bots := int(Online.lobby.get("bots", 0))
	var seats := int(Online.lobby["max"])
	# The code is what a guest, or anyone not yet a friend, joins with.
	var code := String(Online.lobby["code"])
	var code_line := UI.row(16)
	var code_label := UI.label("Code  %s" % code, 52, UI.GILT, false)
	code_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	code_line.add_child(code_label)
	# Copies the code, to paste into a message to a friend.
	var copy := UI.stamp("Copy", func() -> void: pass, Vector2(150, 72), 30, UI.INK)
	copy.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	copy.pressed.connect(func() -> void:
		DisplayServer.clipboard_set(code)
		copy.text = "Copied")
	code_line.add_child(copy)
	stack.add_child(code_line)
	stack.add_child(UI.label("%d of %d seats taken" % [members.size() + bots, seats], 28, UI.SLATE, false))
	for member in members:
		var owner := String(member) == String(Online.lobby["owner"])
		stack.add_child(UI.label("%s%s" % [String(member), "  ·  owner" if owner else ""], 34, UI.PAPER, false))
	for index in bots:
		stack.add_child(UI.label("Bot %d" % (index + 1), 34, Color("9fb4d8"), false))
	for waiting in Online.lobby["invited"]:
		stack.add_child(UI.label("%s  ·  invited" % String(waiting), 30, UI.SLATE, false))
	stack.add_child(UI.spacer(0, 0, true))
	if Online.is_owner():
		var bot_line := UI.row(12)
		var add := UI.stamp("Add a bot", func() -> void:
			_message = ""
			Online.add_bot(), Vector2(0, 84), 30, UI.INK)
		add.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		add.disabled = members.size() + bots + (Online.lobby["invited"] as Array).size() >= seats
		bot_line.add_child(add)
		var remove := UI.stamp("Remove a bot", func() -> void:
			_message = ""
			Online.remove_bot(), Vector2(0, 84), 30, UI.INK)
		remove.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		remove.disabled = bots == 0
		bot_line.add_child(remove)
		stack.add_child(bot_line)
		var start := UI.stamp("Start the match", func() -> void:
			_message = ""
			Online.start_match(false), Vector2(0, 104), 38, UI.VERDIGRIS)
		start.disabled = members.size() + bots < 2
		stack.add_child(start)
	else:
		stack.add_child(UI.label("Waiting for %s to start the match." % String(Online.lobby["owner"]), 30, UI.SLATE))
	stack.add_child(UI.stamp("Leave the lobby", func() -> void: Online.leave_lobby(), Vector2(0, 84), 30, UI.INK))
	return panel
