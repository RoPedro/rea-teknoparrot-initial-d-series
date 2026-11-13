//! What to write and what to run for a game, as plain data.
//!
//! [`plan`] turns a game, its folders and the settings into the files the
//! launcher writes and the process it starts, so tests can check the whole
//! wiring (setting → file / argument / environment) without starting
//! anything. Every value here was seen to work in docs/RUNTIME_NOTES.md.

use std::path::{Path, PathBuf};

use crate::catalog::{Game, Route};
use crate::input::StickLayout;
use crate::settings::{Crt, Settings, Size};

/// A process to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Files written (replaced) before every launch.
    pub files: Vec<(PathBuf, String)>,
    /// Symlinks made before every launch (`at` → `target`).
    pub links: Vec<(PathBuf, PathBuf)>,
    pub game: Command,
    /// Arguments for gamescope, which wraps the game.
    pub gamescope: Vec<String>,
    /// The launcher feeds the game's input through the loader's stdin.
    pub input_pipe: Option<StickLayout>,
    /// The size the game draws at.
    pub game_size: Size,
}

/// The launcher's own components (built or downloaded on request).
#[derive(Debug, Clone)]
pub struct Components {
    /// lindbergh-loader's build: `lindbergh`, `lindbergh.so`, its 32-bit libraries.
    pub lindbergh: PathBuf,
    /// windowsarcadeloader: `arcade-launcher`, `payloads/`, `systemprofiles/`.
    pub wal: PathBuf,
    /// The Wine runner windowsarcadeloader runs games with (`bin/wine`).
    pub runner: PathBuf,
    /// ReShade effects (`crt-lottes.fx`).
    pub shaders: PathBuf,
}

/// Where the launcher keeps per-game state.
#[derive(Debug, Clone)]
pub struct Places {
    /// The game's runtime folder.
    pub runtime: PathBuf,
    /// windowsarcadeloader's root (shared by every Windows game: prefix, links).
    pub wal_root: PathBuf,
    /// The NAOMI 2 BIOS set, when the player has one.
    pub naomi2_bios: Option<PathBuf>,
}

/// How this machine reaches the network (lindbergh-loader's `NIC_NAME`):
/// the default route's interface, else loopback.
pub fn default_interface(proc_net_route: &str) -> String {
    proc_net_route
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let (name, dest) = (f.next()?, f.next()?);
            (dest == "00000000" && name != "lo").then(|| name.to_string())
        })
        .next()
        .unwrap_or_else(|| "lo".into())
}

pub fn game_size(game: &Game, s: &Settings) -> Size {
    match game.route {
        // Flycast draws the 4:3 game at a multiple of 480 lines, widened to 16:9
        Route::Flycast { .. } => {
            let h = s.render.map_or(game.native.h, |r| r.h);
            Size::new(h * 16 / 9, h)
        }
        _ => s.render.unwrap_or(game.native),
    }
}

