# Iris Agent Guidelines & Architecture Manual

## Overview
**Iris** is an ultra-fast, keyboard-first, liquid-glass email client engineered for Arch Linux and modern Wayland compositors (specifically **Hyprland** and **Omarchy**). It serves as the sister application and companion to **[[Hermes]]** (`/home/bry/hermes`).

Designed around the best qualities of the iOS/macOS Mail interface (minimalism, thread cards, inset grouped rows, gestures), Iris combines them with the speed, modal control, and privacy of the Unix mail ecosystem:
- **Backend**: `notmuch` (fast indexing & search) + `maildir` (multi-account local storage) + `mbsync` (sync engine).
- **Frontend**: Rust (2024 edition) + GTK4 + Libadwaita.
- **Visuals**: Hyprland dual-kawase acrylic blur, subtle liquid glass translucency, live **Noctalia** theme following, and built-in palettes (Catppuccin, Everforest, Rosé Pine, Tokyo Night, etc.) with both **Subtle (Accent Only)** and **Full (Surface)** modes.

---

## Workspace & Sibling Navigation
- **Hermes (Sibling)**: `/home/bry/hermes` — File manager, media hub, and music player.
- **Iris (This Project)**: `/home/bry/iris` — Email client and communications hub.
- **Obsidian Vault**: `projects/Iris - Dev Notes.md` & `projects/Hermes - Dev Notes.md`.

---

## Core Tenets & Engineering Rules

### 1. Performance Obsession
- Zero main-thread blocking. All Notmuch searches, MIME decodes, and Maildir file moves MUST happen on bounded background workers (`gio::Task` or `tokio`/threadpools).
- Virtualized lists (`GtkListView` / `AdwNavigationSplitView`) so mailboxes with 100,000+ messages open instantaneously with zero allocation spikes.
- Zero idle CPU when no operations or animations are active.

### 2. Privacy & Sandbox Security
- **Mail Privacy Protection**: Remote images and tracking pixels MUST be blocked by default.
- **Bubblewrap (`bwrap`) Sandboxing**: Render untrusted HTML email bodies and parse unknown attachments inside an isolated unprivileged bubblewrap sandbox (inspired by the Hermes media preview sandbox).

### 3. Theming & Aesthetics
- Support the standard `ThemeTokens` format matching Hermes (`azure-glow`, `catppuccin`, `everforest`, `rose-pine`, `tokyo-night`, `graphite`, `paper`, `midnight`).
- Dynamic **Noctalia** auto-refresh: Monitor `~/.config/gtk-4.0/noctalia.css` with `gio::FileMonitor`, debouncing file replacements and updating CSS providers live without restarting.
- **Subtle Mode**: In subtle mode, keep the translucent liquid-glass window background and inject theme colors strictly into accents (`@theme_accent`), active thread selection indicators, unread badges, and focus rings.

### 4. Gestures & Keyboard First
- Full Vim modal bindings (`j`/`k` navigate, `e` archive, `#` trash, `c` compose, `r` reply, `/` search).
- Trackpad gesture parity: 2-finger swipe right to Mark Read, swipe left to Archive/Trash (especially optimized for Omarchy running on Apple MacBooks).
- Command Palette (`Ctrl+K`): Omnibar for jumping to accounts, folders, and executing commands.
- 10-Second Undo Send toast (`AdwToast`) before firing SMTP transactions.
