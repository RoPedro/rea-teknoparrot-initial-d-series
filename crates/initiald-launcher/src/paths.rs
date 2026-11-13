//! Where the launcher keeps things.
//!
//! - data (`$INITIALD_DATA`, else `$XDG_DATA_HOME/rea-initiald` or
//!   `~/.local/share/rea-initiald`): `launcher.cfg`, `settings/`,
//!   `runtimes/` (one per game, saves included), `wal/` (windowsarcadeloader's
//!   root: its Wine prefix and profile layers), `launcher.log`;
//! - components (`$INITIALD_COMPONENTS`, else `components/` beside the
//!   program): `lindbergh/`, `wal/`, `runner/`, `shaders/`.

use std::path::{Path, PathBuf};

use initiald_core::launch::Components;

#[derive(Debug, Clone)]
pub struct Paths {
    pub data: PathBuf,
    pub components: PathBuf,
}

/// What `launcher.cfg` remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// Folders the games are looked for in.
    pub libraries: Vec<PathBuf>,
    /// The NAOMI 2 BIOS set (`naomi2.zip`).
    pub naomi2_bios: Option<PathBuf>,
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let mut c = Config::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "library" if !v.is_empty() => c.libraries.push(v.into()),
                "naomi2_bios" if !v.is_empty() => c.naomi2_bios = Some(v.into()),
                _ => {}
            }
        }
        c
    }

    pub fn save(&self) -> String {
        let mut s = String::from("# Written by the launcher.\n");
        for l in &self.libraries {
            s.push_str(&format!("library = {}\n", l.display()));
        }
        if let Some(b) = &self.naomi2_bios {
            s.push_str(&format!("naomi2_bios = {}\n", b.display()));
        }
        s
    }

    pub fn add_library(&mut self, dir: &Path) {
        if !self.libraries.iter().any(|l| l == dir) {
            self.libraries.push(dir.to_path_buf());
        }
    }
}

impl Paths {
    pub fn discover() -> Self {
        let data = std::env::var_os("INITIALD_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(default_data);
        let components = std::env::var_os("INITIALD_COMPONENTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|e| e.parent().map(|p| p.join("components")))
                    .unwrap_or_else(|| PathBuf::from("components"))
            });
        Self { data, components }
    }

    pub fn config_file(&self) -> PathBuf {
        self.data.join("launcher.cfg")
    }
    pub fn settings(&self, slug: &str) -> PathBuf {
        self.data.join("settings").join(format!("{slug}.cfg"))
    }
    pub fn runtime(&self, slug: &str) -> PathBuf {
        self.data.join("runtimes").join(slug)
    }
    pub fn wal_root(&self) -> PathBuf {
        self.data.join("wal")
    }
    pub fn log(&self) -> PathBuf {
        self.data.join("launcher.log")
    }
    pub fn components(&self) -> Components {
        Components {
            lindbergh: self.components.join("lindbergh"),
            wal: self.components.join("wal"),
            runner: self.components.join("runner"),
            shaders: self.components.join("shaders"),
        }
    }

    pub fn config(&self) -> Config {
        Config::parse(&std::fs::read_to_string(self.config_file()).unwrap_or_default())
    }

    pub fn save_config(&self, c: &Config) -> std::io::Result<()> {
        let f = self.config_file();
        std::fs::create_dir_all(&self.data)?;
        let tmp = f.with_extension("tmp");
        std::fs::write(&tmp, c.save())?;
        std::fs::rename(tmp, f)
    }
}

fn default_data() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rea-initiald")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_several_libraries_and_the_bios() {
        let mut c = Config::default();
        c.add_library(Path::new("/roms/TeknoParrot"));
        c.add_library(Path::new("/roms/naomi2"));
        c.add_library(Path::new("/roms/naomi2"));
        c.naomi2_bios = Some("/bios/naomi2.zip".into());
        assert_eq!(Config::parse(&c.save()), c);
        assert_eq!(c.libraries.len(), 2);
    }
}
