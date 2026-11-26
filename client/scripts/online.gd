## The player's connection to the Owl server: the account, friends, the
## lobby, and the socket a match is played over (docs/protocol.md).
##
## Accounts and friends go over HTTP; lobbies and matches over one
## WebSocket. A match's views and events are handed to `Transport`, so the
## screens of a match work exactly as they do offline. Holds no rules.
extends Node

## Something here changed: login, connection, friends or the lobby.
signal changed
## The server refused something, or could not be reached. For the player to read.
signal failed(text: String)
## A friend invites the player to their lobby.
signal invited(from: String, code: String)
## The match of the player's lobby begins.
signal match_started
## The host refused a game command.
signal refused(text: String)

const FILE := "user://online.cfg"
## The game's own server. Players never type an address.
const DEFAULT_SERVER := "https://owl.predictionlegend.com"
const PROTOCOL := 8
## The privacy notice this build carries, readable offline. `android.bat`
## and `play.bat` copy it from `docs/privacy.md`.
const BUNDLED_NOTICE := "res://data/privacy.md"
const RECONNECT_MS := 3000

## The server in use, with no trailing slash: the game's own, unless a
## developer pointed this device at another one (`set_server`).
var server := DEFAULT_SERVER
## The session token, or "" when logged out. Kept on the device between runs.
var token := ""
var username := ""
## Playing as a guest: a made-up name and no password, erased on leaving.
var guest := false
## {"friends": [{"username", "online"}], "incoming": [names], "outgoing": [names]}
var friends := {"friends": [], "incoming": [], "outgoing": []}
## The lobby the player is in ({"code", "owner", "members", "invited", "bots", "max", "away"}), or empty.
var lobby := {}
## The socket is open and the server has said Welcome.
var connected := false
## The server's privacy notice changed since the player last confirmed it.
## Nothing online works until they read and confirm the new one.
var notice_pending := false
## The version of the notice the player last read, sent along when they
## confirm it, so a notice changed meanwhile is not confirmed unread.
var notice_read_version := 0
## The server has said whether the notice is confirmed, or cannot be reached.
var _notice_known := false
## The version of the notice in force on the server, once it has said; 0 before.
var server_notice_version := 0

var _socket: WebSocketPeer
var _said_hello := false
var _seq := 0
var _command_id := 0
var _retry_at := 0


func _ready() -> void:
	var file := ConfigFile.new()
	if file.load(FILE) == OK:
		server = String(file.get_value("online", "server", DEFAULT_SERVER))
		if server == "":
			server = DEFAULT_SERVER
		token = String(file.get_value("online", "token", ""))
		username = String(file.get_value("online", "username", ""))
		guest = bool(file.get_value("online", "guest", false))
	if logged_in():
		_check_notice()


func logged_in() -> bool:
	return token != ""


func is_owner() -> bool:
	return not lobby.is_empty() and String(lobby["owner"]).to_lower() == username.to_lower()


func _save() -> void:
	var file := ConfigFile.new()
	file.set_value("online", "server", server)
	file.set_value("online", "token", token)
	file.set_value("online", "username", username)
	file.set_value("online", "guest", guest)
	file.save(FILE)


## True when this device talks to a server other than the game's own.
func uses_other_server() -> bool:
	return server != DEFAULT_SERVER


## Points this device at another server; an empty address means the game's
## own again. For developers: the field for it is hidden in the profile.
## Only there may an address be unencrypted (`http://`), for a test server
## on the home network. Returns false if the address was refused.
func set_server(address: String) -> bool:
	var wanted := address.strip_edges().trim_suffix("/")
	if wanted == "":
		wanted = DEFAULT_SERVER
	if not (wanted.begins_with("https://") or wanted.begins_with("http://")):
		failed.emit("A server address starts with https://")
		return false
	if wanted != server:
		# Another server knows nothing of this session.
		token = ""
		username = ""
		lobby = {}
		connected = false
		_socket = null
		server = wanted
		changed.emit()
	_save()
	return true


# ---- HTTP ---------------------------------------------------------------------

## A server's message as a sentence: its first letter upper case, the rest
## as the server wrote it. (`capitalize()` would upper-case every word.)
static func sentence(text: String) -> String:
	var line := text.strip_edges()
	if line == "":
		return line
	line = line.left(1).to_upper() + line.substr(1)
	return line if line.ends_with(".") else line + "."


