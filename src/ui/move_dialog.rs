// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;

pub struct MoveToFolderDialog {
    dialog: adw::Dialog,
    search_entry: gtk::SearchEntry,
    list_box: gtk::ListBox,
    folders: Rc<RefCell<Vec<String>>>,
    on_select: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
}

impl MoveToFolderDialog {
    pub fn new() -> Rc<Self> {
        let dialog = adw::Dialog::builder()
            .title("Move Conversation")
            .content_width(380)
            .content_height(420)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
        vbox.set_margin_start(16);
        vbox.set_margin_end(16);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);

        let title_label = gtk::Label::builder()
            .label("Move to Folder")
            .xalign(0.0)
            .build();
        title_label.add_css_class("title-3");
        vbox.append(&title_label);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Filter or create folder... (Enter to move)")
            .hexpand(true)
            .build();
        search_entry.add_css_class("iris-search-entry");
        vbox.append(&search_entry);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        list_box.add_css_class("boxed-list");
        scrolled.set_child(Some(&list_box));
        vbox.append(&scrolled);

        dialog.set_child(Some(&vbox));

        let folders = Rc::new(RefCell::new(Vec::new()));
        let on_select = Rc::new(RefCell::new(None));

        // Connect search entry Stop search (Esc) to close
        let dialog_esc = dialog.clone();
        search_entry.connect_stop_search(move |_| {
            dialog_esc.close();
        });

        let this = Rc::new(Self {
            dialog,
            search_entry,
            list_box,
            folders,
            on_select,
        });

        this.setup_handlers();
        this
    }

    fn setup_handlers(self: &Rc<Self>) {
        let this_for_search = Rc::downgrade(self);
        self.search_entry.connect_search_changed(move |entry| {
            if let Some(this) = this_for_search.upgrade() {
                this.filter_list(&entry.text());
            }
        });

        // Enter key in search entry activates the currently selected row
        let this_for_activate = Rc::downgrade(self);
        self.search_entry.connect_activate(move |_| {
            if let Some(this) = this_for_activate.upgrade() {
                if let Some(row) = this.list_box.selected_row() {
                    this.activate_row(&row);
                }
            }
        });

        let this_for_row = Rc::downgrade(self);
        self.list_box.connect_row_activated(move |_list, row| {
            if let Some(this) = this_for_row.upgrade() {
                this.activate_row(row);
            }
        });
    }

    pub fn set_on_select(&self, callback: Rc<dyn Fn(String)>) {
        *self.on_select.borrow_mut() = Some(callback);
    }

    pub fn present(
        &self,
        parent: &impl IsA<gtk::Widget>,
        current_folder: Option<&str>,
        available_folders: &[String],
    ) {
        let mut filtered_folders = available_folders.to_vec();
        if let Some(cur) = current_folder {
            filtered_folders.retain(|f| !f.eq_ignore_ascii_case(cur));
        }
        *self.folders.borrow_mut() = filtered_folders;

        self.search_entry.set_text("");
        self.filter_list("");
        self.dialog.present(Some(parent));
        self.search_entry.grab_focus();
    }

    fn filter_list(&self, query: &str) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let q = query.trim().to_lowercase();
        let all_folders = self.folders.borrow();

        let matching: Vec<&String> = all_folders
            .iter()
            .filter(|f| q.is_empty() || f.to_lowercase().contains(&q))
            .collect();

        for folder in &matching {
            let row = create_folder_row(folder, false);
            self.list_box.append(&row);
        }

        // If query is not empty and doesn't exactly match any existing folder, offer to create it
        if !q.is_empty() && !all_folders.iter().any(|f| f.to_lowercase() == q) {
            let row = create_folder_row(query.trim(), true);
            self.list_box.append(&row);
        }

        // Select the first row by default
        if let Some(first) = self.list_box.row_at_index(0) {
            self.list_box.select_row(Some(&first));
        }
    }

    fn activate_row(&self, row: &gtk::ListBoxRow) {
        let folder_name = row.widget_name().to_string();
        if !folder_name.is_empty() {
            if let Some(ref cb) = *self.on_select.borrow() {
                cb(folder_name);
            }
            self.dialog.close();
        }
    }
}

fn create_folder_row(folder_name: &str, is_create: bool) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(folder_name);

    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    let icon_name = match folder_name.to_lowercase().as_str() {
        "inbox" => "mail-folder-inbox-symbolic",
        "archive" => "mail-archive-symbolic",
        "trash" => "user-trash-symbolic",
        "sent" => "mail-sent-symbolic",
        "drafts" => "document-edit-symbolic",
        _ => {
            if is_create {
                "list-add-symbolic"
            } else {
                "folder-symbolic"
            }
        }
    };

    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_icon_size(gtk::IconSize::Normal);
    if is_create {
        icon.add_css_class("accent");
    } else {
        icon.add_css_class("dim-label");
    }
    hbox.append(&icon);

    let label_text = if is_create {
        format!("Create & Move to \"{folder_name}\"")
    } else {
        folder_name.to_string()
    };

    let label = gtk::Label::builder()
        .label(&label_text)
        .xalign(0.0)
        .hexpand(true)
        .build();
    if is_create {
        label.add_css_class("accent");
    }
    hbox.append(&label);

    row.set_child(Some(&hbox));
    row
}
