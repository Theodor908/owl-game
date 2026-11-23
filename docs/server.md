# Running the Owl server

The server (`crates/server`, package `owl-server`) holds accounts and friends in one SQLite file and hosts lobbies and matches over a WebSocket. It is one small program with no other services to install. This page covers running it on a Raspberry Pi.

Nothing here has been run on a Pi yet. The server is tested on the development machine only.

## The short way: Docker and a Cloudflare domain

This is the recommended setup for a Pi at home with a domain managed by Cloudflare. Two containers run side by side: the server, and Cloudflare's tunnel client. Nothing is opened on the router, the home address stays private, and Cloudflare provides HTTPS. The files are in `deploy/`.

Not yet run on a Pi: the image build and the tunnel are untested.

### Once, in Cloudflare

1. Open Zero Trust, then Networks, Tunnels. Create a tunnel of type Cloudflared and name it `owl`.
2. Copy the token it shows (the long string after `--token`).
3. Add a public hostname to the tunnel: a subdomain such as `owl`, your domain, type `HTTP`, URL `owl:8080`. That is the server's name and port inside Docker.

### Once, on the Pi

```sh
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker $USER      # then log out and in again
mkdir -p ~/owl/deploy
echo "TUNNEL_TOKEN=<the token>" > ~/owl/deploy/.env
chmod 600 ~/owl/deploy/.env
```

### Getting the server onto the Pi

There are two ways. The first is the normal one.

**A. Through GitHub (automatic).** Every push to `master` runs the tests; if they pass and the server changed, GitHub builds the image for the Pi and stores it in the repository's private registry (`.github/workflows/server.yml`). The Pi checks for a new image every few minutes and restarts the server when there is one. GitHub never connects to the Pi.

Set the Pi up for it once:

1. On GitHub, make a token the Pi can read images with: Settings, Developer settings, Personal access tokens, Tokens (classic), with only the `read:packages` scope.
2. On the PC, copy the `deploy` folder to the Pi: `scp -r deploy theo@rasppi.local:owl/`
3. On the Pi:

   ```sh
   docker login ghcr.io -u Theodor908        # paste the token as the password
   chmod +x ~/owl/deploy/update.sh
   ~/owl/deploy/update.sh                    # first start
   (crontab -l 2>/dev/null; echo "*/5 * * * * $HOME/owl/deploy/update.sh >/dev/null 2>&1") | crontab -
   ```

From then on, deploying is `git push`.

**B. Straight from the PC (no GitHub).** Run `deploy.bat`. It copies the server's sources and the content to the Pi, builds the image there and starts both containers. The first build takes a while on a Pi (compiling Rust). To send it somewhere other than `theo@rasppi.local`: `deploy.bat user@host`.

Check it from anywhere: `https://owl.<your domain>/api/privacy` should show the privacy notice.

`https://owl.<your domain>/` is the download page: the icon, a Download button, the published build number, the size of the APK, how to install it, screenshots and the release note. The server builds it on every request from `version.json` and `owl.apk` in the releases folder, so it needs no step when a build is published; with nothing published it says "Coming soon". The page itself is sent with `Cache-Control: no-cache`. Its pictures and font are compiled into the server (`crates/server/assets/`, served under `/site/`) and may be kept for a day. It uses no cookies and no scripts, loads nothing from other sites, and asks search engines not to list it.

### Publishing a new build of the game

A game updates itself when it is opened. Most builds reach players without Android asking to install anything.

On the PC:

```
release.bat "What is new in this build"
```

It raises the build number (`client/build.txt`), builds the game, signs it and copies it with the note to `~/owl/releases/` on the Pi. Commit `client/build.txt` afterwards, so the number is not lost.

A build has up to three files, and `release.bat` works out which are needed by comparing this build with the published `version.json`:

