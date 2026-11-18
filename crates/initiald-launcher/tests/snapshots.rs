//! Golden snapshots of the command line (rea-launcher-sdk
//! docs/STANDARDS.md, "Golden snapshots"): `--help`, and every mode run on
//! a synthetic library (empty files and CHD headers carrying the known
//! digests, never game files), with the files it writes. A diff is reviewed
//! like code: it must match an intended change or an SDK Migration item.
//!
//! `REA_UPDATE_SNAPSHOTS=1 cargo test --test snapshots` rewrites them.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The launchers the tests start take the same instance on Windows.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A temporary data folder, components folder and library.
struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Fixture {
        let f = Fixture {
            root: tempfile::tempdir().unwrap(),
        };
        for dir in ["data", "components", "bin"] {
            fs::create_dir_all(f.path(dir)).unwrap();
        }
        f
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.path().join(rel)
    }

    fn arg(&self, rel: &str) -> String {
        self.path(rel).display().to_string()
    }

    fn write(&self, rel: &str, bytes: &[u8]) {
        let p = self.path(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    /// `text` with this fixture's folder as `<tmp>` and `/` between names.
    fn clean(&self, text: &str) -> String {
        text.replace(&self.root.path().display().to_string(), "<tmp>")
            .replace('\\', "/")
    }

    /// Runs the launcher with `args`; appends what it did to `out`.
    fn run(&self, out: &mut String, args: &[&str]) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_initiald-launcher"));
        cmd.args(args)
            .env("INITIALD_DATA", self.path("data"))
            .env("INITIALD_COMPONENTS", self.path("components"))
            // the single-instance lock of this fixture's launcher (Linux)
            .env("XDG_RUNTIME_DIR", self.path("bin"))
            // headless: a refusal is never shown in a message window
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY");
        // Windows: a command's refusal (--list, --check) is a dialog for a
        // player without a terminal, which would wait forever under Wine;
        // `--linux` says the output is read (the SDK template's rule). A
        // --play refusal is never a dialog (a run without the window).
        if cfg!(windows) && args.iter().any(|a| matches!(*a, "--list" | "--check")) {
            cmd.arg("--linux");
        }
        // Windows: one launcher at a time per user session, whatever the fixture
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        // no git, docker or flycast: what the launcher says without them
        if cfg!(unix) {
            cmd.env("PATH", self.path("bin"));
        }
        let o = cmd.output().expect("run the launcher");
        let _ = writeln!(
            out,
            "$ initiald-launcher {}\nexit {}\n-- stdout\n{}-- stderr\n{}",
            self.clean(&args.join(" ")),
            o.status.code().unwrap_or(-1),
            self.clean(&String::from_utf8_lossy(&o.stdout)),
            without_usage(&self.clean(&String::from_utf8_lossy(&o.stderr))),
        );
    }

    /// Appends a file of the data folder to `out`.
    fn show(&self, out: &mut String, rel: &str) {
        let text = fs::read_to_string(self.path("data").join(rel))
            .map(|t| self.clean(&t))
            .unwrap_or_else(|_| "(no such file)\n".into());
        let _ = write!(out, "-- data/{rel}\n{text}");
    }
}

/// `stderr` with the usage text that follows an error shortened (help.txt
/// holds it).
fn without_usage(stderr: &str) -> String {
    match stderr.split_once("\n\ninitiald-launcher —") {
        Some((error, _)) => format!("{error}\n<usage text>\n"),
        None => stderr.to_string(),
    }
}

/// A CHD v5 header recording `sha1` for the disc: what the library reads.
fn fake_chd(sha1: &str) -> Vec<u8> {
    let mut h = vec![0u8; 124];
    h[..8].copy_from_slice(b"MComprHD");
    h[12..16].copy_from_slice(&5u32.to_be_bytes());
    for (i, byte) in h[84..104].iter_mut().enumerate() {
        *byte = u8::from_str_radix(&sha1[i * 2..i * 2 + 2], 16).unwrap();
    }
    h
}

/// A library: a NAOMI 2 folder with two known discs (one without its
/// cartridge key), a TeknoParrot-style folder with an unknown build and
/// one with a file that is not a CHD.
fn library(f: &Fixture) {
    f.write(
        "lib/naomi2/initdexp/gds-0025a.chd",
        &fake_chd("d6ec4295122e3d69b9e109778ab1cb0cb0dfc839"),
    );
    f.write("lib/naomi2/initdexp.zip", b"key");
    f.write(
        "lib/naomi2/initdv2e/gds-0027.chd",
        &fake_chd("c146e8c4d71d1a0cbe836a0b50dea87797869da8"),
    );
    f.write("lib/tp/Initial D 5/id5.elf", b"not the game");
    f.write("lib/tp/junk/initdv3e/gds-0033.chd", b"junk");
    f.write("bios/naomi2.zip", b"bios");
}

