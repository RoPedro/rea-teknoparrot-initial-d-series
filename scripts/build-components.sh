#!/bin/sh
# Builds the launcher's components from pinned sources into a components folder:
#   lindbergh/  lindbergh-loader (CC BY-NC-SA 4.0: built here on the player's machine, never
#               shipped in our packages) with patches/lindbergh applied, in a throwaway
#               ubuntu:22.04 container (docker or podman)
#   wal/        windowsarcadeloader (GPL-3.0) with patches/wal applied: arcade-launcher,
#               payloads/, systemprofiles/  (needs Rust with the i686/x86_64-pc-windows-gnu
#               targets and MinGW)
#   runner/     GE-Proton 11-12, checked by SHA-256
#   shaders/    the launcher's own CRT effect
#   art/ fonts/ the window's original illustrations and the Barlow typefaces (OFL)
# usage: scripts/build-components.sh <components dir> [lindbergh|wal|runner|shaders ...]
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
out=$(mkdir -p "$1" && cd "$1" && pwd); shift
parts=${*:-lindbergh wal runner shaders assets}
work="$out/.build"; mkdir -p "$work"

LINDBERGH_REPO=https://github.com/lindbergh-loader/lindbergh-loader
LINDBERGH_COMMIT=0af606d845b70339c335785c0eba68b47b78df3c
WAL_REPO=https://github.com/aderumier/windowsarcadeloader
WAL_COMMIT=4f42c6d86ddaaa9d2e4e8c21e0046bb3027b111d
RUNNER=GE-Proton11-12-x86_64
RUNNER_URL=https://github.com/GloriousEggroll/proton-ge-custom/releases/download/GE-Proton11-12/$RUNNER.tar.gz
RUNNER_SHA256=0595159e417c3dbbfb7cc3ae0d3ed15a617cc607aa23ef756ea79029d7af5d70

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
  "$engine" run --rm -v "$work/lindbergh-loader:/src:ro" -v "$out/lindbergh:/out" \
    -v "$here/scripts/lindbergh-in-container.sh:/build.sh:ro" -e OWNER="$(id -u):$(id -g)" \
    ubuntu:22.04 sh /build.sh
  cp "$work/lindbergh-loader/libs/libssl.so.0.9.7" "$work/lindbergh-loader/libs/libcrypto.so.0.9.7" \
     "$work/lindbergh-loader/libs/libopenal.so.0" "$out/lindbergh/"
  cp "$work/lindbergh-loader/LICENSE.md" "$out/lindbergh/LICENSE.md"
  ;;
wal)
  checkout "$WAL_REPO" "$WAL_COMMIT" "$work/windowsarcadeloader"
  for p in "$here"/patches/wal/*.patch; do git -C "$work/windowsarcadeloader" apply "$p"; done
  (cd "$work/windowsarcadeloader" && ./build.sh)
  mkdir -p "$out/wal"
  cp "$work/windowsarcadeloader/dist/arcade-launcher" "$out/wal/"
  rm -rf "$out/wal/payloads" "$out/wal/systemprofiles"
  cp -r "$work/windowsarcadeloader/dist/payloads" "$out/wal/payloads"
  cp -r "$work/windowsarcadeloader/systemprofiles" "$out/wal/systemprofiles"
  cp "$work/windowsarcadeloader/LICENSE" "$out/wal/LICENSE"
  ;;
runner)
  f="$work/$RUNNER.tar.gz"
  [ -f "$f" ] || curl -fsSL -o "$f" "$RUNNER_URL"
  echo "$RUNNER_SHA256  $f" | sha256sum -c -
  tar -xzf "$f" -C "$work"
  rm -rf "$out/runner"; mv "$work/$RUNNER/files" "$out/runner"
  ;;
shaders)
  mkdir -p "$out/shaders"; cp "$here"/assets/shaders/*.fx "$out/shaders/"
  ;;
assets)
  rm -rf "$out/art" "$out/fonts"; cp -r "$here/assets/art" "$out/art"; cp -r "$here/assets/fonts" "$out/fonts"
  ;;
*) echo "unknown part $part" >&2; exit 2;;
esac; done
echo "components ready in $out"
