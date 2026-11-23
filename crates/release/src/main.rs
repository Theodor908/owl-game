//! The tool `release.bat` and `android.bat` call to describe, sign and
//! plan a build. See `docs/server.md`, "Publishing a new build".
//!
//! ```text
//! owl-release keygen   [--pub FILE]
//! owl-release describe --godot VERSION --client DIR --lib FILE --out FILE [--data DIR] [--platform NAME]
//! owl-release plan     --base FILE --published FILE
//! owl-release publish  --base FILE --notes TEXT --pack FILE --published FILE --out FILE [--lib FILE]
//! ```
//!
//! The private key is read from `OWL_RELEASE_KEY`, or from
//! `.owl/release.key` in the user's home folder. It is never printed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use owl_release::{Entry, FULL_RELEASE, PUBLIC_KEY, from_hex, public_key_of, sha256_hex, shell_id, to_hex};
use serde_json::{Value, json};

/// The files of the client that only a whole APK can change. Together with
/// the engine's version they make the shell id.
const SHELL_FILES: [&str; 4] = ["project.godot", "export_presets.cfg", "owl.gdextension", "scripts/boot.gd"];

/// The list of content files a build carries, written into its copy of
/// `data/`. The game writes exactly these out for the rules to read.
const CONTENT_LIST: &str = "files.txt";

/// The platform the published library is built for, as the game names its own.
const PLATFORM: &str = "android.arm64";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = arguments.split_first() else {
        eprintln!("Usage: owl-release keygen | describe | plan | publish (see crates/release/src/main.rs)");
        return ExitCode::FAILURE;
    };
    let options = options(rest);
    let result = match command.as_str() {
        "keygen" => keygen(&options),
        "describe" => describe(&options),
        "plan" => plan_command(&options),
        "publish" => publish_command(&options),
        other => Err(format!("unknown command `{other}`")),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("owl-release: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `--name value` pairs.
fn options(arguments: &[String]) -> HashMap<String, String> {
    arguments
        .chunks(2)
        .filter_map(|pair| Some((pair.first()?.strip_prefix("--")?.to_owned(), pair.get(1)?.clone())))
        .collect()
}

fn option<'a>(options: &'a HashMap<String, String>, name: &str) -> Result<&'a str, String> {
    options.get(name).map(String::as_str).ok_or_else(|| format!("--{name} is missing"))
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("{path}: {error}"))
}

fn read_json(path: &str) -> Result<Value, String> {
    serde_json::from_slice(&read(path)?).map_err(|error| format!("{path}: {error}"))
}

/// What is published now, or nothing when there is no such file or it cannot be read.
fn published(options: &HashMap<String, String>) -> Option<Value> {
    read_json(options.get("published")?).ok()
}

fn key_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("OWL_RELEASE_KEY") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"));
    home.map(|home| Path::new(&home).join(".owl").join("release.key")).map_err(|_| "no home folder to keep the key in".into())
}

fn secret_key() -> Result<[u8; 32], String> {
    let path = key_path()?;
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("the release key {} cannot be read ({error}); make one with `owl-release keygen`", path.display()))?;
    let secret = from_hex::<32>(text.trim()).ok_or_else(|| format!("{} is not a release key", path.display()))?;
    if public_key_of(&secret) != PUBLIC_KEY.trim() {
        return Err(format!(
            "the key in {} is not the one the game trusts (crates/release/release.pub); no phone would accept what it signs",
            path.display()
        ));
    }
    Ok(secret)
}

/// Makes the release key, once. The private half goes to the user's home
/// folder; the public half is printed and, with `--pub`, written to a file.
fn keygen(options: &HashMap<String, String>) -> Result<u8, String> {
    let path = key_path()?;
    if path.exists() {
        return Err(format!("{} exists already; a new key would lock out every installed game", path.display()));
    }
    let mut secret = [0; 32];
    getrandom::fill(&mut secret).map_err(|error| format!("no random numbers: {error}"))?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    std::fs::write(&path, format!("{}\n", to_hex(&secret))).map_err(|error| format!("{}: {error}", path.display()))?;
    let public = public_key_of(&secret);
    if let Some(file) = options.get("pub") {
        std::fs::write(file, format!("{public}\n")).map_err(|error| format!("{file}: {error}"))?;
    }
    println!("The private key is in {}. Keep a copy somewhere safe and never share it.", path.display());
    println!("Public key: {public}");
    Ok(0)
}

