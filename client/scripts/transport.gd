## The one door between the client and whoever hosts the match.
##
## Offline, the host is the Rust `LocalLobby` extension running in this
## process. Online, the host is the server: `Online` owns the socket and
## hands every view and its events to `receive`, and commands go back through
## `Online`. The JSON is the same either way, so nothing above this script
## knows which host it is.
extends Node

## Emitted with the events the host sent since the last frame, in order.
signal events_received(items: Array)

## How fast the match clock runs. 1 is real time; tests and screenshot runs speed it up.
var time_scale := 1.0

## The match in progress is hosted by the server.
var online := false

var _lobby: Object
var _clock_ms := 0.0
var _running := false
## The server's match clock when it last spoke, and this device's clock then.
var _remote_ms := 0
var _remote_at := 0


func _ready() -> void:
	_lobby = ClassDB.instantiate("LocalLobby")
	if _lobby == null:
		push_error("The Rust extension is not loaded. Build it with: cargo build -p owl-local")
		return
	var error: String = _lobby.load_content(_data_dir())
	if error != "":
		push_error(error)
		return
	Store.set_content(JSON.parse_string(_lobby.content_json()))


## The folder of content files, as a path the host can read from disk. In
## the editor that is `data/` beside the project. An exported game carries a
## copy inside its package (`res://data`), which the host cannot open, so
## the copy is first written out to the app's own storage. It is written
## again on every start: a downloaded build brings its own content.
func _data_dir() -> String:
	if OS.has_feature("editor") and not Boot.active:
		return ProjectSettings.globalize_path("res://").path_join("../data")
	_copy_out("res://data", "user://data")
	return ProjectSettings.globalize_path("user://data")


## Writes the content out. `files.txt` (made by `android.bat`) lists what
## the build carries, so files of an older build do not linger; without it
## the folder is copied as it is found.
func _copy_out(from: String, to: String) -> void:
	var listed := FileAccess.get_file_as_string(from.path_join("files.txt"))
	if listed == "":
		_copy_folder(from, to)
		return
	_remove_folder(to)
	for line in listed.split("\n", false):
		var file := line.strip_edges()
		if file == "" or file.contains(".."):
			continue
		DirAccess.make_dir_recursive_absolute(to.path_join(file).get_base_dir())
		_copy_file(from.path_join(file), to.path_join(file))


func _copy_folder(from: String, to: String) -> void:
	DirAccess.make_dir_recursive_absolute(to)
	for folder in DirAccess.get_directories_at(from):
		_copy_folder(from.path_join(folder), to.path_join(folder))
	for file in DirAccess.get_files_at(from):
		_copy_file(from.path_join(file), to.path_join(file))


func _copy_file(from: String, to: String) -> void:
	var out := FileAccess.open(to, FileAccess.WRITE)
	if out != null:
		out.store_buffer(FileAccess.get_file_as_bytes(from))


func _remove_folder(path: String) -> void:
	if not DirAccess.dir_exists_absolute(path):
		return
	for folder in DirAccess.get_directories_at(path):
		_remove_folder(path.path_join(folder))
	for file in DirAccess.get_files_at(path):
		DirAccess.remove_absolute(path.path_join(file))
	DirAccess.remove_absolute(path)


## The match clock in milliseconds. All deadlines from the host use it.
func now() -> int:
	if online:
		# The server's clock, carried forward by the time since it last spoke.
		return _remote_ms + (Time.get_ticks_msec() - _remote_at)
	return int(_clock_ms)


## The next match is hosted by the server.
func go_online() -> void:
	online = true
	_running = false
	Store.view = {}


func go_offline() -> void:
	online = false


## A view from the server, the events that led to it, and its match clock.
func receive(view: Variant, events: Array, server_ms: int) -> void:
	if not online:
		return
	_remote_ms = server_ms
	_remote_at = Time.get_ticks_msec()
	Store.set_view(view)
	if not events.is_empty():
		events_received.emit(events)


## Stops driving the match, when the player leaves it.
func stop_match() -> void:
	_running = false


func start_match(match_seed: int) -> void:
	if _lobby == null:
		return
	online = false
	_lobby.start(match_seed, now())
	_running = true
	_pump()


## Sends one command for the local player. Returns "" if the host accepted
## it, otherwise the reason it was refused.
func send(command: Dictionary) -> String:
	if online:
		# The answer comes later, on `Online.refused` if it is a refusal.
		Online.send_command(command)
		return ""
	if _lobby == null:
		return "no host"
	var error: String = _lobby.send(now(), JSON.stringify(command))
	_pump()
	return error


func _process(delta: float) -> void:
	if not _running:
		return
	_clock_ms += delta * 1000.0 * time_scale
	_lobby.tick(now())
	_pump()


## Reads new events from the host, refreshes the snapshot, then announces the events.
func _pump() -> void:
	var items: Array = JSON.parse_string(_lobby.drain())
	Store.set_view(JSON.parse_string(_lobby.view()))
	if not items.is_empty():
		events_received.emit(items)