| What changed | What is published | What a player sees |
|---|---|---|
| Scripts, scenes, pictures, content (`data/`) | `owl.pck` (about 40 MB) | The game downloads it and starts again. |
| The rules (anything in the Rust crates the game uses) | `owl.pck`, `libowl_local.so` (about 7 MB), `owl.apk` | The game downloads the pack and the library and starts again. |
| The shell: Godot's version, `client/project.godot`, `client/export_presets.cfg`, `client/owl.gdextension`, `client/scripts/boot.gd`, the release key | all three | The game shows a Download button; Android asks to install the APK. |

The APK is published with every changed library too, although players are not asked to install it: it is what a new player installs, and what a phone falls back to if it will not load a downloaded library. `release.bat "note" theo@rasppi.local full` publishes all three whatever changed.

#### What the game does on start

Before the menus open, the game asks the server which build is newest.

- **A newer pack or library that fits:** an "Updating Owl" screen with a progress bar. The game downloads the files, checks them, and starts again into the new build. There is no way past this screen. If a download fails or does not check out, the screen shows "Retry".
- **A newer build that needs the APK:** only a screen with a Download button. Nothing else in the game can be reached until the new APK is installed. A phone installs it over the game it has; accounts and settings stay.
- **No answer within 4 seconds** (no internet, or the server is down): the game opens with the build it has. Playing offline against bots always works.
- **A build this device refused** (it failed to start twice, or its files were damaged) with no newer APK on the server: the game opens with the build it has, until the next build is published.

Runs from the Godot editor and runs with any development switch (`--autoplay`, `--skip-menu`, ...) skip all of this. `--check-updates` turns it on for such a run, and `--updates` lets a run from the editor apply downloads.

#### How a download is trusted

Every pack and library is signed with an Ed25519 key that exists only on the publishing PC: `%USERPROFILE%\.owl\release.key` (or the file named by `OWL_RELEASE_KEY`). The public half is `crates/release/release.pub` and is compiled into the game's native library. On every start the loader (`client/scripts/boot.gd`, the first autoload) checks each downloaded file's size, SHA-256 and signature before using it, with the copy of the library inside the installed APK. Neither the loader nor that copy can be replaced by a download. A file that fails is deleted and the installed build runs. The server, or anyone between it and the phone, cannot make a game run code that was not signed on the PC.

- **Keep a copy of the key somewhere safe and offline.** Without it no build can be published to the games already installed; the only way on would be a new key, a new APK, and every player installing it by hand.
- **Never commit it or copy it to the Pi.** `.gitignore` lists `release.key` as a safety net.
- A new key is made with `cargo run -p owl-release -- keygen --pub crates/release/release.pub`. It refuses to overwrite an existing one.

#### The APK's own signature

Android installs an update only over a game signed with the same key. Build 2 went out signed with Android's debug key (`%USERPROFILE%\.android\debug.keystore`), so `android.bat` signs every APK with it, now on Godot's release engine. Keep that file: with another one, phones refuse the update and the game has to be removed and installed afresh. To move to a key of your own, set `GODOT_ANDROID_KEYSTORE_RELEASE_PATH`, `_USER` and `_PASSWORD` before calling `android.bat`; that too means installing afresh once.

#### Things to know

- A game older than build 3 has no loader. It shows its old "A new version of Owl is out" sheet and the player installs the APK from there, once.
- The pack is whole, not a difference: every pack-only build is a download of about 40 MB, also on mobile data.
- A downloaded library has not been tried on a phone yet. If Android or Godot refuses to load a library from the game's own storage, the loader goes back to the installed one by itself and the game asks for the APK instead; publishing stays the same.
- A change to the autoload list or other project settings is a change to the shell, since those are read before any pack is loaded. `release.bat` sees it and publishes the APK.

### Looking after it

```sh
cd ~/owl
docker compose -f deploy/docker-compose.yml ps        # are both running?
docker compose -f deploy/docker-compose.yml logs -f   # what are they saying?
docker compose -f deploy/docker-compose.yml down      # stop (accounts are kept)
```

