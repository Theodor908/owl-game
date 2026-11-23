# The game's version, for release.bat. See docs/versioning.md.
#   version.ps1 -Bump keep|patch|minor|major   raises the version in
#       client/scripts/version.gd and prints it as "0.1.0-alpha".
#   version.ps1 -Changelog -Build N -Notes TEXT   adds the release to the
#       top of CHANGELOG.md.
# A stage (alpha, beta) is changed by hand in version.gd; raising the major
# number keeps the stage too.
param(
    [string]$Bump = "",
    [switch]$Changelog,
    [int]$Build = 0,
    [string]$Notes = ""
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$file = Join-Path $root "client/scripts/version.gd"
$text = [IO.File]::ReadAllText($file)
if ($text -notmatch 'const NAME := "(\d+)\.(\d+)\.(\d+)(-[a-z]+)?"') {
    throw "No version found in $file"
}
$major = [int]$Matches[1]; $minor = [int]$Matches[2]; $patch = [int]$Matches[3]; $stage = $Matches[4]

if ($Bump -ne "") {
    switch ($Bump) {
        "keep" { }
        "patch" { $patch++ }
        "minor" { $minor++; $patch = 0 }
        "major" { $major++; $minor = 0; $patch = 0 }
        default { throw "Bump is keep, patch, minor or major, not '$Bump'" }
    }
    $name = "$major.$minor.$patch$stage"
    $text = $text -replace 'const NAME := "[^"]*"', "const NAME := `"$name`""
    [IO.File]::WriteAllText($file, $text)
    Write-Output $name
    exit 0
}

if ($Changelog) {
    $name = "$major.$minor.$patch$stage"
    $log = Join-Path $root "CHANGELOG.md"
    $head = "# Changelog`n`nEvery published build, newest first. The version is for people; the build number is what the game's updates go by.`n"
    $body = ""
    if (Test-Path $log) {
        $old = [IO.File]::ReadAllText($log)
        $at = $old.IndexOf("`n## ")
        if ($at -ge 0) { $body = $old.Substring($at + 1) }
    }
    $date = Get-Date -Format "yyyy-MM-dd"
    $entry = "## $($name.Replace('-', ' ')), build $Build ($date)`n`n$Notes`n`n"
    [IO.File]::WriteAllText($log, "$head`n$entry$body")
    exit 0
}

Write-Output "$major.$minor.$patch$stage"
