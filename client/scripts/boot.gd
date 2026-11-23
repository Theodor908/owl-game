## The loader: the first autoload, so its `_init` runs before any other
## script of the game is read. It puts a downloaded build in place of the
## one the installed game carries: first the native library (the rules),
## then the pack (scripts, scenes, pictures and content).
##
## This script and the library inside the installed game are the only code
## a download cannot replace, so everything is checked here, on every start:
## each file against the size, the SHA-256 and the signature of its entry in
## the server's `version.json`. The signature is checked by `ReleaseCheck`
## from the installed library, which holds the public half of the release
## key. A file that fails is dropped and the installed build runs instead.
##
## A pack names the library it needs (`native`) and both name the shell they
## run in (`shell`: the engine, the project settings and this script). A
## change to the shell needs a whole new APK.
##
## Holds no rules. `scripts/updater.gd` downloads, before the menus open;
## this script applies what it left, when the game starts again.
extends Node

## What the installed game is: `{build, shell, native, platform}`, written
## by `android.bat`. Read before any pack is loaded, so it is the APK's own.
const BASE := "res://base.json"
const FOLDER := "user://update"
## The downloaded build to apply: `{"pack": entry, "lib": entry or null}`.
const CURRENT := FOLDER + "/current.json"
## Counts starts that began applying a download and never finished.
const TRYING := FOLDER + "/trying"
## SHA-256 of downloads that failed here and are never tried again.
const REFUSED := FOLDER + "/refused.json"
## The description the downloaded library is loaded through.
const EXTENSION := FOLDER + "/owl.gdextension"
## How many starts may die while applying a download before it is refused.
const TRIES := 2
## How many frames the game must draw before a start counts as good.
const GOOD_AFTER_FRAMES := 30

## False when this run takes no downloads: in the editor, or in a game
## built without `base.json`.
var active := false
## The build, shell and library of the installed game.
var apk_build := 0
var shell := ""
var bundled_native := ""
## This device, as a library entry names it: `android.arm64`.
var platform := ""
## The library in use now: the installed one, or a downloaded one.
var native := ""
## The build in use now.
var build := 0
## The downloaded build waiting or in use (the content of `CURRENT`).
var staged := {}
## SHA-256 of refused downloads.
var refused := {}
## What the loader did, for the log and for tests.
var notes: PackedStringArray = []
## The description the installed library was loaded from, once it is unloaded.
var _installed := ""


func _init() -> void:
	platform = "%s.%s" % [OS.get_name().to_lower(), Engine.get_architecture_name()]
	var base = _read(BASE)
	if not base is Dictionary or String(base.get("platform", "")) != platform:
		return
	# A game run from the editor is the project itself; `--updates` is for testing the loader.
	if OS.has_feature("editor") and not OS.get_cmdline_user_args().has("--updates"):
		return
	apk_build = int(base.get("build", 0))
	shell = String(base.get("shell", ""))
	bundled_native = String(base.get("native", ""))
	native = bundled_native
	build = apk_build
	active = shell != "" and bundled_native != ""
	if not active:
		return
	DirAccess.make_dir_recursive_absolute(FOLDER)
	var known = _read(REFUSED)
	if known is Dictionary:
		refused = known
	var current = _read(CURRENT)
	if current is Dictionary:
		staged = current
		_apply()
	else:
		# Nothing waits: what an unfinished download left behind goes.
		_sweep()
	for line in notes:
		print("Boot: ", line)


func _ready() -> void:
	if not FileAccess.file_exists(TRYING):
		return
	for frame in GOOD_AFTER_FRAMES:
		await get_tree().process_frame
	DirAccess.remove_absolute(TRYING)


## Where a download is kept. The name comes from its SHA-256, so a file is
## never written over while it is in use.
func path_of(entry: Dictionary) -> String:
	var ending := "pck"
	if String(entry.get("kind", "")) == "lib":
		ending = {"windows": "dll", "macos": "dylib"}.get(OS.get_name().to_lower(), "so")
	return "%s/%s.%s" % [FOLDER, String(entry.get("sha256", "")).left(16), ending]


## True if the file of `entry` is on this device, whole, and signed with the
## release key. `kind` is what the caller expects: "pack" or "lib".
func trusted(entry: Dictionary, kind: String) -> bool:
	return _trusted(ClassDB.instantiate("ReleaseCheck"), entry, kind)


## Keeps a downloaded build for the next start. `lib` is null when the pack
## runs on the installed library.
func stage(pack: Dictionary, lib: Variant) -> void:
	staged = {"pack": pack, "lib": lib}
	DirAccess.remove_absolute(TRYING)
	_write(CURRENT, staged)


## The build that is downloaded and starts next time, or 0.
func staged_build() -> int:
	var pack = staged.get("pack")
	return int(pack.get("build", 0)) if pack is Dictionary else 0


