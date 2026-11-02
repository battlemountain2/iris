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
    on_action: Rc<dyn Fn(&str)>,
}

#[derive(Clone)]
struct PaletteAction {
    title: &'static str,
    category: &'static str,
    shortcut: Option<&'static str>,
    id: &'static str,
    icon: &'static str,
}

const ACTIONS: &[PaletteAction] = &[
    PaletteAction {
        title: "Add Email Account...",
        category: "Accounts",
        shortcut: Some("A"),
        id: "add_account",
        icon: "list-add-symbolic",
    },
    PaletteAction {
        title: "Compose New Message",
        category: "Mail",
        shortcut: Some("c"),
        id: "compose",
        icon: "mail-message-new-symbolic",
    },
    PaletteAction {
        title: "Sync Mailboxes (mbsync)",
        category: "Mail",
        shortcut: Some("Ctrl+R"),
        id: "sync",
        icon: "view-refresh-symbolic",
    },
    PaletteAction {
        title: "Go to Inbox",
        category: "Navigation",
        shortcut: Some("g i"),
        id: "nav_inbox",
        icon: "mail-send-receive-symbolic",
    },
    PaletteAction {
        title: "Go to Feed & Newsletters",
        category: "Navigation",
        shortcut: None,
        id: "nav_feed",
        icon: "application-rss+xml-symbolic",
    },
    PaletteAction {
        title: "Go to Sent",
        category: "Navigation",
        shortcut: Some("g s"),
        id: "nav_sent",
        icon: "mail-send-symbolic",
    },
    PaletteAction {
        title: "Go to Archive",
        category: "Navigation",
        shortcut: Some("g a"),
        id: "nav_archive",
        icon: "package-x-generic-symbolic",
    },
    PaletteAction {
        title: "Go to Drafts",
        category: "Navigation",
        shortcut: Some("g d"),
        id: "nav_drafts",
        icon: "document-edit-symbolic",
    },
    PaletteAction {
        title: "Go to Trash",
        category: "Navigation",
        shortcut: Some("g t"),
        id: "nav_trash",
        icon: "user-trash-symbolic",
    },
    PaletteAction {
        title: "Keyboard Shortcuts Cheatsheet",
        category: "Help",
        shortcut: Some("?"),
        id: "shortcuts",
        icon: "help-about-symbolic",
    },
    PaletteAction {
        title: "Preferences & Accounts",
        category: "Settings",
        shortcut: Some("Ctrl+,"),
        id: "settings",
        icon: "emblem-system-symbolic",
    },
    PaletteAction {
        title: "Reply to Sender",
        category: "Message",
        shortcut: Some("r"),
        id: "reply",
        icon: "mail-reply-sender-symbolic",
    },
    PaletteAction {
        title: "Reply All",
        category: "Message",
        shortcut: Some("a"),
        id: "reply_all",
        icon: "mail-reply-all-symbolic",
    },
    PaletteAction {
        title: "Forward Message",
        category: "Message",
        shortcut: Some("f"),
        id: "forward",
        icon: "mail-forward-symbolic",
    },
    PaletteAction {
        title: "Select All Visible Conversations",
        category: "Message",
        shortcut: Some("Ctrl+A"),
        id: "select_all",
        icon: "object-select-symbolic",
    },
    PaletteAction {
        title: "Clear Multi-Selection",
        category: "Message",
        shortcut: Some("Esc"),
        id: "clear_selection",
        icon: "window-close-symbolic",
    },
    PaletteAction {
        title: "Archive Selected Message",
        category: "Message",
        shortcut: Some("e"),
        id: "archive",
        icon: "package-x-generic-symbolic",
    },
    PaletteAction {
        title: "Delete Selected Message",
        category: "Message",
        shortcut: Some("#"),
        id: "trash",
        icon: "user-trash-symbolic",
    },
    PaletteAction {
        title: "Move Conversation to Folder...",
        category: "Message",
        shortcut: Some("v"),
        id: "move_folder",
        icon: "folder-symbolic",
    },
    PaletteAction {
        title: "Add / Edit Labels & Tags...",
        category: "Message",
        shortcut: Some("l"),
        id: "tag",
        icon: "user-bookmarks-symbolic",
    },
    PaletteAction {
        title: "Create New Maildir Folder...",
        category: "Mailbox",
        shortcut: None,
        id: "create_folder",
        icon: "folder-new-symbolic",
    },
    PaletteAction {
        title: "Empty Trash",
        category: "Mailbox",
        shortcut: None,
        id: "empty_trash",
        icon: "user-trash-symbolic",
    },
    PaletteAction {
        title: "Toggle Star / Flag",
        category: "Message",
        shortcut: Some("s"),
        id: "star",
        icon: "starred-symbolic",
    },
    PaletteAction {
        title: "Theme: iOS Dark (Apple Default)",
        category: "Appearance",
        shortcut: None,
        id: "theme_iris_dark",
        icon: "weather-clear-night-symbolic",
    },
    PaletteAction {
        title: "Theme: Attic Olive (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_attic_olive",
        icon: "emblem-favorite-symbolic",
    },
    PaletteAction {
        title: "Theme: Spartan Crimson (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_spartan_crimson",
        icon: "weather-clear-night-symbolic",
    },
    PaletteAction {
        title: "Theme: Byzantine Bronze (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_byzantine_bronze",
        icon: "software-properties-symbolic",
    },
    PaletteAction {
        title: "Theme: Attic Amber (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_attic_amber",
        icon: "weather-clear-symbolic",
    },
    PaletteAction {
        title: "Theme: Corinthian Bronze (Greek)",
        category: "Appearance",
        shortcut: None,
        id: "theme_corinthian_bronze",
        icon: "weather-overcast-symbolic",
    },
    PaletteAction {
        title: "Theme: Catppuccin Mocha",
        category: "Appearance",
        shortcut: None,
        id: "theme_catppuccin",
        icon: "applications-graphics-symbolic",
    },
    PaletteAction {
        title: "Theme: Everforest",
        category: "Appearance",
        shortcut: None,
        id: "theme_everforest",
        icon: "starred-symbolic",
    },
    PaletteAction {
        title: "Theme: Tokyo Night",
        category: "Appearance",
        shortcut: None,
        id: "theme_tokyo_night",
        icon: "night-light-symbolic",
    },
    PaletteAction {
        title: "Theme: Follow Noctalia Palette",
        category: "Appearance",
        shortcut: None,
        id: "theme_noctalia",
        icon: "applications-graphics-symbolic",
    },
    PaletteAction {
        title: "Toggle Subtle Accent Mode",
        category: "Appearance",
        shortcut: None,
        id: "toggle_subtle",
        icon: "weather-few-clouds-symbolic",
    },
];

