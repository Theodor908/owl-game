@echo off
rem Builds and starts the Owl server on this PC for testing on the home
rem network: it listens on every network address of the PC, port 8080.
rem
rem This is plain HTTP, with no encryption. Use it only on your own Wi-Fi,
rem with throwaway test accounts. For real use, see docs\server.md.
rem
rem The account database and the key file go in the "server-data" folder.
setlocal
cd /d "%~dp0"
cargo build --release -p owl-server || exit /b 1
if not exist server-data mkdir server-data
set OWL_BIND=0.0.0.0:8080
set OWL_DB=server-data\owl.db
set OWL_KEY=server-data\owl.key
echo.
echo In the game, under Profile, use one of these as the server address:
for /f "tokens=2 delims=:" %%a in ('ipconfig ^| findstr /c:"IPv4"') do echo    http://%%a:8080
echo (the spaces in the address must be left out)
echo.
echo Press Ctrl+C to stop the server.
target\release\owl-server.exe
