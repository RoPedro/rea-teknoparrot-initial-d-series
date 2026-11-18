#!/bin/sh
# Builds the launcher's components from pinned sources into a components folder:
#   lindbergh/  lindbergh-loader (CC BY-NC-SA 4.0: built here on the player's machine, never
#               shipped in our packages) with patches/lindbergh applied, in a throwaway
#               ubuntu:22.04 container (docker or podman)
#   wal/        windowsarcadeloader (GPL-3.0) with patches/wal applied: arcade-launcher (Linux),
#               arcade-launcher.exe (its Windows build, linked with LLD), payloads/,
#               systemprofiles/  (needs Rust with the i686/x86_64-pc-windows-gnu targets, MinGW,
#               LLD and mono's mcs); not rebuilt while its commit and patches are unchanged
#   runner/     GE-Proton 11-12
#   winetricks  windowsarcadeloader installs native XAudio with it
#   flycast/    Flycast for Windows (GPL-2.0) and its licence
#   vcredist/   Microsoft's Visual C++ 2008 SP1 Redistributable (x86), Microsoft's own download
#               (signed by Microsoft): Initial D 6, 7 and 8 on Windows need it, the launcher
#               installs it on their first start
#   shaders/    the launcher's own CRT effect
#   art/        the window's original illustrations
# The downloads are pinned in packaging/components.pins and fetched through the SDK's
# sdk/packaging/pins.sh (checked by SHA-256; a download cut short is never used).
# usage: scripts/build-components.sh <components dir> [lindbergh|wal|runner|winetricks|flycast|vcredist|shaders|assets ...]
# LAUNCHER_FEATURES: passed to windowsarcadeloader's build (packages: sdl3/build-from-source-static)
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
out=$(mkdir -p "$1" && cd "$1" && pwd); shift
parts=${*:-lindbergh wal runner winetricks shaders assets}
work="$out/.build"; mkdir -p "$work"

LINDBERGH_REPO=https://github.com/lindbergh-loader/lindbergh-loader
LINDBERGH_COMMIT=0af606d845b70339c335785c0eba68b47b78df3c
WAL_REPO=https://github.com/aderumier/windowsarcadeloader
WAL_COMMIT=4f42c6d86ddaaa9d2e4e8c21e0046bb3027b111d
RUNNER=GE-Proton11-12-x86_64

# fetch NAME: the pinned download NAME in the work folder (its path)
fetch() {
  . "$here/sdk/packaging/pins.sh"
  PIN_FILE="$here/packaging/components.pins" PIN_DIR='' pin_fetch "$1" "$work" \
    || { echo "$1: download failed (see above)" >&2; exit 1; }
}

checkout() { # repo commit dir
  [ -d "$3/.git" ] || git clone -q "$1" "$3"
  git -C "$3" fetch -q origin "$2" 2>/dev/null || true
  git -C "$3" checkout -q --force "$2"
  git -C "$3" clean -qfdx
}

for part in $parts; do case $part in
lindbergh)
  checkout "$LINDBERGH_REPO" "$LINDBERGH_COMMIT" "$work/lindbergh-loader"
  for p in "$here"/patches/lindbergh/*.patch; do git -C "$work/lindbergh-loader" apply "$p"; done
  engine=$(command -v docker || command -v podman) || { echo "lindbergh: needs docker or podman" >&2; exit 1; }
  mkdir -p "$out/lindbergh"
  # REA_BUILD_NAME: the container's name, so the launcher can stop it
  "$engine" run --rm --name "${REA_BUILD_NAME:-rea-initiald-build-$$}" -v "$work/lindbergh-loader:/src:ro" -v "$out/lindbergh:/out" \
    -v "$here/scripts/lindbergh-in-container.sh:/build.sh:ro" -e OWNER="$(id -u):$(id -g)" \
    ubuntu:22.04 sh /build.sh
  cp "$work/lindbergh-loader/libs/libssl.so.0.9.7" "$work/lindbergh-loader/libs/libcrypto.so.0.9.7" \
     "$work/lindbergh-loader/libs/libopenal.so.0" "$out/lindbergh/"
  cp "$work/lindbergh-loader/LICENSE.md" "$out/lindbergh/LICENSE.md"
  ;;
wal)
  stamp="$WAL_COMMIT $(cat "$here"/patches/wal/*.patch | sha256sum | cut -c1-16)"
  if [ -f "$out/wal/.stamp" ] && [ "$(cat "$out/wal/.stamp")" = "$stamp" ]; then
    echo "wal: up to date ($stamp)"
    continue
  fi
  checkout "$WAL_REPO" "$WAL_COMMIT" "$work/windowsarcadeloader"
  for p in "$here"/patches/wal/*.patch; do git -C "$work/windowsarcadeloader" apply "$p"; done
  (cd "$work/windowsarcadeloader" && ./build.sh)
  # the Windows host build (patches/wal/0003), linked with LLD (GNU ld 2.47 mis-links Rust's
  # Windows import tables)
  (cd "$work/windowsarcadeloader" && \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C link-arg=-fuse-ld=lld" \
    cargo build --release --target x86_64-pc-windows-gnu -p wal-winhost)
  mkdir -p "$out/wal"
  cp "$work/windowsarcadeloader/dist/arcade-launcher" "$out/wal/"
  cp "$work/windowsarcadeloader/target/x86_64-pc-windows-gnu/release/arcade-launcher.exe" "$out/wal/"
  rm -rf "$out/wal/payloads" "$out/wal/systemprofiles"
  cp -r "$work/windowsarcadeloader/dist/payloads" "$out/wal/payloads"
  cp -r "$work/windowsarcadeloader/systemprofiles" "$out/wal/systemprofiles"
  cp "$work/windowsarcadeloader/LICENSE" "$out/wal/LICENSE"
  echo "$stamp" > "$out/wal/.stamp"
  ;;
flycast)
  f=$(fetch flycast-win)
  l=$(fetch flycast-license)
  rm -rf "$out/flycast"; mkdir -p "$out/flycast"
  unzip -q -o "$f" flycast.exe -d "$out/flycast"
  cp "$l" "$out/flycast/LICENSE"
  ;;
vcredist)
  f=$(fetch vcredist)
  mkdir -p "$out/vcredist"; cp "$f" "$out/vcredist/vcredist_x86.exe"
  ;;
runner)
  f=$(fetch runner)
  tar -xzf "$f" -C "$work"
  rm -rf "$out/runner"; mv "$work/$RUNNER/files" "$out/runner"
  ;;
winetricks)
  f=$(fetch winetricks)
  install -m 755 "$f" "$out/winetricks"
  ;;
shaders)
  mkdir -p "$out/shaders"; cp "$here"/assets/shaders/*.fx "$out/shaders/"
  ;;
assets)
  rm -rf "$out/art" "$out/fonts"; cp -r "$here/assets/art" "$out/art"
  ;;
*) echo "unknown part $part" >&2; exit 2;;
esac; done
echo "components ready in $out"
