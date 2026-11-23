## The profile: the name used in offline games, and the online account.
## Logged out, it offers logging in or making an account, after the privacy
## notice. Logged in, it offers everything the notice promises: exporting
## the data, changing the name or the password, and deleting the account.
## Holds no rules.
extends Control

signal back
## The player wants their friends and lobby.
signal friends

## "", "notice", "username", "password" or "delete": the form or sheet that is open.
var _form := ""
var _message := ""
var _message_good := false
var _notice_read := false
var _notice_text := ""
var _fields := {}
## The server address field is showing. It is for developers: it appears
## after several taps on the "Online account" heading, or when this device
## already uses another server than the game's own.
var _developer := false
var _heading_taps := 0
const TAPS_FOR_DEVELOPER := 7


func _ready() -> void:
	Online.changed.connect(_refresh)
	Online.failed.connect(_fail)
	_refresh()


func _exit_tree() -> void:
	Online.changed.disconnect(_refresh)
	Online.failed.disconnect(_fail)


func _fail(text: String) -> void:
	_say(text, false)


func _say(text: String, good := true) -> void:
	_message = text
	_message_good = good
	_refresh()


func _field(key: String, placeholder: String, text := "", secret := false) -> LineEdit:
	var field := UI.field(placeholder, text, secret)
	# What was typed survives a rebuild of the screen.
	if _fields.has(key) and is_instance_valid(_fields[key]):
		field.text = (_fields[key] as LineEdit).text
	_fields[key] = field
	return field


func _text(key: String) -> String:
	return (_fields[key] as LineEdit).text if _fields.has(key) and is_instance_valid(_fields[key]) else ""


func _refresh() -> void:
	# Keep the typed text across the rebuild.
	var kept := {}
	for key in _fields:
		if is_instance_valid(_fields[key]):
			kept[key] = (_fields[key] as LineEdit).text
	UI.clear(self)
	_fields = {}

	var top := UI.row(24)
	top.set_anchors_preset(Control.PRESET_TOP_WIDE)
	top.custom_minimum_size.y = 96
	top.add_child(UI.stamp("Back", func() -> void: back.emit(), Vector2(200, 92), 36, UI.INK))
	var title := UI.label("Profile", UI.GLANCE, UI.PAPER, false)
	title.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	top.add_child(title)
	add_child(top)

	var holder := CenterContainer.new()
	holder.set_anchors_preset(Control.PRESET_FULL_RECT)
	holder.offset_top = 100
	add_child(holder)
	var sheet := UI.panel(UI.INK_RAISED, 16, 30)
	sheet.custom_minimum_size.x = 1100
	holder.add_child(sheet)
	var stack := UI.column(14)
	sheet.add_child(stack)

	if _form == "notice":
		_build_notice(stack)
	elif _form != "":
		_build_form(stack)
	elif Online.logged_in():
		_build_account(stack)
	else:
		_build_login(stack)
	if _message != "":
		stack.add_child(UI.centered(_message, 28, UI.VERDIGRIS if _message_good else UI.VERMILION))
	for key in kept:
		if _fields.has(key):
			(_fields[key] as LineEdit).text = kept[key]