/// Writes what this build is to a file that goes into the game: its number,
/// its shell and its native library.
fn describe(options: &HashMap<String, String>) -> Result<u8, String> {
    let client = Path::new(option(options, "client")?);
    let mut files = Vec::new();
    for name in SHELL_FILES {
        files.push((name.to_owned(), read(&client.join(name).to_string_lossy())?));
    }
    let build_file = client.join("build.txt");
    let build: u64 = String::from_utf8_lossy(&read(&build_file.to_string_lossy())?)
        .trim()
        .parse()
        .map_err(|_| format!("{} holds no build number", build_file.display()))?;
    let base = json!({
        "build": build,
        "shell": shell_id(option(options, "godot")?, &files),
        "native": sha256_hex(&read(option(options, "lib")?)?),
        "platform": options.get("platform").map_or(PLATFORM, String::as_str),
    });
    if let Some(data) = options.get("data") {
        let mut names = Vec::new();
        list_files(Path::new(data), "", &mut names)?;
        names.retain(|name| name != CONTENT_LIST);
        names.sort();
        let list = Path::new(data).join(CONTENT_LIST);
        std::fs::write(&list, names.join("\n") + "\n").map_err(|error| format!("{}: {error}", list.display()))?;
    }
    let out = option(options, "out")?;
    std::fs::write(out, format!("{base}\n")).map_err(|error| format!("{out}: {error}"))?;
    println!("Build {build}: shell {}, native {}", base["shell"].as_str().unwrap_or(""), base["native"].as_str().unwrap_or(""));
    Ok(0)
}

/// Every file under `folder`, as paths from it with forward slashes.
fn list_files(folder: &Path, prefix: &str, names: &mut Vec<String>) -> Result<(), String> {
    let failed = |error: std::io::Error| format!("{}: {error}", folder.display());
    for entry in std::fs::read_dir(folder).map_err(failed)? {
        let entry = entry.map_err(failed)?;
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            list_files(&entry.path(), &format!("{name}/"), names)?;
        } else {
            names.push(name);
        }
    }
    Ok(())
}

/// Why this build cannot go out as a pack alone, or `None` when it can:
/// the published library and shell are the ones it was built with.
fn needs_full(base: &Value, published: Option<&Value>) -> Option<&'static str> {
    let Some(published) = published else { return Some("nothing readable is published yet") };
    if published.get("apk").is_none() {
        return Some("the published build has no pack loader");
    }
    if published["shell"] != base["shell"] {
        return Some("the engine, the project settings or the loader changed");
    }
    if published["lib"]["native"] != base["native"] || published["lib"]["shell"] != base["shell"] {
        return Some("the native library changed");
    }
    None
}

fn plan_command(options: &HashMap<String, String>) -> Result<u8, String> {
    let base = read_json(option(options, "base")?)?;
    match needs_full(&base, published(options).as_ref()) {
        Some(reason) => {
            println!("A full release: {reason}.");
            Ok(FULL_RELEASE as u8)
        }
        None => {
            println!("A pack is enough: the native library and the shell are the published ones.");
            Ok(0)
        }
    }
}