fn gamescope_args(s: &Settings, size: Size, c: &Components, headless: bool) -> Vec<String> {
    let mut a: Vec<String> = [
        "-w",
        &size.w.to_string(),
        "-h",
        &size.h.to_string(),
        "-W",
        &s.output.w.to_string(),
        "-H",
        &s.output.h.to_string(),
        "-r",
        "60",
        "-F",
        if headless { "linear" } else { s.upscaler.key() },
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    if !headless && matches!(s.upscaler.key(), "fsr" | "nis") {
        a.extend(["--sharpness".into(), s.sharpness.to_string()]);
    }
    if s.crt == Crt::Lottes {
        a.extend([
            "--reshade-effect".into(),
            c.shaders
                .join("crt-lottes.fx")
                .to_string_lossy()
                .into_owned(),
        ]);
    }
    if headless {
        a.extend(["--backend".into(), "headless".into()]);
    } else if s.fullscreen {
        a.push("-f".into());
    }
    a
}

pub fn lindbergh_ini(size: Size, nic: &str) -> String {
    format!(
        "# Written by the launcher before every start.\n\
         [Display]\nWIDTH = {w}\nHEIGHT = {h}\nBOOST_RENDER_RES = true\nFULLSCREEN = true\n\
         KEEP_ASPECT_RATIO = true\nHIDE_CURSOR = true\nBORDER_ENABLED = false\n\
         [Input]\nINPUT_MODE = 0\n\
         [Emulation]\nREGION = EX\nFREEPLAY = true\nEMULATE_JVS = true\nEMULATE_DRIVEBOARD = AUTO\n\
         EMULATE_ID_CARDREADER = AUTO\n\
         [System]\nDEBUG_MSGS = false\n\
         [Cards]\nID_CARDFILE_AUTOLOAD = true\nCARDFILE_01 = \"Card_01.crd\"\nCARDFILE_02 = \"Card_02.crd\"\n\
         [Paths]\nSRAM_PATH = \"sram.bin\"\nEEPROM_PATH = \"eeprom.bin\"\n\
         [Graphics]\nGPU_VENDOR = 0\nFPS_LIMITER_ENABLED = true\nFPS_TARGET = 60.0\n\
         [Network]\nNIC_NAME = \"{nic}\"\n",
        w = size.w,
        h = size.h
    )
}

/// lindbergh-loader's `controls.ini`: the launcher's pad layout for SDL game
/// controllers, keyboard as a fallback.
pub const LINDBERGH_CONTROLS: &str = "# Written by the launcher before every start.
[Common]
Test = KEY_T
P1_Coin = KEY_5, GC0_BUTTON_BACK
P1_Start = KEY_1, GC0_BUTTON_START, GC0_BUTTON_A
P1_Service = KEY_S
[Driving]
P1_Steer_Left = KEY_Left
P1_Steer_Right = KEY_Right
P1_Gas_Digital = KEY_Up
P1_Brake_Digital = KEY_Down
ViewChange = KEY_V, GC0_BUTTON_Y
GearUp = KEY_A, GC0_BUTTON_RIGHTSHOULDER
GearDown = KEY_Z, GC0_BUTTON_LEFTSHOULDER
CardInsert = KEY_F7, GC0_BUTTON_X
P1_Steer = GC0_AXIS_LEFTX
P1_Gas = GC0_AXIS_RIGHTTRIGGER
P1_Brake = GC0_AXIS_LEFTTRIGGER
";

pub fn flycast_cfg(render_h: u32) -> String {
    format!(
        "# Written by the launcher before every start.\n[config]\n\
         rend.Resolution = {render_h}\nrend.WideScreen = yes\nrend.WidescreenGameHacks = yes\n\
         Dynarec.Enabled = yes\n[window]\nfullscreen = yes\n"
    )
}

/// The profile layer the launcher gives windowsarcadeloader for one game:
/// the runner, a prefix with native XACT (the RingEdge games' sound thread
/// crashes on Wine's own), no ReShade or gamescope of its own (the launcher
/// wraps the game), and the render size.
pub fn wal_layer(game: &Game, size: Size) -> String {
    let mut env = Vec::new();
    if let Route::Wal {
        system: "ringedge", ..
    } = game.route
    {
        env.push(("WAL_RINGEDGE_GL_SCALE", format!("{}x{}", size.w, size.h)));
    }
    let mut out = String::from(
        "# Written by the launcher before every start.\n\
         runner: runner\nprefix: wine-prefix/initiald\nprefix_tricks: [xact]\nreshade: false\n\
         gamescope:\n  enabled: false\n",
    );
    if !env.is_empty() {
        out.push_str("env:\n");
        for (k, v) in env {
            out.push_str(&format!("  {k}: \"{v}\"\n"));
        }
    }
    out
}

pub fn plan(
    game: &Game,
    entry_dir: &Path,
    places: &Places,
    c: &Components,
    s: &Settings,
    nic: &str,
    headless: bool,
) -> Plan {
    let size = game_size(game, s);
    let rt = &places.runtime;
    let mut files = Vec::new();
    let mut links = Vec::new();
    let mut input_pipe = None;
    let game_cmd = match game.route {
        Route::Lindbergh => {
            files.push((rt.join("lindbergh.ini"), lindbergh_ini(size, nic)));
            files.push((rt.join("controls.ini"), LINDBERGH_CONTROLS.to_string()));
            Command {
                // the loader finds lindbergh.so next to the folder it starts in
                program: c.lindbergh.join("lindbergh"),
                args: vec![
                    "-c".into(),
                    rt.join("lindbergh.ini").to_string_lossy().into_owned(),
                    "-o".into(),
                    rt.join("controls.ini").to_string_lossy().into_owned(),
                    "-g".into(),
                    rt.to_string_lossy().into_owned(),
                ],
                env: vec![(
                    "LD_LIBRARY_PATH".into(),
                    format!("{}:{}", c.lindbergh.display(), rt.join("lib").display()),
                )],
                cwd: c.lindbergh.clone(),
            }
        }
        Route::Flycast { set } => {
            files.push((rt.join("config/flycast/emu.cfg"), flycast_cfg(size.h)));
            if let Some(bios) = &places.naomi2_bios {
                links.push((rt.join("data/flycast/naomi2.zip"), bios.clone()));
            }
            Command {
                program: "flycast".into(),
                args: vec![
                    entry_dir
                        .join(format!("{set}.zip"))
                        .to_string_lossy()
                        .into_owned(),
                ],
                env: vec![
                    (
                        "XDG_CONFIG_HOME".into(),
                        rt.join("config").to_string_lossy().into_owned(),
                    ),
                    (
                        "XDG_DATA_HOME".into(),
                        rt.join("data").to_string_lossy().into_owned(),
                    ),
                ],
                cwd: rt.clone(),
            }
        }
        Route::Wal { system, profile } => {
            let root = &places.wal_root;
            files.push((
                rt.join(format!("{profile}.windowsloader")),
                format!("{}\n", game.main_file),
            ));
            let layer = root.join(format!("layer-{}.yaml", game.slug));
            files.push((layer.clone(), wal_layer(game, size)));
            links.push((root.join("systemprofiles"), c.wal.join("systemprofiles")));
            links.push((root.join("dist/payloads"), c.wal.join("payloads")));
            links.push((root.join("wine-runners/runner"), c.runner.clone()));
            input_pipe = Some(if system == "ringedge" {
                StickLayout::RingEdge
            } else {
                StickLayout::Alls
            });
            Command {
                program: c.wal.join("arcade-launcher"),
                args: vec![
                    "run".into(),
                    rt.to_string_lossy().into_owned(),
                    "--root".into(),
                    root.to_string_lossy().into_owned(),
                    "--profile".into(),
                    layer.to_string_lossy().into_owned(),
                    "--input-pipe".into(),
                ],
                env: vec![],
                cwd: root.clone(),
            }
        }
    };
    Plan {
        files,
        links,
        game: game_cmd,
        gamescope: gamescope_args(s, size, c, headless),
        input_pipe,
        game_size: size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GameId, game};

    fn comps() -> Components {
        Components {
            lindbergh: "/c/lindbergh".into(),
            wal: "/c/wal".into(),
            runner: "/c/runner".into(),
            shaders: "/c/shaders".into(),
        }
    }

    fn places(slug: &str) -> Places {
        Places {
            runtime: PathBuf::from("/d/runtimes").join(slug),
            wal_root: "/d/wal".into(),
            naomi2_bios: Some("/bios/naomi2.zip".into()),
        }
    }

    fn arg_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    }

    #[test]
    fn the_default_route_interface_is_picked() {
        let route = "Iface\tDestination\tGateway\n\
                     lo\t0000007F\t00000000\n\
                     eth0\t00000000\t010011AC\n";
        assert_eq!(default_interface(route), "eth0");
        assert_eq!(default_interface("Iface\tDestination\n"), "lo");
    }

    #[test]
    fn ringedge_render_size_reaches_the_payload_and_gamescope() {
        let g = game(GameId::Id6);
        let s = Settings {
            render: Some(Size::new(3840, 2160)),
            ..Settings::default()
        };
        let p = plan(
            g,
            Path::new("/lib/id6"),
            &places("id6"),
            &comps(),
            &s,
            "eth0",
            false,
        );
        let layer = &p
            .files
            .iter()
            .find(|(f, _)| f.ends_with("layer-id6.yaml"))
            .unwrap()
            .1;
        assert!(
            layer.contains("WAL_RINGEDGE_GL_SCALE: \"3840x2160\""),
            "{layer}"
        );
        assert!(layer.contains("prefix_tricks: [xact]"));
        assert_eq!(arg_after(&p.gamescope, "-w").as_deref(), Some("3840"));
        assert_eq!(p.input_pipe, Some(StickLayout::RingEdge));
        assert!(p.game.args.contains(&"--input-pipe".to_string()));
        let marker = &p
            .files
            .iter()
            .find(|(f, _)| f.ends_with("initial-d-arcade-stage-6.windowsloader"))
            .unwrap()
            .1;
        assert_eq!(marker, "id6_dump_.exe\n");
    }

    #[test]
    fn the_seasons_use_the_alls_stick_layout() {
        let p = plan(
            game(GameId::TheArcade5),
            Path::new("/lib/s5"),
            &places("ta5"),
            &comps(),
            &Settings::default(),
            "eth0",
            false,
        );
        assert_eq!(p.input_pipe, Some(StickLayout::Alls));
    }

    #[test]
    fn lindbergh_gets_its_size_interface_and_controls() {
        let s = Settings {
            render: Some(Size::new(2560, 1440)),
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id5),
            Path::new("/lib/id5"),
            &places("id5"),
            &comps(),
            &s,
            "enp5s0",
            false,
        );
        let ini = &p.files[0].1;
        assert!(ini.contains("WIDTH = 2560\nHEIGHT = 1440"));
        assert!(ini.contains("NIC_NAME = \"enp5s0\""));
        assert!(ini.contains("GPU_VENDOR = 0"));
        assert_eq!(p.game.cwd, PathBuf::from("/c/lindbergh"));
        assert_eq!(
            arg_after(&p.game.args, "-g").as_deref(),
            Some("/d/runtimes/id5")
        );
        assert!(p.input_pipe.is_none());
    }

    #[test]
    fn flycast_draws_at_the_render_height_widened() {
        let s = Settings {
            render: Some(Size::new(3840, 2160)),
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id1),
            Path::new("/roms/naomi2"),
            &places("id1"),
            &comps(),
            &s,
            "eth0",
            false,
        );
        assert!(p.files[0].1.contains("rend.Resolution = 2160"));
        assert_eq!(p.game_size, Size::new(3840, 2160));
        assert_eq!(p.game.args, ["/roms/naomi2/initdexp.zip"]);
        assert_eq!(p.links[0].1, PathBuf::from("/bios/naomi2.zip"));
    }

    #[test]
    fn crt_upscaler_and_headless_reach_gamescope() {
        let s = Settings {
            crt: Crt::Lottes,
            ..Settings::default()
        };
        let p = plan(
            game(GameId::Id6),
            Path::new("/l"),
            &places("id6"),
            &comps(),
            &s,
            "eth0",
            false,
        );
        assert_eq!(
            arg_after(&p.gamescope, "--reshade-effect").as_deref(),
            Some("/c/shaders/crt-lottes.fx")
        );
        assert_eq!(arg_after(&p.gamescope, "-F").as_deref(), Some("fsr"));
        assert!(p.gamescope.contains(&"-f".to_string()));
        let h = plan(
            game(GameId::Id6),
            Path::new("/l"),
            &places("id6"),
            &comps(),
            &s,
            "eth0",
            true,
        );
        assert_eq!(
            arg_after(&h.gamescope, "--backend").as_deref(),
            Some("headless")
        );
        assert_eq!(arg_after(&h.gamescope, "-F").as_deref(), Some("linear"));
        assert!(!h.gamescope.contains(&"-f".to_string()));
    }
}
