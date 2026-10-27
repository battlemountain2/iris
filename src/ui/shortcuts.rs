#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;

#[derive(Clone, Copy)]
pub enum ShortcutKeys<'a> {
    Single(&'a str),
    Or(&'a [&'a str]),
    Chord(&'a [&'a str]),
}

pub struct ShortcutsDialog {
    dialog: adw::Dialog,
}

impl ShortcutsDialog {
    pub fn new() -> Self {
        let dialog = adw::Dialog::builder()
            .title("Keyboard Shortcuts")
            .content_width(620)
            .content_height(580)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.add_css_class("iris-shortcuts-dialog");

        // Headerbar with Title & Close button
        let headerbar = adw::HeaderBar::builder()
            .show_end_title_buttons(true)
            .build();
        vbox.append(&headerbar);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.set_margin_top(16);
        content.set_margin_bottom(24);

        // Subtitle / Hint
        let hint_label = gtk::Label::builder()
            .label("Liquid-Glass Keyboard Navigation & Triage Cheatsheet")
            .xalign(0.0)
            .build();
        hint_label.add_css_class("dim-label");
        hint_label.add_css_class("caption");
        content.append(&hint_label);

        // Section 1: Navigation
        content.append(&build_section("Navigation", &[
            ("Next conversation / message", ShortcutKeys::Or(&["j", "↓"])),
            ("Previous conversation / message", ShortcutKeys::Or(&["k", "↑"])),
            ("Go to All Inboxes", ShortcutKeys::Chord(&["g", "i"])),
            ("Go to Starred / Flagged", ShortcutKeys::Chord(&["g", "s"])),
            ("Go to Drafts", ShortcutKeys::Chord(&["g", "d"])),
            ("Go to Archive", ShortcutKeys::Chord(&["g", "a"])),
            ("Go to Trash", ShortcutKeys::Chord(&["g", "t"])),
            ("Go to Sent Mail", ShortcutKeys::Chord(&["g", "e"])),
            ("Go to Feed & Newsletters", ShortcutKeys::Chord(&["g", "f"])),
            ("Back / Clear list selection", ShortcutKeys::Or(&["Esc", "Backspace"])),
        ]));

        // Section 2: Triage
        content.append(&build_section("Triage & Organization", &[
            ("Select / multi-select conversation", ShortcutKeys::Single("x")),
            ("Select all conversations", ShortcutKeys::Single("Ctrl+A")),
            ("Archive conversation", ShortcutKeys::Or(&["e", "Swipe Right"])),
            ("Delete / Trash conversation", ShortcutKeys::Or(&["#", "Delete", "Swipe Left"])),
            ("Star / Flag conversation", ShortcutKeys::Single("s")),
            ("Mark as Read / Unread", ShortcutKeys::Single("u")),
            ("Undo last triage action", ShortcutKeys::Or(&["z", "Ctrl+Z"])),
        ]));

        // Section 3: Compose & Reading
        content.append(&build_section("Compose & Reading", &[
            ("New conversation / Compose", ShortcutKeys::Single("c")),
            ("Reply to sender", ShortcutKeys::Single("r")),
            ("Reply all recipients", ShortcutKeys::Single("a")),
            ("Forward conversation", ShortcutKeys::Single("f")),
            ("Send email (in compose)", ShortcutKeys::Single("Ctrl+Enter")),
            ("Attach files (in compose)", ShortcutKeys::Single("Ctrl+Shift+A")),
            ("Discard / Close compose dialog", ShortcutKeys::Single("Esc")),
        ]));

        // Section 4: Global & Search
        content.append(&build_section("Global & Search", &[
            ("Search messages & filter", ShortcutKeys::Single("/")),
            ("Command Palette", ShortcutKeys::Or(&["Ctrl+K", "Space"])),
            ("Synchronize mailboxes (mbsync)", ShortcutKeys::Single("Ctrl+R")),
            ("Open Keyboard Shortcuts cheatsheet", ShortcutKeys::Single("?")),
        ]));

        scrolled.set_child(Some(&content));
        vbox.append(&scrolled);

        dialog.set_child(Some(&vbox));

        Self { dialog }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.dialog.present(Some(parent));
    }
}

fn build_section(title: &str, items: &[(&str, ShortcutKeys)]) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 6);

    let heading = gtk::Label::builder()
        .label(title)
        .xalign(0.0)
        .build();
    heading.add_css_class("iris-shortcuts-heading");
    section.append(&heading);

    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
    group.add_css_class("iris-shortcuts-group");

    for (desc, keys) in items {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.add_css_class("iris-shortcut-row");

        let desc_label = gtk::Label::builder()
            .label(*desc)
            .xalign(0.0)
            .hexpand(true)
            .build();
        row.append(&desc_label);

        let keys_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        match keys {
            ShortcutKeys::Single(k) => {
                keys_box.append(&make_key_badge(k));
            }
            ShortcutKeys::Or(list) => {
                for (idx, k) in list.iter().enumerate() {
                    if idx > 0 {
                        let sep = gtk::Label::new(Some("or"));
                        sep.add_css_class("iris-key-sep");
                        keys_box.append(&sep);
                    }
                    keys_box.append(&make_key_badge(k));
                }
            }
            ShortcutKeys::Chord(list) => {
                for (idx, k) in list.iter().enumerate() {
                    if idx > 0 {
                        let sep = gtk::Label::new(Some("then"));
                        sep.add_css_class("iris-key-sep");
                        keys_box.append(&sep);
                    }
                    keys_box.append(&make_key_badge(k));
                }
            }
        }

        row.append(&keys_box);
        group.append(&row);
    }

    section.append(&group);
    section
}

fn make_key_badge(key_str: &str) -> gtk::Label {
    let badge = gtk::Label::builder()
        .label(key_str)
        .build();
    badge.add_css_class("iris-key-badge");
    badge
}
