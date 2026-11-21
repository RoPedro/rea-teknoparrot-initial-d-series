//! The Windows executable's version information, icon and manifest
//! (rea-launcher-build): a named, versioned program with the launcher's
//! icon instead of an anonymous one that antivirus heuristics distrust.
fn main() {
    rea_launcher_build::Resources::new(
        "Initial D Launcher",
        "One launcher for the Initial D arcade series",
    )
    .company("AkitaOnRails")
    .file_name("initiald-launcher.exe")
    .icon("../../packaging/appimage/initiald-launcher.png")
    .embed()
    .unwrap();
}
