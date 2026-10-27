// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use adw::prelude::*;
use chrono::{DateTime, Datelike, Local, Utc};

use crate::model::Thread;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FilterTab {
    #[default]
    All,
    Personal,
    Newsletters,
    Unread,
}

pub struct ThreadListView {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
    pub title: adw::WindowTitle,
    pub sidebar_toggle_btn: gtk::Button,
    pub compose_btn: gtk::Button,
    pub empty_trash_btn: gtk::Button,
    pub search_entry: gtk::SearchEntry,
    pub palette_btn: gtk::Button,
    pub filter_tab: Cell<FilterTab>,
    pub tab_all_btn: gtk::ToggleButton,
    pub tab_personal_btn: gtk::ToggleButton,
    pub tab_newsletters_btn: gtk::ToggleButton,
    pub tab_unread_btn: gtk::ToggleButton,
    pub all_threads: RefCell<Vec<Thread>>,
    pub visible_threads: RefCell<Vec<Thread>>,
    pub selected_thread_ids: Rc<RefCell<std::collections::HashSet<String>>>,
    pub swipe_archive_cb: RefCell<Option<Rc<dyn Fn(Thread)>>>,
    pub swipe_trash_cb: RefCell<Option<Rc<dyn Fn(Thread)>>>,
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

        let empty_trash_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text("Empty Trash (permanently delete all)")
            .label("Empty Trash")
            .visible(false)
            .build();
        empty_trash_btn.add_css_class("destructive-action");
        header.pack_end(&empty_trash_btn);

        container.append(&header);

        // Search trigger & live filter bar
        let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        search_box.set_margin_start(10);
        search_box.set_margin_end(10);
        search_box.set_margin_top(6);
        search_box.set_margin_bottom(4);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search messages, bodies, files... (/)")
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

        // Spark-Inspired Filter Tabs ([ All ] [ Personal ] [ Newsletters ] [ Unread ])
        let tabs_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        tabs_box.add_css_class("iris-filter-tabs");
        tabs_box.set_margin_start(10);
        tabs_box.set_margin_end(10);
        tabs_box.set_margin_bottom(6);
        tabs_box.set_homogeneous(true);

        let tab_all_btn = gtk::ToggleButton::builder()
            .label("All")
            .active(true)
            .build();
        tab_all_btn.add_css_class("flat");
        tab_all_btn.add_css_class("iris-tab-btn");

        let tab_personal_btn = gtk::ToggleButton::builder()
            .label("Personal")
            .group(&tab_all_btn)
            .build();
        tab_personal_btn.add_css_class("flat");
        tab_personal_btn.add_css_class("iris-tab-btn");

        let tab_newsletters_btn = gtk::ToggleButton::builder()
            .label("Newsletters")
            .group(&tab_all_btn)
            .build();
        tab_newsletters_btn.add_css_class("flat");
        tab_newsletters_btn.add_css_class("iris-tab-btn");

        let tab_unread_btn = gtk::ToggleButton::builder()
            .label("Unread")
            .group(&tab_all_btn)
            .build();
        tab_unread_btn.add_css_class("flat");
        tab_unread_btn.add_css_class("iris-tab-btn");

        tabs_box.append(&tab_all_btn);
        tabs_box.append(&tab_personal_btn);
        tabs_box.append(&tab_newsletters_btn);
        tabs_box.append(&tab_unread_btn);

        container.append(&tabs_box);

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

        // Escape inside search clears query and returns focus to list
        let list_box_focus = list_box.clone();
        search_entry.connect_stop_search(move |entry| {
            entry.set_text("");
            list_box_focus.grab_focus();
        });

        let all_threads = RefCell::new(Vec::new());
        let visible_threads = RefCell::new(Vec::new());
        let selected_thread_ids = Rc::new(RefCell::new(std::collections::HashSet::new()));
        let swipe_archive_cb = RefCell::new(None);
        let swipe_trash_cb = RefCell::new(None);
        let filter_tab = Cell::new(FilterTab::All);

        let view = Self {
            container,
            list_box,
            title,
            sidebar_toggle_btn,
            compose_btn,
            empty_trash_btn,
            search_entry,
            palette_btn,
            filter_tab,
            tab_all_btn,
            tab_personal_btn,
            tab_newsletters_btn,
            tab_unread_btn,
            all_threads,
            visible_threads,
            selected_thread_ids,
            swipe_archive_cb,
            swipe_trash_cb,
        };