The accounts live in the Docker volume `owl-db` and the key file in `owl-key`. They survive rebuilds and restarts. `docker compose down -v` deletes both, and with them every account.

Back up the key once, to somewhere offline, and the database on a schedule, to somewhere else:

```sh
docker compose -f deploy/docker-compose.yml cp owl:/etc/owl/owl.key ./owl.key        # once; then move it off the Pi
docker compose -f deploy/docker-compose.yml cp owl:/var/lib/owl/owl.db ./owl-backup.db
```

Copy the database only while the server is stopped, or accept that a copy taken mid-write may be incomplete.

Because Cloudflare ends the encryption and passes the traffic on, it can see it. `docs/privacy.md` names it for that reason.

The rest of this page describes running the server without Docker, and the details that apply either way.

## What it needs

- The game's `data/` directory (cards, heroes, numbers) and `docs/privacy.md`.
- A place for two files it creates: the database and the key file.
- Something in front of it that provides HTTPS. The server itself speaks plain HTTP and by default listens only on the Pi itself (`127.0.0.1:8080`). **Never expose that port directly**: passwords and session tokens would cross the network unencrypted.

## Build

On the Pi (64-bit Raspberry Pi OS), with Rust installed from rustup.rs:

```sh
sudo apt install build-essential
cargo build --release -p owl-server
```

The program is `target/release/owl-server`. SQLite is compiled in.

## Settings

All by environment variable; every one has a default.

| Variable | Default | Meaning |
|---|---|---|
| `OWL_BIND` | `127.0.0.1:8080` | Address to listen on. Keep it local. |
| `OWL_DATA` | `data` | The content directory. |
| `OWL_DB` | `owl.db` | The account database. |
| `OWL_KEY` | `owl.key` | The key file, created on first start. |
| `OWL_PRIVACY` | `docs/privacy.md` | The privacy notice shown before registering. |
| `OWL_RELEASES` | `releases` | The published build: `version.json`, `owl.apk`, `owl.pck`, `libowl_local.so`. It may be missing. |
| `OWL_TRUST_CF_IP` | off | `1` when the server sits behind a Cloudflare tunnel. The limits then count a request by the address in its `CF-Connecting-IP` header. Without it they count by the address of the connection. |

`deploy/docker-compose.yml` sets `OWL_TRUST_CF_IP=1`. Set it only when nothing but the tunnel can reach the server; anyone else could write any address in that header. Leave it off for `server.bat` and on a LAN, where the connection's address is the player's own. Behind the tunnel without it, every player counts as the one tunnel address and shares one allowance.

## Limits

Anyone can ask for a password hash or a new player without being logged in, and each costs the Pi memory, a core or a row on disk. These limits keep one client from using the server up. The numbers are constants at the top of `crates/server/src/limits.rs`.

| What | Limit | Answer when reached |
|---|---|---|
| Wrong passwords for one name (any case, whether or not the name exists) | 3 free, then each try waits 1 second, doubling up to 5 minutes. Forgotten 30 minutes after the last one. Logging in, changing the password or the name, and deleting the account all count. | 429 |
| Password checks from one address | 20 per 10 minutes | 429 |
| New accounts and guests from one address | 5 per hour, shared | 429 |
| New accounts and guests on the whole server | 60 per hour | 429 |
| Password hashes at once | 2, with 8 more waiting in turn | 503 at once when 8 are waiting |
| Game connections open at once | 10 per address, 300 in all | 429 before the WebSocket opens |

- A 429 or 503 carries `Retry-After` in seconds and a plain message, such as "too many wrong passwords; try again in 8 seconds".
- An IPv6 address counts by its first 64 bits, the part one home or phone network gets.
- The waits for a name grow but never lock it, so nobody can lock a friend out for long by guessing. The same answers come back for a name that does not exist.
- Everything is kept in memory and lost on restart. Each map has a largest size (10,000 names or addresses). When one is full, entries that have run out go first, then the one with the fewest tries.
- Client addresses are kept only in memory, only as long as the window they count in, and are never written to disk or to a log (`docs/privacy.md`).
- `Hello` on the WebSocket checks only a session token, a SHA-256 and one database row, so it needs no hash turn.

