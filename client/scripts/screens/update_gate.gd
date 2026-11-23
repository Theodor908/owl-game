## The screen before the menus: the game brings itself up to the newest
## build here, and nothing else can be reached until it has. It shows the
## download with a bar, "Retry" when a download fails, and the way to the
## APK when the whole game must be installed again. When there is nothing
## new, or the server does not answer, it steps aside at once.
## Holds no rules; `scripts/updater.gd` does the work.
extends Control

## The game may open: it is up to date, or the server cannot be reached.
signal done

const Updater := preload("res://scripts/updater.gd")

@export var sheet_width := 900
@export var title_size := 56
@export var text_size := 30
@export var bar_height := 34
## How long the note that the game starts again stays up, in seconds.
@export var restart_after := 1.2

## What the screen shows now: "looking", "downloading", "failed", "restarting" or "apk".
var state := "looking"
var _updater: Node
var _stack: VBoxContainer
var _bar: ProgressBar


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	var centre := CenterContainer.new()
	centre.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(centre)
	var sheet := UI.panel(UI.INK_RAISED, 16, 30)
	sheet.custom_minimum_size.x = sheet_width
	centre.add_child(sheet)
	_stack = UI.column(18)
	sheet.add_child(_stack)
	_say("looking", "Looking for updates", "", "")

	_updater = Updater.new()
	add_child(_updater)
	_updater.nothing_new.connect(func() -> void: done.emit())
	_updater.downloading.connect(_downloading)
	_updater.failed.connect(_failed)
	_updater.ready_to_restart.connect(_restart)
	_updater.apk_needed.connect(_apk_needed)
	_updater.check()


## Replaces what the sheet says: a title, a line under it, and a longer text.
func _say(now: String, title: String, line: String, text: String) -> void:
	state = now
	for child in _stack.get_children():
		_stack.remove_child(child)
		child.queue_free()
	_bar = null
	_stack.add_child(UI.centered(title, title_size, UI.PAPER))
	if line != "":
		_stack.add_child(UI.centered(line, text_size, UI.SLATE))
	if text.strip_edges() != "":
		_stack.add_child(UI.label(text, text_size + 2, Color("e6eaf2")))


func _downloading(build: int, notes: String, share: float) -> void:
	if _bar == null:
		_say("downloading", "Updating Owl", "Build %d is on its way. The game starts by itself when it is here." % build, notes)
		_bar = ProgressBar.new()
		_bar.custom_minimum_size.y = bar_height
		_bar.max_value = 100.0
		_bar.add_theme_font_size_override("font_size", text_size - 6)
		_bar.add_theme_stylebox_override("background", UI.box(UI.INK, 14, 0, Color("b9c6dc"), 2))
		_bar.add_theme_stylebox_override("fill", UI.box(UI.VERDIGRIS, 14, 0))
		_stack.add_child(_bar)
	_bar.value = share * 100.0


func _failed(build: int) -> void:
	_say("failed", "The update did not arrive", "Build %d could not be downloaded whole." % build,
		"Owl needs it before you can play. Check your connection and try again.")
	_stack.add_child(UI.stamp("Retry", func() -> void:
		_say("looking", "Looking for updates", "", "")
		_updater.check(), Vector2(0, 110), 40, UI.VERDIGRIS))


## The new build is on the device: the game starts again into it. A phone
## that does not start the game again by itself closes it; opening it
## again gives the new build.
func _restart(build: int) -> void:
	_say("restarting", "Owl is up to date", "Build %d is here. Owl starts again now." % build,
		"If it only closes, open it again.")
	await get_tree().create_timer(restart_after).timeout
	OS.set_restart_on_exit(true, OS.get_cmdline_args())
	get_tree().quit()


func _apk_needed(build: int, notes: String) -> void:
	_say("apk", "A new version of Owl is out", "You have build %d; build %d is needed to play." % [Online.build(), build], notes)
	_stack.add_child(UI.stamp("Download", func() -> void:
		OS.shell_open(Online.download_address("owl.apk", build)), Vector2(0, 110), 40, UI.VERDIGRIS))
	_stack.add_child(UI.label("Your phone downloads the game and asks to install it over this one. Nothing you have is lost.", 26, UI.SLATE))