        view
    }

    pub fn connect_swipe_archive<F>(&self, callback: F)
    where
        F: Fn(Thread) + 'static,
    {
        *self.swipe_archive_cb.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn connect_swipe_trash<F>(&self, callback: F)
    where
        F: Fn(Thread) + 'static,
    {
        *self.swipe_trash_cb.borrow_mut() = Some(Rc::new(callback));
    }

    /// Sets up click handlers for the Spark-style filter tabs.
    pub fn setup_tab_handlers(self: &Rc<Self>) {
        let view_all = Rc::downgrade(self);
        self.tab_all_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                if let Some(v) = view_all.upgrade() {
                    v.filter_tab.set(FilterTab::All);
                    v.apply_filter(&v.search_entry.text());
                }
            }
        });

        let view_p = Rc::downgrade(self);
        self.tab_personal_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                if let Some(v) = view_p.upgrade() {
                    v.filter_tab.set(FilterTab::Personal);
                    v.apply_filter(&v.search_entry.text());
                }
            }
        });

        let view_n = Rc::downgrade(self);
        self.tab_newsletters_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                if let Some(v) = view_n.upgrade() {
                    v.filter_tab.set(FilterTab::Newsletters);
                    v.apply_filter(&v.search_entry.text());
                }
            }
        });

        let view_u = Rc::downgrade(self);
        self.tab_unread_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                if let Some(v) = view_u.upgrade() {
                    v.filter_tab.set(FilterTab::Unread);
                    v.apply_filter(&v.search_entry.text());
                }
            }
        });
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

    /// Returns true if one or more threads are currently selected in multi-select mode.
    pub fn is_multi_select(&self) -> bool {
        !self.selected_thread_ids.borrow().is_empty()
    }

    /// Returns a list of all currently multi-selected threads.
    pub fn get_selected_threads(&self) -> Vec<Thread> {
        let sel = self.selected_thread_ids.borrow();
        self.visible_threads
            .borrow()
            .iter()
            .filter(|t| sel.contains(&t.id))
            .cloned()
            .collect()
    }

    /// Toggles multi-selection for the currently focused row in the list box.
    pub fn toggle_current_selection(&self) {
        if let Some(row) = self.list_box.selected_row() {
            let idx = row.index() as usize;
            if let Some(thread) = self.get_thread_at_index(idx) {
                let mut sel = self.selected_thread_ids.borrow_mut();
                if sel.contains(&thread.id) {
                    sel.remove(&thread.id);
                } else {
                    sel.insert(thread.id.clone());
                }
                drop(sel);
                self.update_row_visuals();
            }
        }
    }

    /// Selects all currently visible threads in the list box.
    pub fn select_all(&self) {
        let mut sel = self.selected_thread_ids.borrow_mut();
        for t in self.visible_threads.borrow().iter() {
            sel.insert(t.id.clone());
        }
        drop(sel);
        self.update_row_visuals();
    }

    /// Clears multi-selection and restores normal list subtitle.
    pub fn clear_selection(&self) {
        self.selected_thread_ids.borrow_mut().clear();
        self.update_row_visuals();
    }

    /// Synchronizes CSS classes and subtitle to reflect active multi-selection.
    pub fn update_row_visuals(&self) {
        let sel = self.selected_thread_ids.borrow();
        let sel_count = sel.len();
        let visible = self.visible_threads.borrow();
        for (idx, thread) in visible.iter().enumerate() {
            if let Some(row) = self.list_box.row_at_index(idx as i32) {
                if sel.contains(&thread.id) {
                    row.add_css_class("iris-row-multiselected");
                } else {
                    row.remove_css_class("iris-row-multiselected");
                }
            }
        }
        if sel_count > 0 {
            self.title.set_subtitle(&format!("{sel_count} selected • e: Archive, #: Delete, s: Star, Esc: Clear"));
        } else {
            let q = self.search_entry.text();
            let active_tab = self.filter_tab.get();
            let unread_count = visible.iter().filter(|t| t.is_unread).count();
            if q.is_empty() {
                let tab_label = match active_tab {
                    FilterTab::All => format!("{unread_count} unread"),
                    FilterTab::Personal => format!("{} personal", visible.len()),
                    FilterTab::Newsletters => format!("{} newsletters", visible.len()),
                    FilterTab::Unread => format!("{unread_count} unread"),
                };
                self.title.set_subtitle(&tab_label);
            } else {
                self.title.set_subtitle(&format!("{} matching", visible.len()));
            }
        }
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
        self.selected_thread_ids.borrow_mut().clear();
        self.apply_filter(&self.search_entry.text());
    }

    /// Filters and populates visible thread cards based on the search query and active tab.
    pub fn apply_filter(&self, query: &str) {
        let q = query.trim().to_lowercase();
        let search_query = crate::services::search::SearchQuery::parse(query);
        let active_tab = self.filter_tab.get();
        let all = self.all_threads.borrow();

        let filtered: Vec<Thread> = all
            .iter()
            .filter(|t| {
                let matches_tab = match active_tab {
                    FilterTab::All => true,
                    FilterTab::Unread => t.is_unread,
                    FilterTab::Personal => is_personal_thread(t),
                    FilterTab::Newsletters => is_newsletter_thread(t),
                };
                if !matches_tab {
                    return false;
                }

                if q.is_empty() {
                    true
                } else {
                    search_query.matches_thread(t)
                }
            })
            .cloned()
            .collect();

        *self.visible_threads.borrow_mut() = filtered.clone();

        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let unread_count = filtered.iter().filter(|t| t.is_unread).count();
        let sel_count = self.selected_thread_ids.borrow().len();
        if sel_count > 0 {
            self.title.set_subtitle(&format!("{sel_count} selected • e: Archive, v: Move, #: Delete, s: Star, Esc: Clear"));
        } else if q.is_empty() {
            let tab_label = match active_tab {
                FilterTab::All => format!("{unread_count} unread"),
                FilterTab::Personal => format!("{} personal", filtered.len()),
                FilterTab::Newsletters => format!("{} newsletters", filtered.len()),
                FilterTab::Unread => format!("{unread_count} unread"),
            };
            self.title.set_subtitle(&tab_label);
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

        let archive_cb = self.swipe_archive_cb.borrow().clone();
        let trash_cb = self.swipe_trash_cb.borrow().clone();
        let sel_ids = self.selected_thread_ids.borrow();

        for thread in &filtered {
            let sender = thread
                .senders
                .first()
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let date_str = format_human_timestamp(&thread.latest_timestamp);
            let has_attachment = thread.messages.iter().any(|m| !m.attachments.is_empty());
            let is_selected = sel_ids.contains(&thread.id);

            let row = create_thread_card(
                thread,
                sender,
                &thread.subject,
                &thread.snippet,
                &date_str,
                thread.is_unread,
                thread.is_flagged,
                has_attachment,
                is_selected,
                archive_cb.clone(),
                trash_cb.clone(),
            );
            self.list_box.append(&row);
        }
    }
}