impl CommandPalette {
    pub fn new(theme_manager: Rc<ThemeManager>) -> Self {
        Self::with_action_handler(theme_manager, |_| {})
    }

    pub fn with_action_handler<F>(theme_manager: Rc<ThemeManager>, on_action: F) -> Self
    where
        F: Fn(&str) + 'static,
    {
        let dialog = adw::Dialog::builder()
            .title("Command Palette")
            .content_width(580)
            .content_height(440)
            .build();
        dialog.add_css_class("iris-dialog");
        dialog.add_css_class("iris-palette-dialog");

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

        let action_rc: Rc<dyn Fn(&str)> = Rc::new(on_action);
        populate_actions(&list_box, "", &theme_manager, &action_rc, &dialog);

        let list_box_clone = list_box.clone();
        let tm_clone = theme_manager.clone();
        let act_clone = action_rc.clone();
        let dialog_clone = dialog.clone();
        entry.connect_search_changed(move |e| {
            let query = e.text().to_string();
            populate_actions(&list_box_clone, &query, &tm_clone, &act_clone, &dialog_clone);
        });

        scrolled.set_child(Some(&list_box));
        vbox.append(&scrolled);

        dialog.set_child(Some(&vbox));

        Self {
            dialog,
            entry,
            list_box,
            theme_manager,
            on_action: action_rc,
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
    on_action: &Rc<dyn Fn(&str)>,
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

        let icon = gtk::Image::from_icon_name(action.icon);
        icon.set_icon_size(gtk::IconSize::Normal);
        icon.add_css_class("iris-palette-icon");
        hbox.append(&icon);

        let title_label = gtk::Label::builder()
            .label(action.title)
            .xalign(0.0)
            .hexpand(true)
            .build();
        title_label.add_css_class("body");
        hbox.append(&title_label);

        let cat_label = gtk::Label::builder()
            .label(action.category)
            .build();
        cat_label.add_css_class("caption");
        cat_label.add_css_class("iris-palette-category");
        hbox.append(&cat_label);

        if let Some(shortcut) = action.shortcut {
            let sc_label = gtk::Label::builder().label(shortcut).build();
            sc_label.add_css_class("iris-key-badge");
            hbox.append(&sc_label);
        }

        row.set_child(Some(&hbox));

        let id = action.id;
        let tm = theme_manager.clone();
        let act = on_action.clone();
        let diag = dialog.clone();
        row.connect_activate(move |_| {
            execute_action(id, &tm, &act);
            diag.close();
        });

        list_box.append(&row);
    }
}

fn execute_action(
    id: &str,
    theme_manager: &Rc<ThemeManager>,
    on_action: &Rc<dyn Fn(&str)>,
) {
    match id {
        "theme_iris_dark" => theme_manager.set_theme("iris-dark"),
        "theme_attic_olive" => theme_manager.set_theme("attic-olive"),
        "theme_spartan_crimson" => theme_manager.set_theme("spartan-crimson"),
        "theme_byzantine_bronze" => theme_manager.set_theme("byzantine-bronze"),
        "theme_attic_amber" => theme_manager.set_theme("attic-amber"),
        "theme_corinthian_bronze" => theme_manager.set_theme("corinthian-bronze"),
        "theme_catppuccin" => theme_manager.set_theme("catppuccin"),
        "theme_everforest" => theme_manager.set_theme("everforest"),
        "theme_tokyo_night" => theme_manager.set_theme("tokyo-night"),
        "theme_noctalia" => theme_manager.set_follow_noctalia(true),
        "toggle_subtle" => {
            let current = *theme_manager.subtle_mode.borrow();
            theme_manager.set_subtle_mode(!current);
        }
        action => {
            (on_action)(action);
        }
    }
}
