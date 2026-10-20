#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;
use adw::prelude::*;

use crate::ui::theme::ThemeManager;

pub struct CommandPalette {
    dialog: adw::Dialog,
    entry: gtk::SearchEntry,
    list_box: gtk::ListBox,
    theme_manager: Rc<ThemeManager>,
}

#[derive(Clone)]
struct PaletteAction {
    title: &'static str,
    category: &'static str,
    shortcut: Option<&'static str>,
    id: &'static str,
}

const ACTIONS: &[PaletteAction] = &[
    PaletteAction {
        title: "Compose New Message",
        category: "Mail",
        shortcut: Some("c"),
        id: "compose",
    },
    PaletteAction {
        title: "Sync Mailboxes (mbsync)",
        category: "Mail",
        shortcut: Some("Ctrl+R"),
        id: "sync",
    },
    PaletteAction {
        title: "Go to Inbox",
        category: "Navigation",
        shortcut: Some("g i"),
        id: "nav_inbox",
    },
    PaletteAction {
        title: "Go to Feed & Newsletters",
        category: "Navigation",
        shortcut: None,
        id: "nav_feed",
    },
    PaletteAction {
        title: "Go to Sent",
        category: "Navigation",
        shortcut: Some("g s"),
        id: "nav_sent",
    },
    PaletteAction {
        title: "Go to Archive",
        category: "Navigation",
        shortcut: Some("g a"),
        id: "nav_archive",
    },
    PaletteAction {
        title: "Archive Selected Message",
        category: "Message",
        shortcut: Some("e"),
        id: "archive",
    },
    PaletteAction {
        title: "Delete Selected Message",
        category: "Message",
        shortcut: Some("#"),
        id: "trash",
    },
    PaletteAction {
        title: "Toggle Star / Flag",
        category: "Message",
        shortcut: Some("s"),
        id: "star",
    },
    PaletteAction {
        title: "Theme: Attic Olive (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_attic_olive",
    },
    PaletteAction {
        title: "Theme: Corinthian Bronze (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_corinthian_bronze",
    },
    PaletteAction {
        title: "Theme: Catppuccin Mocha",
        category: "Appearance",
        shortcut: None,
        id: "theme_catppuccin",
    },
    PaletteAction {
        title: "Theme: Everforest",
        category: "Appearance",
        shortcut: None,
        id: "theme_everforest",
    },
    PaletteAction {
        title: "Theme: Tokyo Night",
        category: "Appearance",
        shortcut: None,
        id: "theme_tokyo_night",
    },
    PaletteAction {
        title: "Theme: Follow Noctalia Palette",
        category: "Appearance",
        shortcut: None,
        id: "theme_noctalia",
    },
    PaletteAction {
        title: "Toggle Subtle Accent Mode",
        category: "Appearance",
        shortcut: None,
        id: "toggle_subtle",
    },
];

impl CommandPalette {
    pub fn new(theme_manager: Rc<ThemeManager>) -> Self {
        let dialog = adw::Dialog::builder()
            .title("Command Palette")
            .content_width(560)
            .content_height(420)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
        vbox.set_margin_start(16);
        vbox.set_margin_end(16);
        vbox.set_margin_top(14);
        vbox.set_margin_bottom(14);

        let entry = gtk::SearchEntry::builder()
            .placeholder_text("Type a command, folder, or theme... (Ctrl+K)")
            .build();
        entry.add_css_class("iris-command-search");
        vbox.append(&entry);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        list_box.add_css_class("boxed-list");

        populate_actions(&list_box, "", &theme_manager, &dialog);

        let list_box_clone = list_box.clone();
        let tm_clone = theme_manager.clone();
        let dialog_clone = dialog.clone();
        entry.connect_search_changed(move |e| {
            let query = e.text().to_string();
            populate_actions(&list_box_clone, &query, &tm_clone, &dialog_clone);
        });

        scrolled.set_child(Some(&list_box));
        vbox.append(&scrolled);

        dialog.set_child(Some(&vbox));

        Self {
            dialog,
            entry,
            list_box,
            theme_manager,
        }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.entry.set_text("");
        self.dialog.present(Some(parent));
        self.entry.grab_focus();
    }
}

fn populate_actions(
    list_box: &gtk::ListBox,
    query: &str,
    theme_manager: &Rc<ThemeManager>,
    dialog: &adw::Dialog,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let q = query.trim().to_lowercase();

    for action in ACTIONS {
        if !q.is_empty()
            && !action.title.to_lowercase().contains(&q)
            && !action.category.to_lowercase().contains(&q)
        {
            continue;
        }

        let row = gtk::ListBoxRow::new();
        row.add_css_class("iris-palette-row");

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        let cat_label = gtk::Label::builder()
            .label(action.category)
            .build();
        cat_label.add_css_class("caption");
        cat_label.add_css_class("dim-label");
        hbox.append(&cat_label);

        let title_label = gtk::Label::builder()
            .label(action.title)
            .xalign(0.0)
            .hexpand(true)
            .build();
        title_label.add_css_class("body");
        hbox.append(&title_label);

        if let Some(shortcut) = action.shortcut {
            let sc_label = gtk::Label::builder().label(shortcut).build();
            sc_label.add_css_class("caption");
            sc_label.add_css_class("dim-label");
            hbox.append(&sc_label);
        }

        row.set_child(Some(&hbox));

        let id = action.id;
        let tm = theme_manager.clone();
        let diag = dialog.clone();
        row.connect_activate(move |_| {
            execute_action(id, &tm);
            diag.close();
        });

        list_box.append(&row);
    }
}

fn execute_action(id: &str, theme_manager: &Rc<ThemeManager>) {
    match id {
        "theme_attic_olive" => theme_manager.set_theme("attic-olive"),
        "theme_corinthian_bronze" => theme_manager.set_theme("corinthian-bronze"),
        "theme_catppuccin" => theme_manager.set_theme("catppuccin"),
        "theme_everforest" => theme_manager.set_theme("everforest"),
        "theme_tokyo_night" => theme_manager.set_theme("tokyo-night"),
        "theme_noctalia" => theme_manager.set_follow_noctalia(true),
        "toggle_subtle" => {
            let current = *theme_manager.subtle_mode.borrow();
            theme_manager.set_subtle_mode(!current);
        }
        "compose" => {
            tracing::info!("Triggered Compose from Command Palette");
        }
        "sync" => {
            tracing::info!("Triggered Sync from Command Palette");
        }
        other => {
            tracing::info!("Executed palette action: {}", other);
        }
    }
}