pub fn is_newsletter_thread(t: &Thread) -> bool {
    let lower_subj = t.subject.to_lowercase();
    let lower_senders = t
        .senders
        .iter()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>();
    let newsletter_keywords = [
        "newsletter",
        "digest",
        "no-reply",
        "noreply",
        "notifications",
        "update",
        "marketing",
        "receipt",
        "invoice",
    ];

    t.tags.iter().any(|tag| tag == "feed" || tag == "newsletter")
        || newsletter_keywords.iter().any(|k| lower_subj.contains(k))
        || lower_senders
            .iter()
            .any(|s| newsletter_keywords.iter().any(|k| s.contains(k)))
}

pub fn is_personal_thread(t: &Thread) -> bool {
    !is_newsletter_thread(t)
}

pub fn format_human_timestamp(dt: &DateTime<Utc>) -> String {
    format_human_timestamp_with_ref(dt, Local::now())
}

pub fn format_human_timestamp_with_ref(dt: &DateTime<Utc>, now: DateTime<Local>) -> String {
    let local_dt = dt.with_timezone(&Local);
    let today = now.date_naive();
    let email_date = local_dt.date_naive();

    if email_date == today {
        local_dt.format("%-I:%M %p").to_string()
    } else if email_date == today.pred_opt().unwrap_or(today) {
        "Yesterday".to_string()
    } else if today.signed_duration_since(email_date).num_days() < 7
        && today.signed_duration_since(email_date).num_days() > 0
    {
        local_dt.format("%A").to_string()
    } else if email_date.year() == today.year() {
        local_dt.format("%b %-d").to_string()
    } else {
        local_dt.format("%b %-d, %Y").to_string()
    }
}

