#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
version="${LINUXDRIFT_VERSION:-1.0.1}"
arch="$(dpkg --print-architecture)"
if [[ "${1:-}" != "--no-build" ]]; then
    cargo build --locked --release -p linuxdrift
fi
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
install -Dm755 target/release/linuxdrift "$stage/usr/bin/linuxdrift"
strip "$stage/usr/bin/linuxdrift"
install -Dm644 packaging/linuxdrift.desktop "$stage/usr/share/applications/linuxdrift.desktop"
install -Dm644 packaging/linuxdrift-config.desktop "$stage/usr/share/applications/linuxdrift-config.desktop"
install -Dm644 packaging/linuxdrift.svg "$stage/usr/share/icons/hicolor/scalable/apps/linuxdrift.svg"
install -Dm644 LICENSE "$stage/usr/share/doc/linuxdrift/copyright"
install -Dm644 README.md "$stage/usr/share/doc/linuxdrift/README.md"
for asset in docs/assets/*; do
    install -Dm644 "$asset" "$stage/usr/share/doc/linuxdrift/$asset"
done
install -Dm644 packaging/linuxdrift.xml "$stage/usr/share/xscreensaver/config/linuxdrift.xml"
install -Dm755 packaging/register-xscreensaver.py "$stage/usr/bin/linuxdrift-register-xscreensaver"
install -Dm644 packaging/linuxdrift-xscreensaver.desktop "$stage/etc/xdg/autostart/linuxdrift-xscreensaver.desktop"
mkdir -p "$stage/usr/libexec/xscreensaver"
ln -s /usr/bin/linuxdrift "$stage/usr/libexec/xscreensaver/linuxdrift"
mkdir -p "$stage/DEBIAN" dist
install -m755 packaging/postinst "$stage/DEBIAN/postinst"
# shlibdeps derives minimum library versions from the actual build host.
# Python/GTK are runtime dependencies for --config only.
mkdir -p "$stage/debian"
printf 'Source: linuxdrift\nSection: x11\nPriority: optional\nMaintainer: LinuxDrift contributors <noreply@localhost>\nStandards-Version: 4.6.2\n\nPackage: linuxdrift\nArchitecture: any\nDescription: Offline Linux screensaver\n' > "$stage/debian/control"
deps="$(cd "$stage" && dpkg-shlibdeps -O -eusr/bin/linuxdrift 2>/dev/null | sed -n 's/^shlibs:Depends=//p')"
if [[ -z "$deps" ]]; then
    echo "Could not determine shared-library dependencies; install dpkg-dev." >&2
    exit 1
fi
rm -r "$stage/debian"
cat > "$stage/DEBIAN/control" <<CONTROL
Package: linuxdrift
Version: $version
Section: x11
Priority: optional
Architecture: $arch
Maintainer: LinuxDrift contributors <noreply@localhost>
Depends: $deps, libvulkan1, libwayland-client0, libxkbcommon0, libxkbcommon-x11-0, libx11-6, libxcursor1, libxi6, libxrandr2, python3, python3-gi, gir1.2-gtk-4.0
Conflicts: linuxflux
Replaces: linuxflux
Recommends: mesa-vulkan-drivers, xscreensaver
Installed-Size: $(du -sk "$stage/usr" | cut -f1)
Homepage: https://github.com/spacecdr/LinuxDrift
Description: Linux Drift/Deriva screensaver for X11, Wayland and XScreenSaver
 LinuxDrift renders the Flux fluid animation fullscreen, without overlays.
 Based on Flux by Sander Melnikov, inspired by macOS Drift/Deriva.
 Includes persistent settings and native XScreenSaver integration.
CONTROL
out="dist/linuxdrift_${version}_${arch}.deb"
dpkg-deb --root-owner-group --build "$stage" "$out"
(cd dist && sha256sum "linuxdrift_${version}_${arch}.deb") > "$out.sha256"
printf 'Created %s\n' "$out"
