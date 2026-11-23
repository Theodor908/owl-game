## The game's version, for people: MAJOR.MINOR.PATCH and a stage (alpha,
## beta, or none for a release). `release.bat` raises it when asked; a stage
## is changed here by hand. The update system goes by the build number in
## `res://build.txt`, not by this. See docs/versioning.md.
class_name Version

const NAME := "0.1.1-alpha"


## The version as shown: "0.1.0 alpha".
static func shown() -> String:
	return NAME.replace("-", " ")
