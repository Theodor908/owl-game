## The game's sound: one music track for the menus and one for matches,
## crossfaded when the player moves between them, and short effects on what
## happens (a button pressed, a card placed, a hit flying). Music and
## effects each have a volume, and both can be muted; the choice is kept on
## this device. Holds no rules.
class_name Sound
extends Node

static var instance: Sound

const FILE := "user://sound.cfg"
const MUSIC := {
	"menu": "res://assets/sound/music_menu.mp3",
	"game": "res://assets/sound/music_game.mp3",
	"battle": "res://assets/sound/music_battle.mp3",
}
const EFFECTS := {
	"button": "res://assets/sound/button.mp3",
	"place": "res://assets/sound/place.mp3",
	"hit": "res://assets/sound/hit.mp3",
	"tick": "res://assets/sound/tick.mp3",
	"reveal": "res://assets/sound/reveal.mp3",
	"wrong": "res://assets/sound/wrong.mp3",
	"correct": "res://assets/sound/correct.mp3",
	"golden": "res://assets/sound/golden.mp3",
	"victory": "res://assets/sound/victory.mp3",
	"defeat": "res://assets/sound/defeat.mp3",
}

## Seconds one track takes to fade into the other.
@export var music_fade := 1.5
## Loudness of each effect against the others, in decibels.
@export var effect_db := {
	"button": -8.0, "place": -2.0, "hit": -6.0, "tick": -4.0, "reveal": -6.0,
	"wrong": -4.0, "correct": -4.0, "golden": -2.0, "victory": -3.0, "defeat": -3.0,
}
## Effects played at their own pitch every time: the ticks keep time, and
## the stings are music.
const STEADY := ["tick", "victory", "defeat", "golden"]
## How far an effect's pitch may wander each time, so repeats do not sound copied.
@export var pitch_jitter := 0.06
## Effects that can sound at once; the oldest is cut when more come.
@export var voices := 8

var music_volume := 0.6
var effects_volume := 0.8
var muted := false

var _tracks: Array[AudioStreamPlayer] = []
var _playing := ""
var _fade: Tween
var _voices: Array[AudioStreamPlayer] = []
var _next_voice := 0
var _streams := {}


func _ready() -> void:
	instance = self
	process_mode = Node.PROCESS_MODE_ALWAYS
	for bus in ["Music", "Effects"]:
		if AudioServer.get_bus_index(bus) < 0:
			AudioServer.add_bus()
			AudioServer.set_bus_name(AudioServer.bus_count - 1, bus)
			AudioServer.set_bus_send(AudioServer.bus_count - 1, "Master")
	for index in 2:
		var track := AudioStreamPlayer.new()
		track.bus = "Music"
		track.volume_db = -80.0
		add_child(track)
		_tracks.append(track)
	for index in voices:
		var voice := AudioStreamPlayer.new()
		voice.bus = "Effects"
		add_child(voice)
		_voices.append(voice)
	_load()
	_apply()
	get_tree().node_added.connect(_on_node_added)


## Plays `track` ("menu", "game" or "battle"), fading out the one before. Nothing
## happens if it is already playing.
func music(track: String) -> void:
	if track == _playing or not MUSIC.has(track):
		return
	_playing = track
	var stream := _stream(MUSIC[track])
	if stream == null:
		return
	if stream is AudioStreamMP3:
		(stream as AudioStreamMP3).loop = true
	var old := _tracks[0]
	var new := _tracks[1]
	_tracks.reverse()
	new.stream = stream
	new.volume_db = -40.0
	new.play()
	if _fade != null:
		_fade.kill()
	_fade = create_tween().set_parallel()
	_fade.tween_property(new, "volume_db", 0.0, music_fade)
	if old.playing:
		_fade.tween_property(old, "volume_db", -60.0, music_fade)
		_fade.chain().tween_callback(old.stop)


## Plays effect `name` once, now, `quieter` decibels below its usual level.
func play(name: String, quieter := 0.0) -> void:
	var stream := _stream(EFFECTS.get(name, ""))
	if stream == null:
		return
	var voice := _voices[_next_voice]
	_next_voice = (_next_voice + 1) % _voices.size()
	voice.stream = stream
	voice.volume_db = float(effect_db.get(name, 0.0)) - quieter
	voice.pitch_scale = 1.0 if STEADY.has(name) else 1.0 + randf_range(-pitch_jitter, pitch_jitter)
	voice.play()


## Plays effect `name` after `seconds`.
func cue(name: String, seconds: float) -> void:
	if seconds <= 0.01:
		play(name)
	else:
		get_tree().create_timer(seconds).timeout.connect(play.bind(name))


func set_music_volume(value: float) -> void:
	music_volume = clampf(value, 0.0, 1.0)
	_apply()
	_save()


func set_effects_volume(value: float) -> void:
	effects_volume = clampf(value, 0.0, 1.0)
	_apply()
	_save()


func set_muted(value: bool) -> void:
	muted = value
	_apply()
	_save()


# Every button in the game clicks when pressed. Cards are buttons too, but
# placing one has its own sound, so a card's tap stays quiet.
func _on_node_added(node: Node) -> void:
	if node is BaseButton and not node is CardView:
		var click := play.bind("button")
		if not (node as BaseButton).pressed.is_connected(click):
			(node as BaseButton).pressed.connect(click)


func _stream(path: String) -> AudioStream:
	if path == "":
		return null
	if not _streams.has(path):
		_streams[path] = load(path) if ResourceLoader.exists(path) else null
	return _streams[path]


func _apply() -> void:
	var music_bus := AudioServer.get_bus_index("Music")
	var effects_bus := AudioServer.get_bus_index("Effects")
	AudioServer.set_bus_volume_db(music_bus, linear_to_db(maxf(music_volume, 0.0001)))
	AudioServer.set_bus_volume_db(effects_bus, linear_to_db(maxf(effects_volume, 0.0001)))
	AudioServer.set_bus_mute(music_bus, muted or music_volume <= 0.0)
	AudioServer.set_bus_mute(effects_bus, muted or effects_volume <= 0.0)


func _load() -> void:
	var config := ConfigFile.new()
	if config.load(FILE) != OK:
		return
	music_volume = float(config.get_value("sound", "music", music_volume))
	effects_volume = float(config.get_value("sound", "effects", effects_volume))
	muted = bool(config.get_value("sound", "muted", muted))


func _save() -> void:
	var config := ConfigFile.new()
	config.set_value("sound", "music", music_volume)
	config.set_value("sound", "effects", effects_volume)
	config.set_value("sound", "muted", muted)
	config.save(FILE)