fn create_thread_card(
    thread: &Thread,
    sender: &str,
    subject: &str,
    snippet: &str,
    date: &str,
    is_unread: bool,
    is_flagged: bool,
    has_attachment: bool,
    is_multiselected: bool,
    on_archive: Option<Rc<dyn Fn(Thread)>>,
    on_trash: Option<Rc<dyn Fn(Thread)>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("iris-message-row");
    if is_multiselected {
        row.add_css_class("iris-row-multiselected");
    }

    // Gesture controller for two-finger trackpad horizontal pan and mouse drag-swipe
    let gesture = gtk::GestureDrag::new();
    gesture.set_touch_only(false);

    let row_drag = row.clone();
    let row_end = row.clone();
    let thread_for_arch = thread.clone();
    let thread_for_trash = thread.clone();

    gesture.connect_drag_update(move |_gesture, offset_x, offset_y| {
        // Only react when motion is dominantly horizontal
        if offset_x.abs() > offset_y.abs() * 1.1 {
            if offset_x > 25.0 {
                row_drag.add_css_class("swipe-archive");
                row_drag.remove_css_class("swipe-trash");
            } else if offset_x < -25.0 {
                row_drag.add_css_class("swipe-trash");
                row_drag.remove_css_class("swipe-archive");
            } else {
                row_drag.remove_css_class("swipe-archive");
                row_drag.remove_css_class("swipe-trash");
            }
        } else {
            row_drag.remove_css_class("swipe-archive");
            row_drag.remove_css_class("swipe-trash");
        }
    });

    gesture.connect_drag_end(move |_gesture, offset_x, offset_y| {
        row_end.remove_css_class("swipe-archive");
        row_end.remove_css_class("swipe-trash");

        // Trigger action if motion was dominantly horizontal and surpassed 50px threshold
        if offset_x.abs() > offset_y.abs() * 1.2 && offset_x.abs() >= 50.0 {
            if offset_x > 0.0 {
                // Swiped right -> Archive
                if let Some(ref cb) = on_archive {
                    cb(thread_for_arch.clone());
                }
            } else {
                // Swiped left -> Trash
                if let Some(ref cb) = on_trash {
                    cb(thread_for_trash.clone());
                }
            }
        }
    });

    row.add_controller(gesture);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(8);
    vbox.set_margin_bottom(8);

    // Line 1: [Unread Dot] Sender Name [Attachment/Star icons] Right-aligned Date
    let line1 = gtk::Box::new(gtk::Orientation::Horizontal, 6);

    let dot = gtk::Label::new(None);
    if is_unread {
        dot.add_css_class("iris-unread-dot");
    } else {
        dot.add_css_class("iris-unread-placeholder");
    }
    line1.append(&dot);

    let sender_label = gtk::Label::builder()
        .label(sender)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .hexpand(true)
        .build();
    sender_label.add_css_class("iris-row-sender");
    if is_unread {
        sender_label.add_css_class("unread-bold");
    }
    line1.append(&sender_label);

    if has_attachment {
        let attach_icon = gtk::Image::from_icon_name("mail-attachment-symbolic");
        attach_icon.set_icon_size(gtk::IconSize::Normal);
        attach_icon.add_css_class("iris-icon-muted");
        line1.append(&attach_icon);
    }

    if is_flagged {
        let flag_icon = gtk::Image::from_icon_name("starred-symbolic");
        flag_icon.set_icon_size(gtk::IconSize::Normal);
        flag_icon.add_css_class("iris-icon-starred");
        line1.append(&flag_icon);
    }

    let date_label = gtk::Label::builder().label(date).xalign(1.0).build();
    date_label.add_css_class("iris-row-date");
    if is_unread {
        date_label.add_css_class("unread-accent");
    }
    line1.append(&date_label);

    vbox.append(&line1);

    // Line 2: Subject (Medium weight, single line ellipsize)
    let line2 = gtk::Label::builder()
        .label(subject)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    line2.add_css_class("iris-row-subject");
    if is_unread {
        line2.add_css_class("unread-bold");
    }
    vbox.append(&line2);

    // Line 3: Snippet (Dim muted text, single line preview)
    let line3 = gtk::Label::builder()
        .label(snippet)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    line3.add_css_class("iris-row-snippet");
    vbox.append(&line3);

    // Line 4 (optional): Tags pill strip
    if !thread.tags.is_empty() {
        let tag_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        tag_box.set_margin_top(2);
        for tag in &thread.tags {
            let pill = gtk::Label::builder()
                .label(tag)
                .build();
            pill.add_css_class("iris-tag-pill");
            tag_box.append(&pill);
        }
        vbox.append(&tag_box);
    }

    row.set_child(Some(&vbox));
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_format_human_timestamp_today() {
        let now = Local.with_ymd_and_hms(2026, 10, 25, 14, 30, 0).unwrap();
        let email_utc = now.with_timezone(&Utc);

        let formatted = format_human_timestamp_with_ref(&email_utc, now);
        assert_eq!(formatted, "2:30 PM");
    }

    #[test]
    fn test_format_human_timestamp_yesterday() {
        let now = Local.with_ymd_and_hms(2026, 10, 25, 14, 30, 0).unwrap();
        let yesterday = Local.with_ymd_and_hms(2026, 10, 24, 9, 0, 0).unwrap();
        let email_utc = yesterday.with_timezone(&Utc);

        let formatted = format_human_timestamp_with_ref(&email_utc, now);
        assert_eq!(formatted, "Yesterday");
    }

    #[test]
    fn test_format_human_timestamp_this_week() {
        let now = Local.with_ymd_and_hms(2026, 10, 25, 14, 30, 0).unwrap(); // Sunday
        let wednesday = Local.with_ymd_and_hms(2026, 10, 21, 11, 0, 0).unwrap(); // 4 days ago
        let email_utc = wednesday.with_timezone(&Utc);

        let formatted = format_human_timestamp_with_ref(&email_utc, now);
        assert_eq!(formatted, "Wednesday");
    }

    #[test]
    fn test_format_human_timestamp_this_year() {
        let now = Local.with_ymd_and_hms(2026, 10, 25, 14, 30, 0).unwrap();
        let past = Local.with_ymd_and_hms(2026, 3, 15, 8, 0, 0).unwrap();
        let email_utc = past.with_timezone(&Utc);

        let formatted = format_human_timestamp_with_ref(&email_utc, now);
        assert_eq!(formatted, "Mar 15");
    }

    #[test]
    fn test_format_human_timestamp_older_years() {
        let now = Local.with_ymd_and_hms(2026, 10, 25, 14, 30, 0).unwrap();
        let older = Local.with_ymd_and_hms(2024, 12, 1, 10, 0, 0).unwrap();
        let email_utc = older.with_timezone(&Utc);

        let formatted = format_human_timestamp_with_ref(&email_utc, now);
        assert_eq!(formatted, "Dec 1, 2024");
    }

    #[test]
    fn test_is_newsletter_thread() {
        let t1 = Thread {
            id: "1".into(),
            subject: "GitHub Security Digest".into(),
            snippet: "Here is your weekly summary".into(),
            senders: vec!["notifications@github.com".into()],
            latest_timestamp: Utc::now(),
            message_count: 1,
            is_unread: true,
            is_flagged: false,
            tags: vec![],
            messages: vec![],
        };
        assert!(is_newsletter_thread(&t1));
        assert!(!is_personal_thread(&t1));

        let t2 = Thread {
            id: "2".into(),
            subject: "Lunch on Friday?".into(),
            snippet: "Hey let's catch up".into(),
            senders: vec!["Alex Rivera <alex@hermes.dev>".into()],
            latest_timestamp: Utc::now(),
            message_count: 1,
            is_unread: true,
            is_flagged: false,
            tags: vec![],
            messages: vec![],
        };
        assert!(!is_newsletter_thread(&t2));
        assert!(is_personal_thread(&t2));
    }
}
