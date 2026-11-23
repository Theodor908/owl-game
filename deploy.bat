@echo off
rem Sends the server to the Raspberry Pi and (re)starts it there with Docker.
rem Usage: deploy.bat [user@host]      default: theo@rasppi.local
rem
rem Needs Docker on the Pi, and on the Pi a file ~/owl/deploy/.env holding
rem the Cloudflare tunnel token (see docs\server.md). You are asked for the
rem Pi's password twice: once to copy, once to build and start.
setlocal
set PI=%1
if "%PI%"=="" set PI=theo@rasppi.local
cd /d "%~dp0"
tar -cf owl-deploy.tar Cargo.toml Cargo.lock crates data docs/privacy.md deploy/Dockerfile deploy/docker-compose.yml deploy/.env.example .dockerignore || exit /b 1
scp owl-deploy.tar %PI%:owl-deploy.tar || exit /b 1
del owl-deploy.tar
ssh -t %PI% "mkdir -p owl && tar -xf owl-deploy.tar -C owl && rm owl-deploy.tar && cd owl && docker compose -f deploy/docker-compose.yml up -d --build && docker compose -f deploy/docker-compose.yml ps"
