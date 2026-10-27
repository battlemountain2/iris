#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use adw::prelude::*;

pub struct SidebarView {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
    pub sync_btn: gtk::Button,
    pub settings_btn: gtk::Button,
    pub add_folder_btn: gtk::Button,
    pub badges: RefCell<HashMap<String, gtk::Label>>,
    pub rows: RefCell<HashMap<String, gtk::ListBoxRow>>,
    pub custom_folder_rows: RefCell<Vec<gtk::ListBoxRow>>,
    pub on_rename_folder: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
    pub on_delete_folder: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
}

impl SidebarView {
    pub fn new() -> Self {
        Self::with_accounts(&[])
    }

    pub fn with_accounts(accounts: &[crate::services::config::AccountConfig]) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-sidebar");

        let header = adw::HeaderBar::new();
        header.set_show_end_title_buttons(false);

        let acct_count = if accounts.is_empty() { 3 } else { accounts.len() };
        let title = adw::WindowTitle::builder()
            .title("Mailboxes")
            .subtitle(format!("{acct_count} accounts"))
            .build();
        header.set_title_widget(Some(&title));

        // Sync button in header
        let sync_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Sync Mailboxes (mbsync)")
            .build();
        sync_btn.add_css_class("flat");
        header.pack_end(&sync_btn);

        // Settings button in header
        let settings_btn = gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Preferences & Accounts (Ctrl+,)")
            .build();
        settings_btn.add_css_class("flat");
        header.pack_end(&settings_btn);

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

        let add_folder_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Maildir Folder")
            .has_frame(false)
            .valign(gtk::Align::Center)
            .build();
        add_folder_btn.add_css_class("flat");
        add_folder_btn.add_css_class("circular");

        let mut badges_map = HashMap::new();
        let mut rows_map = HashMap::new();
        populate_mailboxes(&list_box, accounts, &add_folder_btn, &mut badges_map, &mut rows_map);

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self {
            container,
            list_box,
            sync_btn,
            settings_btn,
            add_folder_btn,
            badges: RefCell::new(badges_map),
            rows: RefCell::new(rows_map),
            custom_folder_rows: RefCell::new(Vec::new()),
            on_rename_folder: Rc::new(RefCell::new(None)),
            on_delete_folder: Rc::new(RefCell::new(None)),
        }
    }

    /// Sets callbacks for renaming and deleting custom folders from context actions.
    pub fn set_folder_action_callbacks(
        &self,
        on_rename: Rc<dyn Fn(String)>,
        on_delete: Rc<dyn Fn(String)>,
    ) {
        *self.on_rename_folder.borrow_mut() = Some(on_rename);
        *self.on_delete_folder.borrow_mut() = Some(on_delete);
    }

    /// Updates or replaces the custom folders rendered under the FOLDERS section.
    pub fn set_custom_folders(&self, folders: &[String]) {
        // Remove previous custom folder rows
        for old_row in self.custom_folder_rows.borrow().iter() {
            self.list_box.remove(old_row);
        }
        self.custom_folder_rows.borrow_mut().clear();

        for folder_name in folders {
            let row_id = format!("custom_folder:{folder_name}");
            let row = gtk::ListBoxRow::new();
            row.set_widget_name(&row_id);

            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            hbox.set_margin_start(12);
            hbox.set_margin_end(8);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);

            let icon = gtk::Image::from_icon_name("folder-symbolic");
            icon.set_icon_size(gtk::IconSize::Normal);
            hbox.append(&icon);

            let label = gtk::Label::builder()
                .label(folder_name)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            hbox.append(&label);

            let badge = gtk::Label::builder()
                .visible(false)
                .build();
            badge.add_css_class("badge");
            badge.add_css_class("numeric");
            hbox.append(&badge);
            self.badges.borrow_mut().insert(row_id.clone(), badge);

            // 3-dots more button for options
            let more_btn = gtk::Button::builder()
                .icon_name("view-more-symbolic")
                .tooltip_text("Folder Options (Rename / Delete)")
                .has_frame(false)
                .valign(gtk::Align::Center)
                .build();
            more_btn.add_css_class("flat");
            more_btn.add_css_class("circular");
            more_btn.add_css_class("iris-folder-row-actions");
            hbox.append(&more_btn);

            // Context Popover with Rename & Delete
            let popover = gtk::Popover::new();
            let pop_vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);
            pop_vbox.set_margin_start(4);
            pop_vbox.set_margin_end(4);
            pop_vbox.set_margin_top(4);
            pop_vbox.set_margin_bottom(4);

            let rename_btn = gtk::Button::builder()
                .label("Rename Folder...")
                .icon_name("document-edit-symbolic")
                .has_frame(false)
                .build();
            rename_btn.add_css_class("flat");

            let delete_btn = gtk::Button::builder()
                .label("Delete Folder")
                .icon_name("user-trash-symbolic")
                .has_frame(false)
                .build();
            delete_btn.add_css_class("flat");
            delete_btn.add_css_class("destructive-action");

            pop_vbox.append(&rename_btn);
            pop_vbox.append(&delete_btn);
            popover.set_child(Some(&pop_vbox));
            popover.set_parent(&more_btn);

            let pop_clone1 = popover.clone();
            more_btn.connect_clicked(move |_| {
                pop_clone1.popup();
            });

            // Right-click gesture on row
            let right_click = gtk::GestureClick::new();
            right_click.set_button(3); // Button 3 is right mouse button
            let pop_clone2 = popover.clone();
            right_click.connect_pressed(move |_gesture, _n_press, _x, _y| {
                pop_clone2.popup();
            });
            row.add_controller(right_click);

            // Connect Rename and Delete buttons
            let pop_clone3 = popover.clone();
            let on_rename_holder = self.on_rename_folder.clone();
            let fn_name1 = folder_name.clone();
            rename_btn.connect_clicked(move |_| {
                pop_clone3.popdown();
                if let Some(ref cb) = *on_rename_holder.borrow() {
                    cb(fn_name1.clone());
                }
            });

            let pop_clone4 = popover.clone();
            let on_delete_holder = self.on_delete_folder.clone();
            let fn_name2 = folder_name.clone();
            delete_btn.connect_clicked(move |_| {
                pop_clone4.popdown();
                if let Some(ref cb) = *on_delete_holder.borrow() {
                    cb(fn_name2.clone());
                }
            });

            row.set_child(Some(&hbox));
            self.list_box.append(&row);
            self.rows.borrow_mut().insert(row_id, row.clone());
            self.custom_folder_rows.borrow_mut().push(row);
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

