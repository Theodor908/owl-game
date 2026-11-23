## Brings the game up to the newest build before the menus open. It asks
## the server which build is newest (`GET /api/version`). A newer build that
## fits the installed game is downloaded, checked and handed to `Boot`,
## which applies it when the game starts again. A build that needs a new
## shell, or one this device cannot load, needs the whole APK installed.
##
## `scripts/screens/update_gate.gd` shows what happens here. Holds no rules.
extends Node

## Nothing to do: the game is up to date, or the server cannot be reached.
## The game works offline with the build it has.
signal nothing_new
## A newer build is being downloaded: its number, what is new in it, and
## how much of it has arrived, from 0 to 1.
signal downloading(build: int, notes: String, share: float)
## The download did not arrive whole, or is not signed. Nothing of it is kept.
signal failed(build: int)
## The newer build is on the device and checked: start the game again.
signal ready_to_restart(build: int)
## The newer build needs the whole game installed again.
signal apk_needed(build: int, notes: String)

## How long the server gets to say which build is newest, in seconds. After
## that the game opens with the build it has.
@export var answer_timeout := 4.0
## How long a download may take before it counts as failed, in seconds.
@export var download_timeout := 900.0
## How much of a download is read at a time, in bytes. Godot reads one
## piece a frame, so this caps the speed: 256 KB is about 15 MB a second.
@export var download_chunk_size := 262144

var _http: HTTPRequest
## What is being fetched: the build, its note, the size of all its files,
## and how much of the files before the current one has arrived.
var _build := 0
var _notes := ""
var _total := 1.0
var _before := 0.0


## Asks the server and acts on the answer. Called again to retry.
func check() -> void:
	Online.fetch_version(_decide, func() -> void: nothing_new.emit(), answer_timeout)


func _decide(version: Dictionary) -> void:
	var newest := int(version.get("build", 0))
	if newest <= Online.build():
		nothing_new.emit()
		return
	var pack = version.get("pack")
	if not Boot.active or not _fits(pack) or int(pack["build"]) <= Online.build():
		_offer_apk(version)
		return
	# The library the pack needs: the installed one, one already here, or a download.
	var wanted := String(pack.get("native", ""))
	var lib = null
	var files: Array[Dictionary] = []
	if wanted != Boot.bundled_native:
		lib = version.get("lib")
		if not _fits(lib) or String(lib.get("native", "")) != wanted or String(lib.get("platform", "")) != Boot.platform:
			_offer_apk(version)
			return
		if not Boot.trusted(lib, "lib"):
			files.append(lib)
	if not Boot.trusted(pack, "pack"):
		files.append(pack)
	_build = int(pack["build"])
	_notes = String(version.get("notes", ""))
	_before = 0.0
	_total = 1.0
	for entry in files:
		_total += float(entry.get("size", 0))
	downloading.emit(_build, _notes, 0.0)
	_fetch(files, func() -> void:
		Boot.stage(pack, lib)
		ready_to_restart.emit(_build))


## True if a pack or library entry is for this game's shell and was not refused here before.
func _fits(entry: Variant) -> bool:
	return entry is Dictionary and String(entry.get("shell", "")) == Boot.shell \
		and not Boot.refused.has(String(entry.get("sha256", "")))


## Asks for the APK on the server. When that one is no newer than the
## installed game there is nothing this device can do, and the game opens.
func _offer_apk(version: Dictionary) -> void:
	var apk = version.get("apk")
	if Boot.active and apk is Dictionary and int(apk.get("build", 0)) <= Boot.apk_build:
		print("Updater: build %d cannot be used here and there is no newer APK" % int(version.get("build", 0)))
		nothing_new.emit()
		return
	apk_needed.emit(int(version.get("build", 0)), String(version.get("notes", "")))


## Downloads the files one after another, then calls `done`. Stops at the
## first that fails or does not check out.
func _fetch(files: Array[Dictionary], done: Callable) -> void:
	if files.is_empty():
		done.call()
		return
	var entry: Dictionary = files.pop_front()
	var kind := String(entry.get("kind", ""))
	var path := Boot.path_of(entry)
	var part := path + ".part"
	_http = HTTPRequest.new()
	_http.timeout = download_timeout
	_http.download_chunk_size = download_chunk_size
	_http.download_file = part
	add_child(_http)
	_http.request_completed.connect(func(result: int, status: int, _headers: PackedStringArray, _body: PackedByteArray) -> void:
		_http.queue_free()
		_http = null
		DirAccess.remove_absolute(path)
		var arrived := result == HTTPRequest.RESULT_SUCCESS and status == 200 and DirAccess.rename_absolute(part, path) == OK
		if arrived and Boot.trusted(entry, kind):
			_before += float(entry.get("size", 0))
			_fetch(files, done)
			return
		# Incomplete, changed on the way, or not signed: nothing of it is kept.
		print("Updater: the %s of build %d was dropped (%s)" % [kind, int(entry.get("build", 0)),
			"damaged or not signed" if arrived else "download failed: result %d, status %d" % [result, status]])
		DirAccess.remove_absolute(part)
		DirAccess.remove_absolute(path)
		failed.emit(_build))
	var file := "libowl_local.so" if kind == "lib" else "owl.pck"
	if _http.request(Online.download_address(file, int(entry.get("build", 0)))) != OK:
		_http.queue_free()
		_http = null
		failed.emit(_build)


func _process(_delta: float) -> void:
	if _http != null:
		downloading.emit(_build, _notes, clampf((_before + _http.get_downloaded_bytes()) / _total, 0.0, 1.0))