## What to show for a refused request. A refusal to wait (429, or 503 when
## the server is busy) carries a plain message, such as "too many wrong
## passwords; try again in 8 seconds", shown as it is.
static func _refusal(status: int, answer: Variant) -> String:
	if answer is Dictionary and answer.has("error"):
		return sentence(String(answer["error"]))
	if status == 429 or status == 503:
		return "The server is busy. Try again in a moment."
	return "The server refused (%d)." % status


## One request. `done` gets the status and the parsed answer (a Dictionary,
## or the raw text for the privacy notice). A refusal is announced on `failed`.
func _request(method: int, path: String, body: Variant, done: Callable) -> void:
	if server == "":
		failed.emit("Enter the server's address first.")
		return
	var http := HTTPRequest.new()
	http.timeout = 15.0
	add_child(http)
	http.request_completed.connect(func(result: int, status: int, _headers: PackedStringArray, raw: PackedByteArray) -> void:
		http.queue_free()
		var text := raw.get_string_from_utf8()
		var parsed = JSON.parse_string(text) if text.begins_with("{") or text.begins_with("[") else text
		if result != HTTPRequest.RESULT_SUCCESS:
			failed.emit("The server could not be reached.")
		elif status >= 400:
			failed.emit(_refusal(status, parsed))
		if done.is_valid():
			done.call(status if result == HTTPRequest.RESULT_SUCCESS else 0, parsed))
	var headers := PackedStringArray(["Content-Type: application/json"])
	if token != "":
		headers.append("Authorization: Bearer %s" % token)
	var error := http.request(server + path, headers, method, "" if body == null else JSON.stringify(body))
	if error != OK:
		http.queue_free()
		failed.emit("That server address does not work.")


func _logged_in_as(status: int, answer: Variant) -> void:
	if status < 200 or status >= 300 or not answer is Dictionary:
		return
	token = String(answer["token"])
	username = String(answer["username"])
	guest = bool(answer.get("guest", false))
	_save()
	_notice_answer(answer)
	changed.emit()


## Asks the server whether the player has confirmed its privacy notice.
func _check_notice() -> void:
	_notice_known = false
	_request(HTTPClient.METHOD_GET, "/api/me", null, func(status: int, answer: Variant) -> void:
		if status == 401:
			_forget()
		elif status == 200 and answer is Dictionary:
			_notice_answer(answer)
		else:
			# The server cannot be reached: the socket keeps trying meanwhile.
			_notice_known = true
		changed.emit())


func _notice_answer(answer: Dictionary) -> void:
	_notice_known = true
	server_notice_version = int(answer.get("notice_version", 0))
	notice_pending = not bool(answer.get("notice_accepted", true))
	if not notice_pending:
		_open_socket()


func register(account: String, password: String, notice_acknowledged: bool) -> void:
	_request(HTTPClient.METHOD_POST, "/api/register", _with_notice({"username": account, "password": password,
		"notice_acknowledged": notice_acknowledged}), _logged_in_as)


## Adds the version of the notice the player read, if they opened it here,
## so the server can refuse a confirmation of a notice that has since changed.
func _with_notice(body: Dictionary) -> Dictionary:
	if notice_read_version > 0:
		body["notice_version"] = notice_read_version
	return body


## Plays without an account, under a name the server makes up.
func play_as_guest(notice_acknowledged: bool) -> void:
	_request(HTTPClient.METHOD_POST, "/api/guest",
		_with_notice({"notice_acknowledged": notice_acknowledged}), _logged_in_as)


## This build's number, from `res://build.txt`; `release.bat` raises it.
## A downloaded pack brings its own, so this is the build in use.
func build() -> int:
	return int(FileAccess.get_file_as_string("res://build.txt").strip_edges())


## Asks the server which build of the game is newest. `done` gets the
## server's answer (`docs/protocol.md`, `GET /api/version`); `unreachable`
## is called instead when no answer comes within `timeout` seconds. Quiet
## either way: the game works offline.
func fetch_version(done: Callable, unreachable: Callable, timeout: float) -> void:
	var http := HTTPRequest.new()
	http.timeout = timeout
	add_child(http)
	http.request_completed.connect(func(result: int, status: int, _headers: PackedStringArray, raw: PackedByteArray) -> void:
		http.queue_free()
		var answer = JSON.parse_string(raw.get_string_from_utf8()) if result == HTTPRequest.RESULT_SUCCESS and status == 200 else null
		if answer is Dictionary:
			done.call(answer)
		else:
			unreachable.call())
	if http.request(server + "/api/version") != OK:
		http.queue_free()
		unreachable.call()