/// The `version.json` of a build. With `lib` it is a full release: the
/// library is signed and the APK entry describes this build. Without, the
/// library and the APK stay as published and only the pack is new.
fn version_json(
    base: &Value,
    notes: &str,
    pack: &[u8],
    lib: Option<&[u8]>,
    published: Option<&Value>,
    secret: &[u8; 32],
) -> Result<Value, String> {
    let field = |name: &str| base[name].as_str().ok_or_else(|| format!("the build's description has no `{name}`"));
    let (shell, native, platform) = (field("shell")?, field("native")?, field("platform")?);
    let build = base["build"].as_u64().ok_or("the build's description has no `build`")?;
    let pack = Entry::of("pack", build, shell, native, "", pack);
    let (lib, apk) = match lib {
        Some(lib) => {
            let entry = Entry::of("lib", build, shell, native, platform, lib);
            if entry.sha256 != native {
                return Err("the library is not the one this build was described with; build again".into());
            }
            (entry.to_json(&entry.sign(secret)), json!({ "build": build, "shell": shell, "native": native }))
        }
        None => {
            if let Some(reason) = needs_full(base, published) {
                return Err(format!("this build needs a full release: {reason}"));
            }
            let published = published.ok_or("nothing is published")?;
            (published["lib"].clone(), published["apk"].clone())
        }
    };
    Ok(json!({
        "build": build, "notes": notes, "shell": shell,
        "pack": pack.to_json(&pack.sign(secret)), "lib": lib, "apk": apk,
    }))
}

fn publish_command(options: &HashMap<String, String>) -> Result<u8, String> {
    let base = read_json(option(options, "base")?)?;
    let pack = read(option(options, "pack")?)?;
    let lib = options.get("lib").map(|path| read(path)).transpose()?;
    let version =
        version_json(&base, option(options, "notes")?, &pack, lib.as_deref(), published(options).as_ref(), &secret_key()?)?;
    let out = option(options, "out")?;
    std::fs::write(out, format!("{version}\n")).map_err(|error| format!("{out}: {error}"))?;
    println!("Signed build {} ({} bytes of pack{}).", version["build"], pack.len(), if lib.is_some() { ", with the library" } else { "" });
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: [u8; 32] = [7; 32];
    const LIB: &[u8] = b"the library";

    fn base(build: u64, shell: &str, lib: &[u8]) -> Value {
        json!({ "build": build, "shell": shell, "native": sha256_hex(lib), "platform": PLATFORM })
    }

    fn full(build: u64) -> Value {
        version_json(&base(build, "shell", LIB), "notes", b"pack", Some(LIB), None, &SECRET).expect("a full release")
    }

    #[test]
    fn the_first_release_and_one_with_no_loader_are_full() {
        assert!(needs_full(&base(3, "shell", LIB), None).is_some());
        assert!(needs_full(&base(3, "shell", LIB), Some(&json!({ "build": 2, "notes": "" }))).is_some());
    }

    #[test]
    fn a_pack_is_enough_until_the_library_or_the_shell_changes() {
        let published = full(3);
        assert_eq!(needs_full(&base(4, "shell", LIB), Some(&published)), None);
        assert!(needs_full(&base(4, "shell", b"another library"), Some(&published)).is_some());
        assert!(needs_full(&base(4, "another shell", LIB), Some(&published)).is_some());
    }

    #[test]
    fn a_pack_release_keeps_the_published_library_and_apk() {
        let published = full(3);
        let next = version_json(&base(4, "shell", LIB), "more", b"pack 4", None, Some(&published), &SECRET).unwrap();
        assert_eq!((next["build"].as_u64(), next["pack"]["build"].as_u64()), (Some(4), Some(4)));
        assert_eq!(next["lib"], published["lib"]);
        assert_eq!(next["apk"], json!({ "build": 3, "shell": "shell", "native": sha256_hex(LIB) }));
        assert_eq!(next["pack"]["native"], next["lib"]["native"], "the pack goes with the published library");
    }

    #[test]
    fn every_entry_is_signed() {
        let version = full(3);
        for kind in ["pack", "lib"] {
            let entry = Entry::from_json(&version[kind]).expect("an entry");
            assert_eq!(entry.kind, kind);
            assert!(entry.verify(&public_key_of(&SECRET), version[kind]["sig"].as_str().unwrap()));
        }
    }

    #[test]
    fn a_changed_library_is_never_published_as_a_pack_alone() {
        let published = full(3);
        let changed = base(4, "shell", b"another library");
        assert!(version_json(&changed, "", b"pack", None, Some(&published), &SECRET).is_err());
        assert!(version_json(&changed, "", b"pack", Some(LIB), Some(&published), &SECRET).is_err(), "the wrong library file");
    }
}
