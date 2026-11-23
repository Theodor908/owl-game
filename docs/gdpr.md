# Owl and the GDPR

This is a working note for the person who runs the server. It is not legal advice.

## Does the GDPR apply?

A private person running a game purely for personal friends, with no commercial side, plausibly falls under the "household exemption" (Article 2(2)(c)) and outside the GDPR. That stops being true once registration is open to the public or the game earns money. The server is built as if the GDPR applies, so nothing has to change on the day it does.

A username is personal data: it identifies a person to their friends. So are friend links and the dates kept with them.

## What the software does

| Obligation | How it is met |
|---|---|
| Keep as little as possible (Art. 5) | Username, password hash, friend links, a few dates. No e-mail, no analytics. Matches are not stored. A client's internet address is held in memory only, for at most an hour, to enforce the limits (`crates/server/src/limits.rs`, legitimate interest, Art. 6(1)(f)); it is never written to disk or to a log and is not linked to an account. |
| Tell people what is kept (Art. 13) | `docs/privacy.md`, shown before registering (`GET /api/privacy`). Registration is refused until the player confirms they read it. |
| Lawful basis (Art. 6) | Contract: the data is needed to provide the account. It is not consent, so there is no consent to withdraw; deleting the account ends it. |
| Access and portability (Art. 15, 20) | `GET /api/me/export` returns everything held, as JSON. |
| Correction (Art. 16) | `POST /api/me/username` and `POST /api/me/password`. |
| Erasure (Art. 17) | `DELETE /api/me` removes the account, its sessions and its friend links at once. Tested. |
| Security (Art. 32) | Passwords hashed one way (Argon2id, salted, peppered); usernames encrypted at rest; session tokens stored only as hashes; growing delays after wrong passwords; limits per internet address and on password hashes at once; HTTPS in front (`docs/server.md`). |
| Limited retention (Art. 5) | Sessions end after 30 days, or 14 days unused. An account unused for a year is erased (`store::INACTIVE_SECONDS`, swept every 6 hours). No backups are made, so nothing outlives a deleted account. |

## What remains yours

- **The notice.** Fill in the blanks in `docs/privacy.md` and change its date when you change it.
- **Requests.** A player may ask about their data in person. Answer within one month (Art. 12). The in-game export and delete cover most of it.
- **Lost passwords.** There is no e-mail to reset by. The only remedy is to delete the account so it can be made again; do it only when you are sure who is asking.
- **Children.** The notice says 16 or over, which is the age in Romania below which a parent would have to agree if consent were the basis. Nothing checks it.
- **Backups.** None are made at present. If you start making them, encrypt them or keep them on the Pi, delete old ones on a schedule, and say in the notice how long they are kept.
- **Updates.** Keep the Pi and the server current.

## Record of processing (Art. 30)

Keep this page as the record.

- **Controller:** the private person who runs the server for their friends. The public notice does not name them, because it is readable by anyone on the internet; players know them personally. If the game is ever offered beyond friends, the notice needs a name and a contact (Art. 13).
- **Purpose:** accounts and friend lists for a private trivia game.
- **People concerned:** players, aged 16 or over.
- **Data:** username; password hash; friend links; account and notice dates; session token hashes with dates.
- **Recipients:** none, or the tunnel service named in the notice.
- **Transfers outside the EU:** none, unless the tunnel service makes one.
- **Retention:** until the account is deleted, or a year after it was last used; sessions at most 30 days; no backups.
- **Security:** as in the table above.

## If the server is broken into (Art. 33, 34)

1. Take the server offline.
2. Write down what happened, when you noticed, and what could have been read. Keep that note whatever you decide next.
3. If there is a risk to players, tell ANSPDCP within 72 hours of noticing (dataprotection.ro). A stolen database without the key file is low risk: usernames are encrypted and passwords are hashed with a pepper that is not in it. A stolen database with the key file is not low risk.
4. Tell the players. There are no e-mail addresses, so tell them in person and in the game.
5. Make a new key file and have everyone register again. A new key invalidates every stored password and username on purpose.

## Sources

Checked on 18 November 2026:

- OWASP Password Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- OWASP Session Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
- OWASP WebSocket Security Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/WebSocket_Security_Cheat_Sheet.html
- NIST SP 800-63B-4 (password length): https://pages.nist.gov/800-63-4/sp800-63b.html
- CipherSweet (encrypted fields with a blind index): https://ciphersweet.paragonie.com/security
- Irish Data Protection Commission on the household exemption: https://dataprotection.ie/en/faqs/general/what-household-exemption
- GDPR Article 30: https://gdpr-info.eu/art-30-gdpr/
- Tailscale Funnel: https://tailscale.com/kb/1223/funnel
- SQLite write-ahead logging: https://www.sqlite.org/wal.html
