@echo off
rem Publishes a new build of the game: raises the build number, builds it,
rem signs it and puts it on the server with a note on what is new. Every
rem game updates itself to it the next time it is opened.
rem
rem What goes up depends on what changed since the published build:
rem   - only scripts, scenes, pictures or content: the pack (build\owl.pck);
rem   - the native library, the engine, the project settings or the loader:
rem     the pack, the library and the whole APK. Games fetch the library by
rem     themselves when only it changed; a changed shell needs the APK.
rem
rem Usage: release.bat "What is new in this build" [user@host] [full]
rem `full` publishes the whole game whatever changed.
rem Started by a double-click, it asks for the note and waits at the end.
rem The note is shown to players as it is. Do not use double quotes inside it.
rem
rem Needs the release key (%USERPROFILE%\.owl\release.key, made once with
rem `cargo run -p owl-release -- keygen`). It never leaves this PC.
setlocal
rem Lets this script find android.bat beside it even where the shell has
rem turned off running files from the current folder.
set NoDefaultCurrentDirectoryInExePath=
set NOTE=%~1
if "%NOTE%"=="" set /p NOTE=What is new in this build?
if "%NOTE%"=="" (
  echo Nothing published: a build needs a note on what is new.
  pause
  exit /b 1
)
set PI=%2
if "%PI%"=="" set PI=theo@rasppi.local
set FULL=
if /i "%~3"=="full" set FULL=1
cd /d "%~dp0"
set /p OLD=<client\build.txt
set /a BUILD=OLD+1
(echo %BUILD%)>client\build.txt

rem The library is built first: whether it changed decides what is published.
call "%~dp0android.bat" prepare || goto failed
if exist build\published.json del build\published.json
scp %PI%:owl/releases/version.json build\published.json >nul 2>&1
if defined FULL goto build
cargo run -q -p owl-release -- plan --base client\base.json --published build\published.json
if errorlevel 10 (
  set FULL=1
) else if errorlevel 1 goto failed

:build
rem The note is kept out of brackets: one in the note would end them early.
if defined FULL goto whole
call "%~dp0android.bat" pack || goto failed
cargo run -q -p owl-release -- publish --base client\base.json --notes "%NOTE%" --pack build\owl.pck --published build\published.json --out build\version.json || goto failed
goto upload
:whole
call "%~dp0android.bat" || goto failed
cargo run -q -p owl-release -- publish --base client\base.json --notes "%NOTE%" --pack build\owl.pck --lib client\lib\android\arm64\libowl_local.so --published build\published.json --out build\version.json || goto failed

:upload
rem Everything goes up under a spare name and is moved into place at the
rem end in one go, the note last: a game never finds half a build.
ssh %PI% "mkdir -p owl/releases && rm -f owl/releases/*.new" || goto failed
scp build\owl.pck %PI%:owl/releases/owl.pck.new || goto failed
if not defined FULL goto note
scp build\owl.apk %PI%:owl/releases/owl.apk.new || goto failed
scp client\lib\android\arm64\libowl_local.so %PI%:owl/releases/libowl_local.so.new || goto failed
:note
scp build\version.json %PI%:owl/releases/version.json.new || goto failed
ssh %PI% "cd owl/releases && for f in owl.apk libowl_local.so owl.pck version.json; do if [ -f $f.new ]; then mv -f $f.new $f || exit 1; fi; done" || goto failed
echo.
if defined FULL (
  echo Build %BUILD% is published: the pack, the library and the APK.
) else (
  echo Build %BUILD% is published: the pack only.
)
echo Commit client\build.txt so the number is kept.
pause
exit /b 0

:failed
(echo %OLD%)>client\build.txt
echo.
echo Nothing was published. client\build.txt is back at %OLD%.
pause
exit /b 1
