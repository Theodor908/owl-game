# Versions and builds

Owl has two numbers.

- **The build number** (`client/build.txt`) goes up by one with every published build, whatever is in it. The game's updates go by it: a game updates when the server has a higher build. Players never choose it.
- **The version** (`client/scripts/version.gd`) is for people: `MAJOR.MINOR.PATCH` and a stage. It is shown in the corner of every screen ("Owl 0.1.0 alpha · build 12"), with every release note, and in `CHANGELOG.md`.

## Raising the version

`release.bat` asks each time, or reads `OWL_BUMP`:

| Answer | When | Example |
|---|---|---|
| keep | Nothing worth a new version: a quick fix to the last one | 0.1.0 stays |
| patch | Fixes, questions, pictures, sound | 0.1.0 → 0.1.1 |
| minor | New cards, rules or features, or a protocol change (older builds can no longer play online) | 0.1.3 → 0.2.0 |
| major | The finished game, or a change as large as a new game | 0.9.2 → 1.0.0 |

## Stages

The stage is changed by hand in `version.gd`; `release.bat` keeps it.

- **alpha** (now): rules and cards still change, accounts may be reset if needed, most questions are not yet reviewed, played among friends.
- **beta**: every rule of version 1 is in and settled (shop, battles, endgame, heroes); most questions are reviewed; no more resets. Mostly fixes, balance and content from here.
- **none** (1.0 and after): released.

## What each release leaves behind

`release.bat`, once a build is published:

1. adds it to the top of `CHANGELOG.md` with its note;
2. commits `build.txt`, `version.gd` and the changelog as "Build N, version X";
3. tags the commit `v<version>+<build>`, for example `v0.1.0-alpha+13`.

Pushing is left to you: `git push --follow-tags`. For the public copy, run `sh public.sh`, commit and push there, and if you like make a GitHub release from the tag with its changelog entry.