func _build_login(stack: VBoxContainer) -> void:
	stack.add_child(UI.label("Your name in offline games", 30, UI.SLATE, false))
	var offline := _field("offline", "Your name", Store.my_name)
	offline.max_length = Store.NAME_LENGTH
	offline.text_changed.connect(Store.set_my_name)
	stack.add_child(offline)

	var heading := Button.new()
	UI.bare(heading)
	heading.custom_minimum_size.y = 60
	var words := UI.label("Online account", 44, UI.PAPER, false)
	words.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	UI.fill(words)
	heading.add_child(words)
	UI.quiet(heading)
	heading.pressed.connect(func() -> void:
		_heading_taps += 1
		if _heading_taps >= TAPS_FOR_DEVELOPER and not _developer:
			_developer = true
			_refresh())
	stack.add_child(heading)
	if _developer or Online.uses_other_server():
		stack.add_child(UI.label("Developer: another server. Leave empty for the game's own. An http:// address is not encrypted; use it only for a test server at home.", 24, UI.SLATE))
		stack.add_child(_field("server", Online.DEFAULT_SERVER, Online.server if Online.uses_other_server() else ""))
	var line := UI.row(14)
	var account := _field("username", "Username")
	account.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line.add_child(account)
	var password := _field("password", "Password (15 characters or more)", "", true)
	password.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line.add_child(password)
	stack.add_child(line)

	var notice := UI.row(14)
	var read := UI.stamp("I have read the privacy notice: %s" % ("yes" if _notice_read else "no"), func() -> void:
		_notice_read = not _notice_read
		_refresh(), Vector2(0, 88), 30, UI.INK)
	read.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	notice.add_child(read)
	notice.add_child(UI.stamp("Read it", _open_notice, Vector2(240, 88), 30, UI.INK))
	stack.add_child(notice)

	var buttons := UI.row(14)
	var log_in := UI.stamp("Log in", _log_in.bind(false), Vector2(0, 104), 38, UI.VERDIGRIS)
	log_in.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(log_in)
	var create := UI.stamp("Create account", _log_in.bind(true), Vector2(0, 104), 38, UI.INK)
	create.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(create)
	stack.add_child(buttons)
	stack.add_child(UI.stamp("Play as a guest, without an account", _guest, Vector2(0, 92), 32, UI.INK))
	stack.add_child(UI.label("An account holds a username and a password, nothing else. A guest gets a made-up name and is forgotten on leaving.", 26, UI.SLATE))


## Applies the developer's server address, if the field is showing.
## False if it was refused.
func _apply_server() -> bool:
	return not _fields.has("server") or Online.set_server(_text("server"))


func _log_in(create: bool) -> void:
	_message = ""
	if not _apply_server():
		return
	if create and not _notice_read:
		_say("Read the privacy notice and confirm it first.", false)
	elif create:
		Online.register(_text("username"), _text("password"), true)
	else:
		Online.login(_text("username"), _text("password"))


func _guest() -> void:
	_message = ""
	if not _apply_server():
		return
	if not _notice_read:
		_say("Read the privacy notice and confirm it first.", false)
	else:
		Online.play_as_guest(true)


func _open_notice() -> void:
	_message = ""
	if not _apply_server():
		return
	Online.fetch_privacy(func(text: String) -> void:
		_notice_text = text
		_form = "notice"
		_refresh())


func _build_notice(stack: VBoxContainer) -> void:
	stack.add_child(UI.label("Privacy notice, version %d" % Online.notice_read_version, 44, UI.PAPER, false))
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size.y = 520
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var text := UI.label(_notice_text.replace("**", "").replace("`", ""), 26, Color("e6eaf2"))
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(text)
	stack.add_child(scroll)
	if Online.logged_in() and Online.notice_pending:
		var line := UI.row(14)
		var accept := UI.stamp("I have read it", func() -> void:
			Online.accept_notice()
			_close_form(), Vector2(0, 96), 34, UI.VERDIGRIS)
		accept.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		line.add_child(accept)
		var later := UI.stamp("Not now", _close_form, Vector2(0, 96), 34, UI.INK)
		later.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		line.add_child(later)
		stack.add_child(line)
		return
	stack.add_child(UI.stamp("Close", _close_form, Vector2(0, 96), 34, UI.INK))


