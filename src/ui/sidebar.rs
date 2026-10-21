#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::collections::HashMap;
use adw::prelude::*;

pub struct SidebarView {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
    pub sync_btn: gtk::Button,
    pub badges: RefCell<HashMap<String, gtk::Label>>,
    pub rows: RefCell<HashMap<String, gtk::ListBoxRow>>,
}

impl SidebarView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-sidebar");

        let header = adw::HeaderBar::new();
        header.set_show_end_title_buttons(false);

        let title = adw::WindowTitle::builder()
            .title("Mailboxes")
            .subtitle("3 accounts")
            .build();
        header.set_title_widget(Some(&title));

        // Sync button in header
        let sync_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Sync Mailboxes (mbsync)")
            .build();
        sync_btn.add_css_class("flat");
        header.pack_end(&sync_btn);

        container.append(&header);

        // Scrolled window for mailboxes
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        list_box.add_css_class("navigation-sidebar");

        let mut badges_map = HashMap::new();
        let mut rows_map = HashMap::new();
        populate_default_mailboxes(&list_box, &mut badges_map, &mut rows_map);

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self {
            container,
            list_box,
            sync_btn,
            badges: RefCell::new(badges_map),
            rows: RefCell::new(rows_map),
        }
    }

    /// Updates the numeric badge count for a specific mailbox/folder.
    pub fn update_badge(&self, folder_id: &str, count: u32) {
        if let Some(badge) = self.badges.borrow().get(folder_id) {
            if count > 0 {
                badge.set_text(&count.to_string());
                badge.set_visible(true);
            } else {
                badge.set_visible(false);
            }
        }
    }

    /// Selects a mailbox row programmatically.
    pub fn select_folder(&self, folder_id: &str) {
        if let Some(row) = self.rows.borrow().get(folder_id) {
            self.list_box.select_row(Some(row));
        }
    }

    /// Returns the folder ID associated with a clicked row.
    pub fn get_row_id(&self, row: &gtk::ListBoxRow) -> Option<String> {
        let name = row.widget_name().to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }
}

fn populate_default_mailboxes(
    list_box: &gtk::ListBox,
    badges: &mut HashMap<String, gtk::Label>,
    rows: &mut HashMap<String, gtk::ListBoxRow>,
) {
    // Section 1: Inboxes
    let inboxes_label = gtk::Label::builder()
        .label("INBOXES")
        .xalign(0.0)
        .margin_start(16)
        .margin_top(12)
        .margin_bottom(4)
        .build();
    inboxes_label.add_css_class("dim-label");
    inboxes_label.add_css_class("caption");

    let header_row1 = gtk::ListBoxRow::new();
    header_row1.set_selectable(false);
    header_row1.set_activatable(false);
    header_row1.set_child(Some(&inboxes_label));
    list_box.append(&header_row1);

    add_row(list_box, badges, rows, "inbox", "All Inboxes", "mail-folder-inbox-symbolic", Some(3));
    add_row(list_box, badges, rows, "account:demo", "Gmail (Personal)", "mail-inbox-symbolic", Some(2));
    add_row(list_box, badges, rows, "account:work", "Gmail (Work)", "mail-inbox-symbolic", None);
    add_row(list_box, badges, rows, "account:icloud", "iCloud", "mail-inbox-symbolic", Some(1));

    // Section 2: Smart Views
    let views_label = gtk::Label::builder()
        .label("VIEWS")
        .xalign(0.0)
        .margin_start(16)
        .margin_top(16)
        .margin_bottom(4)
        .build();
    views_label.add_css_class("dim-label");
    views_label.add_css_class("caption");

    let header_row2 = gtk::ListBoxRow::new();
    header_row2.set_selectable(false);
    header_row2.set_activatable(false);
    header_row2.set_child(Some(&views_label));
    list_box.append(&header_row2);

    add_row(list_box, badges, rows, "view:feed", "Feed & Newsletters", "newspaper-symbolic", None);
    add_row(list_box, badges, rows, "view:flagged", "Flagged", "starred-symbolic", None);
    add_row(list_box, badges, rows, "folder:sent", "Sent", "mail-sent-symbolic", None);
    add_row(list_box, badges, rows, "folder:archive", "Archive", "mail-archive-symbolic", None);
    add_row(list_box, badges, rows, "folder:trash", "Trash", "user-trash-symbolic", None);
}

fn add_row(
    list_box: &gtk::ListBox,
    badges: &mut HashMap<String, gtk::Label>,
    rows: &mut HashMap<String, gtk::ListBoxRow>,
    id: &str,
    title: &str,
    icon_name: &str,
    unread: Option<u32>,
) {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(id);

    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_icon_size(gtk::IconSize::Normal);
    hbox.append(&icon);

    let label = gtk::Label::builder()
        .label(title)
        .xalign(0.0)
        .hexpand(true)
        .build();
    hbox.append(&label);

    let badge = gtk::Label::builder()
        .label(unread.map(|c| c.to_string()).unwrap_or_default())
        .visible(unread.is_some_and(|c| c > 0))
        .build();
    badge.add_css_class("badge");
    badge.add_css_class("numeric");
    hbox.append(&badge);

    badges.insert(id.to_string(), badge);
    rows.insert(id.to_string(), row.clone());

    row.set_child(Some(&hbox));
    list_box.append(&row);
}
