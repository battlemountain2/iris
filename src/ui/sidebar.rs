#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;

pub struct SidebarView {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
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

        populate_default_mailboxes(&list_box);

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self {
            container,
            list_box,
        }
    }
}

fn populate_default_mailboxes(list_box: &gtk::ListBox) {
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
    list_box.append(&inboxes_label);

    list_box.append(&create_mailbox_row("All Inboxes", "mail-folder-inbox-symbolic", Some(14)));
    list_box.append(&create_mailbox_row("Gmail (Personal)", "mail-inbox-symbolic", Some(5)));
    list_box.append(&create_mailbox_row("Gmail (Work)", "mail-inbox-symbolic", Some(8)));
    list_box.append(&create_mailbox_row("iCloud", "mail-inbox-symbolic", Some(1)));

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
    list_box.append(&views_label);

    list_box.append(&create_mailbox_row("Feed & Newsletters", "newspaper-symbolic", Some(23)));
    list_box.append(&create_mailbox_row("Flagged", "starred-symbolic", None));
    list_box.append(&create_mailbox_row("Sent", "mail-sent-symbolic", None));
    list_box.append(&create_mailbox_row("Archive", "mail-archive-symbolic", None));
    list_box.append(&create_mailbox_row("Trash", "user-trash-symbolic", None));
}

fn create_mailbox_row(title: &str, icon_name: &str, unread: Option<u32>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
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

    if let Some(count) = unread {
        let badge = gtk::Label::builder()
            .label(count.to_string())
            .build();
        badge.add_css_class("badge");
        badge.add_css_class("numeric");
        hbox.append(&badge);
    }

    row.set_child(Some(&hbox));
    row
}
