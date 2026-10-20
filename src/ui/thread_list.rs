// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use adw::prelude::*;

use crate::model::Thread;

pub struct ThreadListView {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
    pub title: adw::WindowTitle,
    pub sidebar_toggle_btn: gtk::Button,
    pub compose_btn: gtk::Button,
    pub search_entry: gtk::SearchEntry,
    pub palette_btn: gtk::Button,
    pub all_threads: RefCell<Vec<Thread>>,
    pub visible_threads: RefCell<Vec<Thread>>,
}

impl ThreadListView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-thread-pane");

        // Headerbar with Sidebar Toggle, Title, Filter, and Compose Button
        let header = adw::HeaderBar::new();
        header.set_show_start_title_buttons(false);

        // Sidebar toggle button (iOS / macOS Mail sidebar toggle icon)
        let sidebar_toggle_btn = gtk::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Toggle Mailboxes (Ctrl+S)")
            .build();
        sidebar_toggle_btn.add_css_class("flat");
        header.pack_start(&sidebar_toggle_btn);

        let title = adw::WindowTitle::builder()
            .title("Inbox")
            .subtitle("Loading...")
            .build();
        header.set_title_widget(Some(&title));

        let compose_btn = gtk::Button::builder()
            .icon_name("mail-message-new-symbolic")
            .tooltip_text("Compose (c)")
            .build();
        compose_btn.add_css_class("suggested-action");
        header.pack_end(&compose_btn);

        container.append(&header);

        // Search trigger & live filter bar
        let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        search_box.set_margin_start(10);
        search_box.set_margin_end(10);
        search_box.set_margin_top(6);
        search_box.set_margin_bottom(6);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Filter conversations... (/)")
            .hexpand(true)
            .build();
        search_entry.add_css_class("iris-search-entry");
        search_box.append(&search_entry);

        let palette_btn = gtk::Button::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Command Palette (Ctrl+K)")
            .build();
        palette_btn.add_css_class("flat");
        palette_btn.add_css_class("iris-palette-trigger");
        search_box.append(&palette_btn);

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

        let all_threads = RefCell::new(Vec::new());
        let visible_threads = RefCell::new(Vec::new());

        Self {
            container,
            list_box,
            title,
            sidebar_toggle_btn,
            compose_btn,
            search_entry,
            palette_btn,
            all_threads,
            visible_threads,
        }
    }

    /// Returns the thread at the specified visible index.
    pub fn get_thread_at_index(&self, index: usize) -> Option<Thread> {
        self.visible_threads.borrow().get(index).cloned()
    }

    /// Returns the currently selected thread in the list box.
    #[allow(dead_code)]
    pub fn get_selected_thread(&self) -> Option<Thread> {
        let row = self.list_box.selected_row()?;
        self.get_thread_at_index(row.index() as usize)
    }

    /// Returns a snapshot of all currently visible threads.
    pub fn get_visible_threads(&self) -> Vec<Thread> {
        self.visible_threads.borrow().clone()
    }

    /// Selects the row at the given index and gives it focus.
    pub fn select_index(&self, index: usize) {
        if let Some(row) = self.list_box.row_at_index(index as i32) {
            self.list_box.select_row(Some(&row));
            row.grab_focus();
        }
    }

    /// Sets the full list of threads for the current folder and applies active search filter.
    pub fn set_threads(&self, threads: &[Thread]) {
        *self.all_threads.borrow_mut() = threads.to_vec();
        self.apply_filter(&self.search_entry.text());
    }

    /// Filters and populates visible thread cards based on the search query.
    pub fn apply_filter(&self, query: &str) {
        let q = query.trim().to_lowercase();
        let all = self.all_threads.borrow();

        let filtered: Vec<Thread> = if q.is_empty() {
            all.clone()
        } else {
            all.iter()
                .filter(|t| {
                    t.subject.to_lowercase().contains(&q)
                        || t.snippet.to_lowercase().contains(&q)
                        || t.senders.iter().any(|s| s.to_lowercase().contains(&q))
                })
                .cloned()
                .collect()
        };

        *self.visible_threads.borrow_mut() = filtered.clone();

        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let unread_count = filtered.iter().filter(|t| t.is_unread).count();
        if q.is_empty() {
            self.title.set_subtitle(&format!("{unread_count} unread"));
        } else {
            self.title.set_subtitle(&format!("{} matching", filtered.len()));
        }

        if filtered.is_empty() {
            let empty_row = gtk::ListBoxRow::new();
            empty_row.set_selectable(false);
            empty_row.set_activatable(false);

            let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
            empty_box.set_margin_top(32);
            empty_box.set_margin_bottom(32);
            empty_box.set_halign(gtk::Align::Center);

            let icon = gtk::Image::from_icon_name(if q.is_empty() {
                "mail-box-symbolic"
            } else {
                "edit-find-symbolic"
            });
            icon.set_icon_size(gtk::IconSize::Large);
            icon.add_css_class("dim-label");
            empty_box.append(&icon);

            let label = gtk::Label::builder()
                .label(if q.is_empty() {
                    "Inbox zero. All caught up! ✨"
                } else {
                    "No matching conversations found."
                })
                .build();
            label.add_css_class("caption");
            label.add_css_class("dim-label");
            empty_box.append(&label);

            empty_row.set_child(Some(&empty_box));
            self.list_box.append(&empty_row);
            return;
        }

        for thread in &filtered {
            let sender = thread
                .senders
                .first()
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let date_str = thread.latest_timestamp.format("%b %d, %H:%M").to_string();
            let has_attachment = thread.messages.iter().any(|m| !m.attachments.is_empty());

            let row = create_thread_card(
                sender,
                &thread.subject,
                &thread.snippet,
                &date_str,
                thread.is_unread,
                thread.is_flagged,
                has_attachment,
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
    is_flagged: bool,
    has_attachment: bool,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("iris-thread-card");
    row.add_css_class("iris-message-row");

    let card_vbox = gtk::Box::new(gtk::Orientation::Vertical, 3);

    // Top line: Unread dot / placeholder + Sender + Icons + Date
    let top_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let unread_dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    if is_unread {
        unread_dot.add_css_class("iris-unread-dot");
    } else {
        unread_dot.add_css_class("iris-unread-placeholder");
    }
    unread_dot.set_valign(gtk::Align::Center);
    top_hbox.append(&unread_dot);

    let sender_label = gtk::Label::builder()
        .label(sender)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    if is_unread {
        sender_label.add_css_class("heading");
        sender_label.add_css_class("iris-sender-unread");
    } else {
        sender_label.add_css_class("body");
        sender_label.add_css_class("iris-sender-read");
    }
    top_hbox.append(&sender_label);

    if has_attachment {
        let attach_icon = gtk::Image::from_icon_name("mail-attachment-symbolic");
        attach_icon.add_css_class("dim-label");
        attach_icon.set_icon_size(gtk::IconSize::Inherit);
        top_hbox.append(&attach_icon);
    }

    if is_flagged {
        let star_icon = gtk::Image::from_icon_name("starred-symbolic");
        star_icon.add_css_class("iris-star-badge");
        star_icon.set_icon_size(gtk::IconSize::Inherit);
        top_hbox.append(&star_icon);
    }

    let date_label = gtk::Label::builder()
        .label(date)
        .xalign(1.0)
        .build();
    date_label.add_css_class("caption");
    date_label.add_css_class("dim-label");
    top_hbox.append(&date_label);

    card_vbox.append(&top_hbox);

    // Subject line (crisp medium text)
    let subject_label = gtk::Label::builder()
        .label(subject)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    if is_unread {
        subject_label.add_css_class("body");
        subject_label.add_css_class("iris-subject-unread");
    } else {
        subject_label.add_css_class("body");
        subject_label.add_css_class("dim-label");
    }
    card_vbox.append(&subject_label);

    // Preview snippet (subdued secondary text)
    let snippet_label = gtk::Label::builder()
        .label(snippet)
        .xalign(0.0)
        .lines(1)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    snippet_label.add_css_class("caption");
    snippet_label.add_css_class("dim-label");
    card_vbox.append(&snippet_label);

    row.set_child(Some(&card_vbox));
    row
}
