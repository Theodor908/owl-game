@echo off
rem Builds the game for Android (arm64) into build\:
rem   owl.pck  the pack: scripts, scenes, pictures and content. A game
rem            downloads it by itself.
rem   owl.apk  the whole game, with Godot's release engine.
rem The native library (the rules) is built first and copied into the
rem project with the data files; client\base.json says which build, shell
rem and library this is.
rem
rem Usage: android.bat           the pack and the APK
rem        android.bat pack      the pack only
rem        android.bat prepare   the library, the data and base.json only
rem
rem Needs: the Android SDK and NDK, `rustup target add aarch64-linux-android`,
rem Godot's export templates, and the Android paths set in Godot's editor settings.
setlocal
set NDK_BIN=%LOCALAPPDATA%\Android\Sdk\ndk\28.2.13676358\toolchains\llvm\prebuilt\windows-x86_64\bin
set CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=%NDK_BIN%\aarch64-linux-android24-clang.cmd
set CC_aarch64_linux_android=%NDK_BIN%\aarch64-linux-android24-clang.cmd
set AR_aarch64_linux_android=%NDK_BIN%\llvm-ar.exe
set GODOT=%LOCALAPPDATA%\Microsoft\WinGet\Packages\GodotEngine.GodotEngine_Microsoft.Winget.Source_8wekyb3d8bbwe\Godot_v4.7.2-stable_win64.exe
rem Part of the shell id: a game takes a pack only from the engine it was built for.
set GODOT_VERSION=4.7.2-stable
rem An update installs only over a game signed with the same key. The key
rem that signs the APK is named in android.local.bat beside this file, which
rem is kept out of git: it sets GODOT_ANDROID_KEYSTORE_RELEASE_PATH, _USER and
rem _PASSWORD. Without it, set those three before calling.
if exist "%~dp0android.local.bat" call "%~dp0android.local.bat"
if "%GODOT_ANDROID_KEYSTORE_RELEASE_PATH%"=="" (
  echo No signing key: create android.local.bat setting GODOT_ANDROID_KEYSTORE_RELEASE_PATH, _USER and _PASSWORD.
  exit /b 1
)

cd /d "%~dp0"
cargo build -p owl-local --release --target aarch64-linux-android || exit /b 1
if not exist client\lib\android\arm64 mkdir client\lib\android\arm64
copy /y target\aarch64-linux-android\release\libowl_local.so client\lib\android\arm64\ >nul || exit /b 1
if exist client\data rmdir /s /q client\data
xcopy /e /i /q /y data client\data >nul || exit /b 1
rem The privacy notice goes into the game too, so it can be read offline.
copy /y docs\privacy.md client\data\privacy.md >nul || exit /b 1
cargo run -q -p owl-release -- describe --godot %GODOT_VERSION% --client client --lib client\lib\android\arm64\libowl_local.so --data client\data --out client\base.json || exit /b 1
if not exist build mkdir build
if /i "%~1"=="prepare" exit /b 0
"%GODOT%" --headless --path client --import >nul 2>&1
"%GODOT%" --headless --path client --export-pack "Android" ..\build\owl.pck || exit /b 1
echo Built build\owl.pck
if /i "%~1"=="pack" exit /b 0
"%GODOT%" --headless --path client --export-release "Android" ..\build\owl.apk || exit /b 1
echo Built build\owl.apk