## The key file

On first start the server writes 32 random bytes to `OWL_KEY`, readable by its user only. Every secret is derived from it: the pepper mixed into password hashes, the key that encrypts usernames, and the key of the username index.

- **Lose it and every account is lost.** No password verifies and no username decrypts. Keep one copy offline (a USB stick in a drawer).
- **Never store it with the database or in the same backup.** The encryption only protects a copy of the database that travels without the key.
- Keep it out of git.

What this protects against: someone getting a copy of the database file or a backup. What it does not protect against: someone who controls the Pi, since the key is on it.

## Reaching it from outside

Pick one. The first is the simplest for a game among friends.

1. **Tailscale Funnel.** Gives a public `https://` address with no port forwarding and no domain name. Install Tailscale on the Pi, then: `tailscale funnel 8080`. Players need nothing installed.
2. **Cloudflare Tunnel.** Also no open port. Cloudflare ends the encryption and can see the traffic, so name it in the privacy notice.
3. **Caddy with your own domain.** Needs a domain (or dynamic DNS) and ports 80 and 443 forwarded on the router, and it shows your home address to players. Caddyfile:

   ```
   owl.example.org {
       reverse_proxy 127.0.0.1:8080
   }
   ```

Whichever you use, name it in `docs/privacy.md` where the notice asks for it. The game then connects to `https://<address>` and `wss://<address>/ws`.

## Running it as a service

Create a user and a directory, copy the program, `data/` and `docs/privacy.md` to `/opt/owl`, then `/etc/systemd/system/owl.service`:

```ini
[Unit]
Description=Owl game server
After=network-online.target
Wants=network-online.target

[Service]
User=owl
WorkingDirectory=/opt/owl
ExecStart=/opt/owl/owl-server
Environment=OWL_DB=/var/lib/owl/owl.db
Environment=OWL_KEY=/etc/owl/owl.key
StateDirectory=owl
Restart=on-failure

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ReadWritePaths=/etc/owl
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
CapabilityBoundingSet=
SystemCallFilter=@system-service

[Install]
WantedBy=multi-user.target
```

```sh
sudo useradd --system --home /opt/owl owl
sudo mkdir -p /etc/owl && sudo chown owl: /etc/owl && sudo chmod 700 /etc/owl
sudo systemctl enable --now owl
systemd-analyze security owl
```

The database lives in `/var/lib/owl`, the key in `/etc/owl`: two different places, so a backup of one does not carry the other.

## Backups

Never copy the live database file; it can be caught mid-write. Make a consistent copy with SQLite itself:

```sh
sqlite3 /var/lib/owl/owl.db "VACUUM INTO '/var/backups/owl/owl-$(date +%F).db'"
find /var/backups/owl -name 'owl-*.db' -mtime +14 -delete
```

The second line deletes backups older than 14 days. Put that number in the privacy notice: it is how long a deleted account can still exist in a backup.

## Password hashing cost

Passwords use Argon2id with 19 MiB of memory, 2 passes and 1 lane, the minimum OWASP recommends. Two hashes run at a time and a few more wait their turn (see Limits), so many logins at once queue instead of exhausting the Pi's memory and cores. Time one login on the Pi; if it takes more than half a second, it is still safe, only slow.

## What you still have to do yourself

Software cannot do these:

- Fill in your name, a contact and the tunnel service in `docs/privacy.md`.
- Keep the Pi updated.
- Keep the offline copy of the key file.
- Answer a player who asks about their data within a month (`docs/gdpr.md`).
- Decide what to do if the Pi is broken into (`docs/gdpr.md`).
