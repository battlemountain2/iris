// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;
use gtk::glib;

use crate::model::Thread;

pub struct ThreadListView {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
    pub title: adw::WindowTitle,
}

impl ThreadListView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-thread-pane");

        // Headerbar with Title, Filter, and Compose Button
        let header = adw::HeaderBar::new();
        header.set_show_start_title_buttons(false);

        let title = adw::WindowTitle::builder()
            .title("Inbox")
            .subtitle("Loading...")
            .build();
        header.set_title_widget(Some(&title));

        let filter_btn = gtk::Button::builder()
            .icon_name("view-filter-symbolic")
            .tooltip_text("Filter unread only")
            .build();
        filter_btn.add_css_class("flat");
        header.pack_start(&filter_btn);

        let compose_btn = gtk::Button::builder()
            .icon_name("mail-message-new-symbolic")
            .tooltip_text("Compose (c)")
            .build();
        compose_btn.add_css_class("suggested-action");
        header.pack_end(&compose_btn);

        container.append(&header);

        // Search trigger bar (Ctrl+K omnibar)
        let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        search_box.set_margin_start(10);
        search_box.set_margin_end(10);
        search_box.set_margin_top(6);
        search_box.set_margin_bottom(6);

        let search_button = gtk::Button::builder()
            .label("Search mail, contacts, tags... (Ctrl+K)")
            .icon_name("edit-find-symbolic")
            .hexpand(true)
            .build();
        search_button.add_css_class("flat");
        search_button.add_css_class("iris-search-trigger");
        search_box.append(&search_button);
        container.append(&search_box);

        // Scrolled window of thread cards
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        list_box.add_css_class("boxed-list-separate");

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self {
            container,
            list_box,
            title,
        }
    }

    /// Populates the list box with real threads scanned from Maildir.
    pub fn set_threads(&self, threads: &[Thread]) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let unread_count = threads.iter().filter(|t| t.is_unread).count();
        self.title.set_subtitle(&format!("{unread_count} unread"));

        for thread in threads {
            let sender = thread
                .senders
                .first()
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let date_str = thread.latest_timestamp.format("%b %d, %H:%M").to_string();

            let row = create_thread_card(
                sender,
                &thread.subject,
                &thread.snippet,
                &date_str,
                thread.is_unread,
                false,
            );
            self.list_box.append(&row);
        }
    }
}

fn create_thread_card(
    sender: &str,
    subject: &str,
    snippet: &str,
    date: &str,
    is_unread: bool,
    has_attachment: bool,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("iris-thread-card");

    let card_vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);

    // Top line: Unread dot + Sender + Date
    let top_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    if is_unread {
        let unread_dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        unread_dot.add_css_class("iris-unread-badge");
        unread_dot.set_valign(gtk::Align::Center);
        top_hbox.append(&unread_dot);
    }

    let sender_label = gtk::Label::builder()
        .label(sender)
        .xalign(0.0)
        .hexpand(true)
        .build();
    if is_unread {
        sender_label.add_css_class("heading");
    } else {
        sender_label.add_css_class("body");
    }
    top_hbox.append(&sender_label);

    if has_attachment {
        let attach_icon = gtk::Image::from_icon_name("mail-attachment-symbolic");
        attach_icon.add_css_class("dim-label");
        top_hbox.append(&attach_icon);
    }

    let date_label = gtk::Label::builder().label(date).xalign(1.0).build();
    date_label.add_css_class("caption");
    date_label.add_css_class("dim-label");
    top_hbox.append(&date_label);

    card_vbox.append(&top_hbox);

    // Subject line
    let subject_label = gtk::Label::builder()
        .label(subject)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    if is_unread {
        subject_label.add_css_class("body");
    } else {
        subject_label.add_css_class("dim-label");
    }
    card_vbox.append(&subject_label);

    // Snippet preview line
    let snippet_label = gtk::Label::builder()
        .label(snippet)
        .xalign(0.0)
        .lines(2)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    snippet_label.add_css_class("caption");
    snippet_label.add_css_class("dim-label");
    card_vbox.append(&snippet_label);

    row.set_child(Some(&card_vbox));
    row
}
