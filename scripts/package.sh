#!/bin/sh
# Assembles a release package from the built launcher and the components.
#
#   scripts/package.sh linux     dist/pkg/InitialDLauncher-<version>-linux-x86_64.tar.gz
#   scripts/package.sh windows   dist/pkg/InitialDLauncher-<version>-windows-x86_64.zip
#
# Built pieces (the release workflow puts them there):
#   linux    dist/linux/initiald-launcher, dist/linux/lib/libSDL2-2.0.so.0
#   windows  dist/windows/initiald-launcher.exe, dist/windows/SDL2.dll
# The components are built or fetched here with scripts/build-components.sh
# (windowsarcadeloader from source with patches/wal, GE-Proton and
# winetricks by SHA-256 on Linux, the CRT effect, the art and fonts).
#
# lindbergh-loader (CC BY-NC-SA) is never packaged: the Linux package
# carries the script, its container recipe and patches/lindbergh, and the
# launcher builds it on the player's machine on first use. No game file is
# ever packaged.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd -P)
os=${1:-linux}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)
die() { echo "package: $*" >&2; exit 1; }

case $os in
linux) name=InitialDLauncher-$version-linux-x86_64; bin=$root/dist/linux ;;
windows) name=InitialDLauncher-$version-windows-x86_64; bin=$root/dist/windows ;;
*) die "unknown target '$os' (linux or windows)" ;;
esac

out=$root/dist/pkg/$name
rm -rf "$out"
mkdir -p "$out/components" "$out/licenses"

if [ "$os" = linux ]; then
	[ -f "$bin/initiald-launcher" ] || die "missing $bin/initiald-launcher - build it first"
	[ -f "$bin/lib/libSDL2-2.0.so.0" ] || die "missing $bin/lib/libSDL2-2.0.so.0"
	install -m 755 "$bin/initiald-launcher" "$out/initiald-launcher"
	mkdir -p "$out/lib"
	cp "$bin"/lib/* "$out/lib/"
	[ -f "$bin/SDL2-copyright.txt" ] && cp "$bin/SDL2-copyright.txt" "$out/licenses/"
	"$root/scripts/build-components.sh" "$out/components" wal runner winetricks shaders assets
	rm -rf "$out/components/.build"
	# the first-use build of lindbergh-loader (Initial D 4 and 5)
	mkdir -p "$out/scripts" "$out/patches"
	install -m 755 "$root/scripts/build-components.sh" "$root/scripts/lindbergh-in-container.sh" "$out/scripts/"
	cp -r "$root/patches/lindbergh" "$out/patches/lindbergh"
	cp "$out/components/wal/LICENSE" "$out/licenses/windowsarcadeloader-LICENSE.txt"
else
	[ -f "$bin/initiald-launcher.exe" ] || die "missing $bin/initiald-launcher.exe - build it first"
	cp "$bin/initiald-launcher.exe" "$out/"
	cp "$bin"/*.dll "$out/"
	for f in "$bin"/*-LICENSE.txt; do [ -f "$f" ] && cp "$f" "$out/licenses/"; done
	"$root/scripts/build-components.sh" "$out/components" wal-windows flycast shaders assets
	rm -rf "$out/components/.build"
	cp "$out/components/wal/LICENSE" "$out/licenses/windowsarcadeloader-LICENSE.txt"
fi

cp "$root/packaging/README-player.txt" "$out/README.txt"
cp "$root/LICENSE" "$out/licenses/LICENSE-GPL-3.0.txt"
cp "$root/THIRD_PARTY.md" "$out/licenses/"
cp "$root/assets/fonts/OFL.txt" "$out/licenses/Barlow-OFL.txt"

cd "$root/dist/pkg"
if [ "$os" = linux ]; then
	tar -czf "$name.tar.gz" "$name"
	echo "package: dist/pkg/$name.tar.gz"
else
	rm -f "$name.zip"
	zip -qr "$name.zip" "$name"
	echo "package: dist/pkg/$name.zip"
fi