func _apply() -> void:
	var pack = staged.get("pack")
	var lib = staged.get("lib")
	if not pack is Dictionary or String(pack.get("shell", "")) != shell or int(pack.get("build", 0)) <= apk_build:
		_forget("the download is not for this game, or the installed game is as new")
		return
	# A start that dies here leaves the count behind; the next one sees it.
	var tries := int(FileAccess.get_file_as_string(TRYING)) if FileAccess.file_exists(TRYING) else 0
	if tries >= TRIES:
		_refuse(pack)
		if lib is Dictionary:
			_refuse(lib)
		_forget("build %d did not start %d times and is refused" % [int(pack["build"]), tries])
		return
	_write_text(TRYING, str(tries + 1))

	# Everything is checked before anything is loaded: a new library is
	# never left running under the installed scripts.
	var check: RefCounted = ClassDB.instantiate("ReleaseCheck")
	# A build that fails here is refused, so the game is never sent round
	# and round between downloading it and dropping it.
	if not _trusted(check, pack, "pack"):
		_refuse(pack)
		_forget("the pack of build %d is missing, damaged or not signed" % int(pack["build"]))
		return
	var wanted := String(pack.get("native", ""))
	var needs_library := wanted != bundled_native
	if needs_library:
		var fits: bool = lib is Dictionary and String(lib.get("native", "")) == wanted \
			and String(lib.get("platform", "")) == platform and String(lib.get("shell", "")) == shell
		if not fits or not _trusted(check, lib, "lib"):
			_refuse(pack)
			_forget("the library build %d needs is missing, damaged or not signed" % int(pack["build"]))
			return
	# The installed library is about to be unloaded: nothing of it may stay.
	check = null
	if needs_library and not _load_library(lib):
		_refuse(lib)
		_forget("this device did not load the downloaded library; the installed one is in use")
		return
	if not ProjectSettings.load_resource_pack(path_of(pack), true):
		if needs_library:
			_load_installed_library()
		_refuse(pack)
		_forget("the pack of build %d did not load" % int(pack["build"]))
		return
	build = int(pack["build"])
	notes.append("build %d in use over installed build %d%s" % [build, apk_build,
		", with its own library" if needs_library else ""])
	_sweep()


func _trusted(check: Variant, entry: Variant, kind: String) -> bool:
	if check == null or not entry is Dictionary or String(entry.get("kind", "")) != kind:
		return false
	var sha := String(entry.get("sha256", ""))
	if sha.length() != 64 or not sha.is_valid_hex_number() or refused.has(sha):
		return false
	# A library is named by its own SHA-256, which is what a pack asks for.
	if kind == "lib" and String(entry.get("native", "")) != sha:
		return false
	var path := path_of(entry)
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null or file.get_length() != int(entry.get("size", -1)):
		return false
	file = null
	return FileAccess.get_sha256(path) == sha and bool(check.signed(JSON.stringify(entry)))


## Swaps the installed library for a downloaded one. False, with the
## installed one back in place, if the device will not load it.
func _load_library(lib: Dictionary) -> bool:
	var installed := _installed_extension()
	if installed == "":
		return false
	# The same description as the installed one, pointing at the new file.
	var description := ConfigFile.new()
	var original := ConfigFile.new()
	if original.load(installed) != OK:
		return false
	for key in original.get_section_keys("configuration"):
		description.set_value("configuration", key, original.get_value("configuration", key))
	description.set_value("libraries", platform, path_of(lib).get_file())
	if description.save(EXTENSION) != OK:
		return false
	_installed = installed
	if GDExtensionManager.unload_extension(installed) != GDExtensionManager.LOAD_STATUS_OK:
		return false
	if GDExtensionManager.load_extension(EXTENSION) == GDExtensionManager.LOAD_STATUS_OK \
			and ClassDB.class_exists("LocalLobby") and ClassDB.class_exists("ReleaseCheck"):
		native = String(lib["native"])
		return true
	_load_installed_library()
	return false


func _load_installed_library() -> void:
	if GDExtensionManager.is_extension_loaded(EXTENSION):
		GDExtensionManager.unload_extension(EXTENSION)
	if _installed != "" and not GDExtensionManager.is_extension_loaded(_installed):
		GDExtensionManager.load_extension(_installed)
	native = bundled_native


## The path the installed game loaded its own library description from.
func _installed_extension() -> String:
	for path in GDExtensionManager.get_loaded_extensions():
		if path.get_file() == "owl.gdextension" and not path.begins_with("user://"):
			return path
	return ""


func _refuse(entry: Dictionary) -> void:
	refused[String(entry.get("sha256", ""))] = true
	_write(REFUSED, refused)


## Drops the downloaded build: the installed one runs.
func _forget(why: String) -> void:
	notes.append(why)
	staged = {}
	_sweep()


## Deletes every file of the folder that the build in `staged` does not use.
func _sweep() -> void:
	var keep := [REFUSED.get_file()]
	if not staged.is_empty():
		keep.append_array([CURRENT.get_file(), TRYING.get_file(), EXTENSION.get_file(), path_of(staged["pack"]).get_file()])
		if staged.get("lib") is Dictionary:
			keep.append(path_of(staged["lib"]).get_file())
	for file in DirAccess.get_files_at(FOLDER):
		if not keep.has(file):
			DirAccess.remove_absolute(FOLDER.path_join(file))


func _read(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		return null
	return JSON.parse_string(FileAccess.get_file_as_string(path))


func _write(path: String, value: Variant) -> void:
	_write_text(path, JSON.stringify(value))


func _write_text(path: String, text: String) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file != null:
		file.store_string(text)
