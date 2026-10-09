#!/usr/bin/env bash
set -e

APPIMAGE="${1:-MCShell-x86_64.AppImage}"
ICON_SRC="${2:-icon.png}"

if [ ! -f "$APPIMAGE" ]; then
    echo "Не найден $APPIMAGE. Сначала запусти build-appimage.sh, либо укажи путь: ./install-appimage.sh /путь/к/файлу.AppImage"
    exit 1
fi

if [ ! -f "$ICON_SRC" ]; then
    echo "Не найден $ICON_SRC (иконка)."
    exit 1
fi

BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
DESKTOP_DIR="$HOME/.local/share/applications"

mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR"

echo "==> Копирую AppImage в $BIN_DIR..."
cp "$APPIMAGE" "$BIN_DIR/MCShell.AppImage"
chmod +x "$BIN_DIR/MCShell.AppImage"

echo "==> Копирую иконку в $ICON_DIR..."
cp "$ICON_SRC" "$ICON_DIR/mcshell.png"

echo "==> Создаю ярлык меню..."
cat << EOF > "$DESKTOP_DIR/mcshell.desktop"
[Desktop Entry]
Name=MCShell
Type=Application
Exec=$BIN_DIR/MCShell.AppImage
Icon=mcshell
Terminal=true
Categories=Game;Utility;
EOF

chmod +x "$DESKTOP_DIR/mcshell.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR"
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "==> Готово! MCShell должен появиться в меню приложений."
echo "Если не появился сразу — перелогинься или перезапусти плазмоид меню."