#!/usr/bin/env bash
set -e

echo "==> Сборка Rust проекта..."
cargo build --release

echo "==> Подготовка AppDir..."
rm -rf AppDir
mkdir -p AppDir/usr/bin

cp target/release/mcshell AppDir/usr/bin/
cp icon.png AppDir/mcshell.png
cp icon.png AppDir/.DirIcon

cat << 'EOF' > AppDir/mcshell.desktop
[Desktop Entry]
Name=MCShell
Type=Application
Exec=mcshell
Icon=mcshell
Terminal=true
Categories=Game;Utility;
EOF

cat << 'EOF' > AppDir/AppRun
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/usr/bin/mcshell" "$@"
EOF

chmod +x AppDir/AppRun

echo "==> Упаковка в AppImage..."
appimagetool AppDir MCShell-x86_64.AppImage

echo "==> Готово! Создан файл MCShell-x86_64.AppImage"