func _build_account(stack: VBoxContainer) -> void:
	stack.add_child(UI.label("%s %s" % ["Playing as guest" if Online.guest else "Logged in as", Online.username], 48, UI.PAPER, false))
	if not Online.notice_pending:
		stack.add_child(UI.label("Connected to the server" if Online.connected else "Connecting to the server…",
			26, UI.SLATE, false))
	if Online.notice_pending:
		# Online play waits until the changed notice is read and confirmed.
		stack.add_child(UI.label("The privacy notice has changed. Read it and confirm it to play online again.", 30, UI.GILT))
		stack.add_child(UI.stamp("Read the new privacy notice", _open_notice_logged_in, Vector2(0, 110), 40, UI.VERDIGRIS))
	else:
		stack.add_child(UI.stamp("Friends and lobby", func() -> void: friends.emit(), Vector2(0, 110), 40, UI.VERDIGRIS))
	if Online.guest:
		# A guest has a name and nothing else: no password to change, and
		# leaving erases it.
		stack.add_child(UI.label("Friends can find you by this name. It is forgotten when you leave; make an account to keep one.", 28, Color("e6eaf2")))
		var line := UI.row(14)
		var read := UI.stamp("Privacy notice", _open_notice_logged_in, Vector2(0, 92), 32, UI.INK)
		read.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		line.add_child(read)
		var leave := UI.stamp("Stop being a guest", func() -> void: Online.logout(), Vector2(0, 92), 32, UI.INK)
		leave.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		line.add_child(leave)
		stack.add_child(line)
		return
	var first := UI.row(14)
	for entry in [["Change username", "username"], ["Change password", "password"]]:
		var button := UI.stamp(String(entry[0]), _open_form.bind(String(entry[1])), Vector2(0, 92), 32, UI.INK)
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		first.add_child(button)
	stack.add_child(first)
	var second := UI.row(14)
	var export := UI.stamp("Export my data", _export, Vector2(0, 92), 32, UI.INK)
	export.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	second.add_child(export)
	var notice := UI.stamp("Privacy notice", _open_notice_logged_in, Vector2(0, 92), 32, UI.INK)
	notice.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	second.add_child(notice)
	stack.add_child(second)
	var third := UI.row(14)
	var out := UI.stamp("Log out", func() -> void: Online.logout(), Vector2(0, 92), 32, UI.INK)
	out.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	third.add_child(out)
	var delete := UI.stamp("Delete my account", _open_form.bind("delete"), Vector2(0, 92), 32, UI.INK)
	var red := UI.skin("plate_danger", 40, 34, 10)
	if red != null:
		UI.face_skin(delete, red)
	delete.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	third.add_child(delete)
	stack.add_child(third)


func _open_notice_logged_in() -> void:
	_message = ""
	Online.fetch_privacy(func(text: String) -> void:
		_notice_text = text
		_form = "notice"
		_refresh())


func _open_form(form: String) -> void:
	_form = form
	_message = ""
	_refresh()


func _close_form() -> void:
	_form = ""
	_refresh()


func _build_form(stack: VBoxContainer) -> void:
	match _form:
		"username":
			stack.add_child(UI.label("Change username", 44, UI.PAPER, false))
			stack.add_child(_field("new_username", "New username"))
			stack.add_child(_field("confirm", "Your password", "", true))
		"password":
			stack.add_child(UI.label("Change password", 44, UI.PAPER, false))
			stack.add_child(_field("confirm", "Your password now", "", true))
			stack.add_child(_field("new_password", "New password (15 characters or more)", "", true))
			stack.add_child(UI.label("You will have to log in again, here and on any other device.", 26, UI.SLATE))
		"delete":
			stack.add_child(UI.label("Delete my account", 44, UI.PAPER, false))
			stack.add_child(UI.label("This erases your account, your friends list and your sessions from the server at once. It cannot be undone.", 28, Color("e6eaf2")))
			stack.add_child(_field("confirm", "Your password", "", true))
	var buttons := UI.row(14)
	var confirm := UI.stamp("Delete it" if _form == "delete" else "Confirm", _submit_form, Vector2(0, 100), 36, UI.VERDIGRIS)
	confirm.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(confirm)
	var cancel := UI.stamp("Cancel", _close_form, Vector2(0, 100), 36, UI.INK)
	cancel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(cancel)
	stack.add_child(buttons)


func _submit_form() -> void:
	_message = ""
	match _form:
		"username":
			Online.change_username(_text("confirm"), _text("new_username"))
		"password":
			Online.change_password(_text("confirm"), _text("new_password"))
		"delete":
			Online.delete_account(_text("confirm"))
	# A refusal arrives on `failed` and reopens nothing; success changes the screen.
	_form = ""
	_refresh()


func _export() -> void:
	_message = ""
	Online.export_data(func(data: Variant) -> void:
		var path := "user://owl-my-data.json"
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file == null:
			_say("The file could not be written.", false)
			return
		file.store_string(JSON.stringify(data, "  "))
		file.close()
		_say("Saved to %s" % ProjectSettings.globalize_path(path)))