## Where a file of a build is downloaded from: `owl.apk`, `owl.pck` or
## `libowl_local.so`. The build number makes each build's address its own,
## so nothing on the way can hand out an older copy.
func download_address(file: String, wanted: int) -> String:
	return server + "/download/%s?build=%d" % [file, wanted]


func login(account: String, password: String) -> void:
	_request(HTTPClient.METHOD_POST, "/api/login", {"username": account, "password": password}, _logged_in_as)


## Forgets the session on this device, whatever the server answers.
func _forget() -> void:
	token = ""
	username = ""
	guest = false
	notice_pending = false
	lobby = {}
	friends = {"friends": [], "incoming": [], "outgoing": []}
	connected = false
	_socket = null
	_save()
	changed.emit()


func logout() -> void:
	_request(HTTPClient.METHOD_POST, "/api/logout", null, Callable())
	_forget()


## The privacy notice. The one this build carries opens at once: every
## release carries the notice in force, and the game updates before it
## opens. Only when the server has said its notice is newer than the
## carried one (`server_notice_version`) is it fetched. `done` gets the text.
func fetch_privacy(done: Callable) -> void:
	var carried := FileAccess.get_file_as_string(BUNDLED_NOTICE)
	if carried != "" and notice_version(carried) >= server_notice_version:
		notice_read_version = notice_version(carried)
		done.call(carried)
		return
	_request(HTTPClient.METHOD_GET, "/api/notice", null, func(status: int, answer: Variant) -> void:
		if status == 200 and answer is Dictionary:
			notice_read_version = int(answer["version"])
			done.call(String(answer["text"]))
		else:
			var text := FileAccess.get_file_as_string(BUNDLED_NOTICE)
			if text != "":
				notice_read_version = notice_version(text)
				done.call(text))


## The version a notice states on a line of its own ("Version 2."); 1 if none.
static func notice_version(text: String) -> int:
	for line in text.split("\n"):
		var plain := line.strip_edges()
		if plain.begins_with("Version "):
			return int(plain.trim_prefix("Version ").trim_suffix("."))
	return 1


## The player confirms the notice they just read. Online play opens again.
func accept_notice() -> void:
	_request(HTTPClient.METHOD_POST, "/api/me/notice", {"version": notice_read_version},
		func(status: int, _answer: Variant) -> void:
			if status == 204:
				notice_pending = false
				_open_socket()
				changed.emit())


## Asks for everything the server holds about the player.
func export_data(done: Callable) -> void:
	_request(HTTPClient.METHOD_GET, "/api/me/export", null, func(status: int, answer: Variant) -> void:
		if status == 200:
			done.call(answer))


func delete_account(password: String) -> void:
	_request(HTTPClient.METHOD_DELETE, "/api/me", {"password": password}, func(status: int, _answer: Variant) -> void:
		if status == 204:
			_forget())


func change_password(old: String, new: String) -> void:
	_request(HTTPClient.METHOD_POST, "/api/me/password", {"old": old, "new": new}, func(status: int, _answer: Variant) -> void:
		# Every session ended, this one included.
		if status == 204:
			_forget())


func change_username(password: String, new: String) -> void:
	_request(HTTPClient.METHOD_POST, "/api/me/username", {"password": password, "username": new},
		func(status: int, answer: Variant) -> void:
			if status == 200 and answer is Dictionary:
				username = String(answer["username"])
				_save()
				_open_socket()
				changed.emit())


func refresh_friends() -> void:
	if not logged_in():
		return
	_request(HTTPClient.METHOD_GET, "/api/friends", null, func(status: int, answer: Variant) -> void:
		if status == 200 and answer is Dictionary:
			friends = answer
			changed.emit()
		elif status == 401:
			_forget())


func _friend_call(path: String, friend: String) -> void:
	_request(HTTPClient.METHOD_POST, path, {"username": friend}, func(_status: int, _answer: Variant) -> void: refresh_friends())


func add_friend(friend: String) -> void:
	_friend_call("/api/friends/request", friend)