fn populate_mailboxes(
    list_box: &gtk::ListBox,
    accounts: &[crate::services::config::AccountConfig],
    add_folder_btn: &gtk::Button,
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

    add_row(list_box, badges, rows, "inbox", "All Inboxes", "mail-folder-inbox-symbolic", None);

    if accounts.is_empty() {
        add_row(list_box, badges, rows, "account:demo", "Gmail (Personal)", "mail-inbox-symbolic", Some(2));
        add_row(list_box, badges, rows, "account:work", "Gmail (Work)", "mail-inbox-symbolic", None);
        add_row(list_box, badges, rows, "account:icloud", "iCloud", "mail-inbox-symbolic", Some(1));
    } else {
        for acct in accounts {
            let row_id = format!("account:{}", acct.id);
            add_row(list_box, badges, rows, &row_id, &acct.display_name, "mail-inbox-symbolic", None);
        }
    }

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
    add_row(list_box, badges, rows, "folder:drafts", "Drafts", "document-edit-symbolic", None);
    add_row(list_box, badges, rows, "folder:sent", "Sent", "mail-sent-symbolic", None);
    add_row(list_box, badges, rows, "folder:archive", "Archive", "mail-archive-symbolic", None);
    add_row(list_box, badges, rows, "folder:trash", "Trash", "user-trash-symbolic", None);

    // Section 3: Folders
    let folders_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    folders_box.set_margin_start(16);
    folders_box.set_margin_end(8);
    folders_box.set_margin_top(16);
    folders_box.set_margin_bottom(4);

    let folders_label = gtk::Label::builder()
        .label("FOLDERS")
        .xalign(0.0)
        .hexpand(true)
        .build();
    folders_label.add_css_class("dim-label");
    folders_label.add_css_class("caption");
    folders_box.append(&folders_label);
    folders_box.append(add_folder_btn);

    let header_row3 = gtk::ListBoxRow::new();
    header_row3.set_selectable(false);
    header_row3.set_activatable(false);
    header_row3.set_child(Some(&folders_box));
    list_box.append(&header_row3);
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
