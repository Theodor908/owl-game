@echo off
rem Builds the Rust extension and starts the game in a landscape phone-shaped window.
rem Extra arguments are passed to the game, e.g.: play.bat --seed=3 --speed=4
setlocal
cd /d "%~dp0"
cargo build -p owl-local || exit /b 1
if not exist client\data mkdir client\data
copy /y docs\privacy.md client\data\privacy.md >nul || exit /b 1
set "GODOT=%LOCALAPPDATA%\Microsoft\WinGet\Packages\GodotEngine.GodotEngine_Microsoft.Winget.Source_8wekyb3d8bbwe\Godot_v4.7.2-stable_win64.exe"
if not exist "%GODOT%" (
  echo Godot was not found at "%GODOT%". Install it with: winget install GodotEngine.GodotEngine
  exit /b 1
)
"%GODOT%" --path client --resolution 1280x720 -- %*