func accept_friend(friend: String) -> void:
	_friend_call("/api/friends/accept", friend)


func remove_friend(friend: String) -> void:
	_friend_call("/api/friends/remove", friend)


# ---- WebSocket ----------------------------------------------------------------

func _open_socket() -> void:
	connected = false
	_said_hello = false
	_seq = 0
	_socket = WebSocketPeer.new()
	var address := server.replace("https://", "wss://").replace("http://", "ws://") + "/ws"
	if _socket.connect_to_url(address) != OK:
		_socket = null
		_retry_at = Time.get_ticks_msec() + RECONNECT_MS


func _send(type: String, payload: Variant = null) -> void:
	if _socket == null or _socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return
	_seq += 1
	var message := {"v": PROTOCOL, "seq": _seq, "type": type}
	if payload != null:
		message["payload"] = payload
	_socket.send_text(JSON.stringify(message))


## Back from the background, the game reconnects at once instead of
## waiting for the next retry. A socket the phone left stale is closed first.
func _notification(what: int) -> void:
	if what != NOTIFICATION_APPLICATION_RESUMED and what != NOTIFICATION_APPLICATION_FOCUS_IN:
		return
	if _socket != null:
		_socket.poll()
		var state := _socket.get_ready_state()
		if state == WebSocketPeer.STATE_OPEN or state == WebSocketPeer.STATE_CONNECTING:
			return
		_socket.close()
		_socket = null
		if connected:
			connected = false
			changed.emit()
	_retry_at = 0


func _process(_delta: float) -> void:
	if _socket == null:
		if logged_in() and _notice_known and not notice_pending and Time.get_ticks_msec() >= _retry_at:
			_open_socket()
		return
	_socket.poll()
	match _socket.get_ready_state():
		WebSocketPeer.STATE_OPEN:
			if not _said_hello:
				_said_hello = true
				_send("Hello", {"v": PROTOCOL, "token": token})
			while _socket != null and _socket.get_available_packet_count() > 0:
				var message = JSON.parse_string(_socket.get_packet().get_string_from_utf8())
				if message is Dictionary:
					_on_message(String(message.get("type", "")), message.get("payload", {}))
		WebSocketPeer.STATE_CLOSED:
			var was := connected
			connected = false
			_socket = null
			_retry_at = Time.get_ticks_msec() + RECONNECT_MS
			if was:
				changed.emit()


func _on_message(type: String, payload: Variant) -> void:
	match type:
		"Welcome":
			connected = true
			username = String(payload["username"])
			# The server says the lobby again right after, if the place was kept.
			lobby = {}
			Store.set_content(payload["content"])
			refresh_friends()
			changed.emit()
		"LobbyState":
			lobby = payload
			changed.emit()
		"LobbyClosed":
			lobby = {}
			changed.emit()
		"Invited":
			invited.emit(String(payload["from"]), String(payload["code"]))
		"MatchStarted":
			Store.seat_names = payload["seats"]
			Transport.go_online()
			match_started.emit()
		"Snapshot":
			Transport.receive(payload["view"], [], int(payload["now"]))
		"Events":
			Transport.receive(payload["view"], payload["events"], int(payload["now"]))
		"Rejected":
			refused.emit(String(payload["reason"]).capitalize())
		"Error":
			# The server closes the connection after saying so. A changed
			# privacy notice keeps the session; anything else before Welcome
			# means the session is bad.
			var reason := String(payload["reason"])
			if reason.contains("privacy notice"):
				notice_pending = true
				_socket = null
				changed.emit()
			elif not connected:
				_forget()
			failed.emit(sentence(reason))


func create_lobby() -> void:
	_send("CreateLobby")


func invite(friend: String) -> void:
	_send("Invite", {"username": friend})


func join_lobby(code: String) -> void:
	_send("JoinLobby", {"code": code})


func add_bot() -> void:
	_send("AddBot")


func remove_bot() -> void:
	_send("RemoveBot")


func decline_invite(code: String) -> void:
	_send("DeclineInvite", {"code": code})


func leave_lobby() -> void:
	_send("LeaveLobby")


func start_match(fill_with_bots: bool) -> void:
	_send("StartMatch", {"fill_with_bots": fill_with_bots})


func send_command(command: Dictionary) -> void:
	_command_id += 1
	_send("Command", {"id": _command_id, "command": command})
