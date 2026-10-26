#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

echo "📦 Installing Iris to user environment..."

# 1. Build release binary if missing or install existing release
if [ ! -f "$REPO_DIR/target/release/iris" ]; then
    echo "🔨 Building release binary..."
    cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"
fi

mkdir -p "$HOME/.local/bin"
cp "$REPO_DIR/target/release/iris" "$HOME/.local/bin/iris"
chmod +x "$HOME/.local/bin/iris"
echo "✅ Installed binary to $HOME/.local/bin/iris"

# 2. Install desktop entry
mkdir -p "$HOME/.local/share/applications"
cp "$REPO_DIR/data/io.github.battlemountain2.Iris.desktop" "$HOME/.local/share/applications/"
echo "✅ Installed desktop entry to $HOME/.local/share/applications/io.github.battlemountain2.Iris.desktop"

# 3. Install hicolor icons
for size in 32x32 48x48 64x64 128x128 512x512; do
    target_dir="$HOME/.local/share/icons/hicolor/$size/apps"
    mkdir -p "$target_dir"
    if [ -f "$REPO_DIR/data/icons/hicolor/$size/apps/io.github.battlemountain2.Iris.png" ]; then
        cp "$REPO_DIR/data/icons/hicolor/$size/apps/io.github.battlemountain2.Iris.png" "$target_dir/"
    fi
done
echo "✅ Installed Attic Olive application icons"

# 4. Update desktop and icon caches if utilities are available
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "✨ Iris is ready! Launch it from your application menu or run 'iris'."
