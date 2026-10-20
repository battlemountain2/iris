# Iris ✉️🌈

**An ultra-fast, liquid-glass email client for Arch Linux & Hyprland — companion to [Hermes](https://github.com/battlemountain2/hermes).**

[![License: GPL-3.0](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Language-Rust_2024-orange.svg)](https://www.rust-lang.org/)
[![GTK4 & Libadwaita](https://img.shields.io/badge/UI-GTK4_&_Libadwaita-purple.svg)](https://gtk.org/)
[![Wayland](https://img.shields.io/badge/Display-Wayland_&_Hyprland-blue.svg)](https://hyprland.org/)

Iris combines the elegance, inset-grouped hierarchy, and gesture fluidity of the iOS/macOS Mail client with the relentless speed and modal efficiency of modern Unix mail (`notmuch` + `maildir`).

---

## ✨ Features

- 🪟 **Liquid Glass & Acrylic Blur**: Designed natively for Hyprland's dual-kawase blur shader with subtle translucency and responsive folding (`AdwNavigationSplitView`).
- 🎨 **Noctalia & Linux Theming**:
  - Live auto-sync with Noctalia shell palettes (`~/.config/gtk-4.0/noctalia.css`).
  - **Subtle Mode**: Translucent glass background with refined, tasteful accent highlights.
  - Built-in palettes: Catppuccin, Everforest, Rosé Pine, Tokyo Night, Gruvbox, and Nord.
- ⚡ **Notmuch & Maildir Engine**:
  - Sub-10ms full-text search across 100,000+ emails.
  - Full support for multiple Gmail accounts (via `mbsync` XOAUTH2 or `lieer`) and Apple iCloud (App-Specific Passwords).
  - Two-way flag synchronization (Read, Flagged) and physical file moves for Archive and Trash.
- 🛡️ **Mail Privacy Protection & Sandboxing**:
  - Tracking pixels and remote images blocked by default with one-click override.
  - Bounded HTML rendering and attachment previews inside an unprivileged Bubblewrap (`bwrap`) sandbox.
- ⌨️ **Keyboard & Gesture First**:
  - **Command Palette (`Ctrl+K`)**: Instant fuzzy jump to folders, accounts, and batch tagging.
  - **Vim Navigation**: `j`/`k` move, `e` archive, `#` trash, `c` compose, `r` reply, `/` search.
  - **Two-Finger Trackpad Swipes**: Swipe-to-read and swipe-to-archive gestures optimized for MacBooks running Omarchy / Arch.
- ⏱️ **10-Second Undo Send**:
  - High-visibility `AdwToast` countdown with one-click undo before SMTP dispatch.

---

## 🏛️ Sibling Architecture: Iris & Hermes

Iris is built to work hand-in-hand with **Hermes** (file manager, media hub, and music player):
- **Drag & Drop**: Drag documents, audio stems, and folders directly from Hermes into Iris's compose window.
- **Shared Tokens**: Shared palette definitions, Noctalia listener architecture, and typography standards (Maple Mono, Inter, JetBrains Mono).
- **Bubblewrap Security**: Reuses Hermes's isolated preview architecture for zero-trust rendering of attachments.