#[test]
fn help() {
    let f = Fixture::new();
    let o = Command::new(env!("CARGO_BIN_EXE_initiald-launcher"))
        .arg("--help")
        .env("INITIALD_DATA", f.path("data"))
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .expect("run the launcher");
    assert!(o.status.success());
    rea_launcher::snapshot!("help.txt", String::from_utf8_lossy(&o.stdout));
}

/// What each wrong command line says, and that it changes nothing.
#[test]
fn every_usage_error() {
    let f = Fixture::new();
    let mut out = String::new();
    let missing = f.arg("missing");
    for args in [
        vec!["--bogus"],
        vec!["--render"],
        vec!["--game", "id6", "--crt", "rainbow"],
        vec!["--play", "id1", "--seconds", "soon"],
        vec!["--crt", "off"],
        vec!["--game", "nope", "--crt", "off"],
        vec!["--play", "nope"],
        vec!["--list"],
        vec!["--library", &missing],
        vec!["--naomi2-bios", &missing],
    ] {
        f.run(&mut out, &args);
    }
    let left = fs::read_dir(f.path("data")).unwrap().count();
    let _ = writeln!(out, "-- the data folder holds {left} entries");
    rea_launcher::snapshot!("cli-errors.txt", out);
}

/// Libraries and the BIOS saved to `launcher.cfg` (a player's comments and
/// unknown keys in it), the list of games found, options saved for a game
/// (its file's comments and unknown keys kept), `--game` alone, and
/// `--play` of a game the library lacks.
#[test]
fn the_library_list_and_options() {
    let f = Fixture::new();
    library(&f);
    let mut out = String::new();
    let (naomi, tp, bios) = (
        f.arg("lib/naomi2"),
        f.arg("lib/tp"),
        f.arg("bios/naomi2.zip"),
    );
    f.write(
        "data/launcher.cfg",
        format!("# my libraries\nlibrary = {naomi}\nfuture = 1\nnaomi2_bios =\n").as_bytes(),
    );
    f.run(&mut out, &["--library", &tp, "--library", &naomi]);
    f.show(&mut out, "launcher.cfg");
    f.run(&mut out, &["--naomi2-bios", &bios]);
    f.show(&mut out, "launcher.cfg");
    f.run(&mut out, &["--list"]);
    f.run(&mut out, &["--game", "id6"]);
    f.show(&mut out, "settings/id6.cfg");
    f.write(
        "data/settings/id6.cfg",
        b"# mine\nupscaler = nis\nfuture_key = 7\nsharpness = 50\n",
    );
    f.run(
        &mut out,
        &[
            "--game",
            "id6",
            "--render",
            "2560x1440",
            "--crt",
            "lottes",
            "--fullscreen",
            "off",
        ],
    );
    f.show(&mut out, "settings/id6.cfg");
    // a picture value already set: the file is the player's, so no look is
    // decided from the screen (Windows knows it: the result would depend on
    // the machine)
    f.write("data/settings/id7.cfg", b"# mine\nfullscreen = on\n");
    f.run(
        &mut out,
        &[
            "--play",
            "id7",
            "--deadzone",
            "0.1",
            "--output",
            "1920x1080",
        ],
    );
    f.show(&mut out, "settings/id7.cfg");
    rea_launcher::snapshot!("cli-library.txt", out);
}

/// `--play` of games the library has, without what they need (no
/// components, no git, docker or gamescope on PATH, a Flycast that is
/// only a name): what the player is told, and the command line the session
/// log records. Nothing can start.
#[test]
fn play_without_the_components() {
    let f = Fixture::new();
    library(&f);
    let mut out = String::new();
    let (naomi, tp) = (f.arg("lib/naomi2"), f.arg("lib/tp"));
    f.run(&mut out, &["--library", &naomi, "--library", &tp]);
    f.run(&mut out, &["--play", "id1", "--headless"]);
    f.write("bin/flycast", b"");
    f.run(&mut out, &["--play", "id2", "--headless"]);
    f.run(&mut out, &["--play", "id1", "--headless"]);
    f.run(&mut out, &["--naomi2-bios", &f.arg("bios/naomi2.zip")]);
    f.run(&mut out, &["--play", "id1", "--headless", "--seconds", "1"]);
    f.show(&mut out, "runtimes/id1/session.log");
    let host = if cfg!(windows) { "windows" } else { "linux" };
    rea_launcher::snapshot!(&format!("cli-play.{host}.txt"), out);
}
