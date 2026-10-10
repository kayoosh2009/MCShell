#!/usr/bin/env bash
# Сборка Windows-версии MCShell и копирование в Releases/
set -euo pipefail

cd "$(dirname "$0")"

TARGET="x86_64-pc-windows-gnu"
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
OUT_DIR="Releases"
NAME="mcshell-windows-v${VERSION}"

# Проверки окружения
if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "error: не найден mingw (x86_64-w64-mingw32-gcc)"
    echo "  Arch:   sudo pacman -S mingw-w64-gcc"
    echo "  Fedora: sudo dnf install mingw64-gcc"
    echo "  Debian: sudo apt install gcc-mingw-w64-x86-64"
    exit 1
fi

if ! rustup target list --installed | grep -q "^${TARGET}$"; then
    echo "==> добавляю target ${TARGET}"
    rustup target add "${TARGET}"
fi

echo "==> сборка v${VERSION} (${TARGET})"
cargo build --release --target "${TARGET}"

mkdir -p "${OUT_DIR}"
cp "target/${TARGET}/release/mcshell.exe" "${OUT_DIR}/${NAME}.exe"

# Архив, чтобы мессенджеры не блокировали .exe (нужен zip, иначе пропускаем)
if command -v zip >/dev/null 2>&1; then
    (cd "${OUT_DIR}" && rm -f "${NAME}.zip" && zip -q "${NAME}.zip" "${NAME}.exe")
    echo "==> ${OUT_DIR}/${NAME}.zip"
fi

echo "==> ${OUT_DIR}/${NAME}.exe ($(du -h "${OUT_DIR}/${NAME}.exe" | cut -f1))"