//! Where the launcher keeps things (`rea_launcher::hub::HubPaths`, plus
//! Initial D's own components).
//!
//! - data (`$INITIALD_DATA`, else `$XDG_DATA_HOME/rea-initiald` or
//!   `~/.local/share/rea-initiald`; on Windows `%LOCALAPPDATA%\rea-initiald`):
//!   `launcher.cfg`, `settings/`, `runtimes/` (one per game, saves
//!   included), `wal/` (windowsarcadeloader's root: its Wine prefix and
//!   profile layers), `launcher.log`;
//! - components (`$INITIALD_COMPONENTS`, else `components/` beside the
//!   program): `wal/`, `runner/`, `shaders/`, `art/`, on Windows `flycast/`,
//!   `vcredist/` and `directx/`, and `lindbergh/` from a developer's build; the
//!   player's lindbergh-loader is built on first use into the data
//!   folder's `components/lindbergh/` (CC BY-NC-SA: never inside a
//!   package).

use std::path::{Path, PathBuf};

use initiald_core::launch::Components;
use rea_launcher::hub::{HubDirs, HubPaths, PathField, PathKind};

pub const DIRS: HubDirs = HubDirs {
    env_prefix: "INITIALD",
    name: "rea-initiald",
    dev_assets: if cfg!(debug_assertions) {
        Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"))
    } else {
        None
    },
};

/// `launcher.cfg`'s other path: the NAOMI 2 BIOS set.
pub const NAOMI2_BIOS: PathField = PathField {
    key: "naomi2_bios",
    aliases: &[],
    kind: PathKind::File,
    flag: "--naomi2-bios",
    flag_aliases: &[],
    help: "where your naomi2.zip is (Ver. 1–3)",
    heading: "NAOMI 2 BIOS (naomi2.zip), needed for Initial D 1 to 3",
    label: "NAOMI 2 BIOS file",
    placeholder: "/path/to/naomi2.zip",
    button: "Use BIOS",
    empty: "Type the path of naomi2.zip first.",
    saved: "NAOMI 2 BIOS saved.",
};

/// The path fields, in `HubConfig::paths` order.
pub static FIELDS: [PathField; 1] = [NAOMI2_BIOS];

/// Initial D's folders besides the hub's.
pub trait Folders {
    /// windowsarcadeloader's root (shared by every Windows game).
    fn wal_root(&self) -> PathBuf;
    /// Where lindbergh-loader is: the components' `lindbergh/` when it is
    /// there, else the one built on first use in the data folder.
    fn lindbergh(&self) -> PathBuf;
    /// The components folder lindbergh-loader is built into on first use.
    fn lindbergh_build_target(&self) -> PathBuf;
    /// `scripts/build-components.sh`: beside the program in a package, the
    /// repository's own from a development build.
    fn build_script(&self) -> PathBuf;
    /// Microsoft's Visual C++ 2008 SP1 Redistributable (x86) in a Windows
    /// package (see [`crate::session::Setup::VisualCpp2008`]).
    fn vcredist(&self) -> PathBuf;
    /// Microsoft's DirectX setup (`DXSETUP.exe` and the cabinets of
    /// XAudio2 2.3) in a Windows package (see
    /// [`crate::session::Setup::DirectX`]).
    fn directx_setup(&self) -> PathBuf;
    fn components(&self) -> Components;
}

impl Folders for HubPaths {
    fn wal_root(&self) -> PathBuf {
        self.data.join("wal")
    }

    fn lindbergh(&self) -> PathBuf {
        let packaged = self.components.join("lindbergh");
        if packaged.join("lindbergh.so").is_file() {
            packaged
        } else {
            self.lindbergh_build_target().join("lindbergh")
        }
    }

    fn lindbergh_build_target(&self) -> PathBuf {
        self.data.join("components")
    }

    fn build_script(&self) -> PathBuf {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("scripts/build-components.sh")));
        match beside {
            Some(s) if s.is_file() => s,
            _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/build-components.sh"),
        }
    }

    fn vcredist(&self) -> PathBuf {
        self.components.join("vcredist").join("vcredist_x86.exe")
    }

    fn directx_setup(&self) -> PathBuf {
        self.components.join("directx").join("DXSETUP.exe")
    }

    fn components(&self) -> Components {
        Components {
            lindbergh: self.lindbergh(),
            wal: self.components.join("wal"),
            runner: self.components.join("runner"),
            winetricks: self.components.join("winetricks"),
            flycast: self.components.join("flycast"),
            shaders: self.components.join("shaders"),
        }
    }
}

/// The folders for a test: data and components under `t`.
#[cfg(test)]
pub fn in_dir(t: &Path) -> HubPaths {
    HubPaths {
        data: t.join("data"),
        components: t.join("components"),
        dev_assets: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lindbergh_is_the_packaged_one_else_the_one_built_in_the_data_folder() {
        let t = tempfile::tempdir().unwrap();
        let p = HubPaths {
            data: t.path().join("data"),
            components: t.path().join("pkg/components"),
            dev_assets: None,
        };
        // a package never carries it: the first start builds it in the data folder
        assert_eq!(p.lindbergh(), t.path().join("data/components/lindbergh"));
        assert_eq!(p.components().lindbergh, p.lindbergh());
        let dev = t.path().join("pkg/components/lindbergh");
        std::fs::create_dir_all(&dev).unwrap();
        std::fs::write(dev.join("lindbergh.so"), b"").unwrap();
        assert_eq!(p.lindbergh(), dev);
    }
}
