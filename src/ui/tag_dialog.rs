// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;

pub struct TagDialog {
    dialog: adw::Dialog,
    search_entry: gtk::SearchEntry,
    list_box: gtk::ListBox,
    thread_id: Rc<RefCell<String>>,
    current_tags: Rc<RefCell<Vec<String>>>,
    on_tags_changed: Rc<RefCell<Option<Rc<dyn Fn(String, Vec<String>)>>>>,
}

impl TagDialog {
    pub fn new() -> Rc<Self> {
        let dialog = adw::Dialog::builder()
            .title("Labels & Tags")
            .content_width(360)
            .content_height(420)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
        vbox.set_margin_start(16);
        vbox.set_margin_end(16);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);

        let title_label = gtk::Label::builder()
            .label("Thread Labels & Tags")
            .xalign(0.0)
            .build();
        title_label.add_css_class("title-3");
        vbox.append(&title_label);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Filter or create tag... (Enter to toggle)")
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

        let thread_id = Rc::new(RefCell::new(String::new()));
        let current_tags = Rc::new(RefCell::new(Vec::new()));
        let on_tags_changed = Rc::new(RefCell::new(None));

        let dialog_esc = dialog.clone();
        search_entry.connect_stop_search(move |_| {
            dialog_esc.close();
        });

        let this = Rc::new(Self {
            dialog,
            search_entry,
            list_box,
            thread_id,
            current_tags,
            on_tags_changed,
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

    pub fn set_on_tags_changed(&self, callback: Rc<dyn Fn(String, Vec<String>)>) {
        *self.on_tags_changed.borrow_mut() = Some(callback);
    }

    pub fn present(
        &self,
        parent: &impl IsA<gtk::Widget>,
        thread_id: &str,
        _thread_subject: &str,
    ) {
        *self.thread_id.borrow_mut() = thread_id.to_string();
        let tags = crate::services::tags::get_tags_for_thread(thread_id);
        *self.current_tags.borrow_mut() = tags;

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
        let mut all_tags = vec![
            "Work".to_string(),
            "Personal".to_string(),
            "Urgent".to_string(),
            "Finance".to_string(),
            "Projects".to_string(),
            "Feed".to_string(),
        ];
        for t in crate::services::tags::list_all_tags() {
            if !all_tags.iter().any(|existing| existing.eq_ignore_ascii_case(&t)) {
                all_tags.push(t);
            }
        }
        for t in self.current_tags.borrow().iter() {
            if !all_tags.iter().any(|existing| existing.eq_ignore_ascii_case(t)) {
                all_tags.push(t.clone());
            }
        }
        all_tags.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

        let active_tags = self.current_tags.borrow().clone();

        // If query is not empty and doesn't exactly match any known tag, show create option
        if !q.is_empty() && !all_tags.iter().any(|t| t.to_lowercase() == q) {
            let row = create_tag_row(query.trim(), false, true);
            self.list_box.append(&row);
        }

        let matching: Vec<&String> = all_tags
            .iter()
            .filter(|t| q.is_empty() || t.to_lowercase().contains(&q))
            .collect();

        for tag in matching {
            let is_active = active_tags.iter().any(|t| t.eq_ignore_ascii_case(tag));
            let row = create_tag_row(tag, is_active, false);
            self.list_box.append(&row);
        }

        if let Some(first) = self.list_box.row_at_index(0) {
            self.list_box.select_row(Some(&first));
        }
    }

    fn activate_row(&self, row: &gtk::ListBoxRow) {
        let name = row.widget_name().to_string();
        if let Some(new_tag) = name.strip_prefix("__create__:") {
            let updated = crate::services::tags::add_tag_to_thread(&self.thread_id.borrow(), new_tag);
            *self.current_tags.borrow_mut() = updated.clone();
            if let Some(ref cb) = *self.on_tags_changed.borrow() {
                cb(self.thread_id.borrow().clone(), updated);
            }
            self.search_entry.set_text("");
            self.filter_list("");
        } else if let Some(tag_name) = name.strip_prefix("tag:") {
            let (_added, updated) = crate::services::tags::toggle_tag_for_thread(&self.thread_id.borrow(), tag_name);
            *self.current_tags.borrow_mut() = updated.clone();
            if let Some(ref cb) = *self.on_tags_changed.borrow() {
                cb(self.thread_id.borrow().clone(), updated);
            }
            self.filter_list(&self.search_entry.text());
        }
    }
}

fn create_tag_row(tag: &str, is_active: bool, is_create: bool) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let row_id = if is_create {
        format!("__create__:{tag}")
    } else {
        format!("tag:{tag}")
    };
    row.set_widget_name(&row_id);

    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    let icon_name = if is_create {
        "list-add-symbolic"
    } else {
        "tag-symbolic"
    };
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_icon_size(gtk::IconSize::Normal);
    if is_create || is_active {
        icon.add_css_class("accent");
    } else {
        icon.add_css_class("dim-label");
    }
    hbox.append(&icon);

    let label_text = if is_create {
        format!("Create tag \"{tag}\"")
    } else {
        tag.to_string()
    };
    let label = gtk::Label::builder()
        .label(&label_text)
        .xalign(0.0)
        .hexpand(true)
        .build();
    if is_create || is_active {
        label.add_css_class("accent");
        if is_active {
            label.add_css_class("heading");
        }
    }
    hbox.append(&label);

    if is_active {
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.set_icon_size(gtk::IconSize::Normal);
        check.add_css_class("accent");
        hbox.append(&check);
    }

    row.set_child(Some(&hbox));
    row
}
