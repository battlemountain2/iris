#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;

pub struct ComposeDialog {
    dialog: adw::Dialog,
    to_entry: gtk::Entry,
    subject_entry: gtk::Entry,
    body_view: gtk::TextView,
    from_combo: adw::ComboRow,
}

impl ComposeDialog {
    pub fn new<F>(on_send: F) -> Self
    where
        F: Fn(String, String, String) + 'static,
    {
        let dialog = adw::Dialog::builder()
            .title("New Message")
            .content_width(680)
            .content_height(540)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.add_css_class("iris-compose");

        // Headerbar with Cancel and Send
        let header = adw::HeaderBar::new();
        header.set_show_start_title_buttons(false);
        header.set_show_end_title_buttons(false);

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        cancel_btn.add_css_class("flat");
        let dialog_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            dialog_cancel.close();
        });
        header.pack_start(&cancel_btn);

        let send_btn = gtk::Button::builder().label("Send").build();
        send_btn.add_css_class("suggested-action");
        header.pack_end(&send_btn);

        vbox.append(&header);

        // Grouped Fields (From, To, Subject)
        let pref_group = adw::PreferencesGroup::new();
        pref_group.set_margin_start(16);
        pref_group.set_margin_end(16);
        pref_group.set_margin_top(8);
        pref_group.set_margin_bottom(8);

        let from_combo = adw::ComboRow::builder()
            .title("From")
            .model(&gtk::StringList::new(&[
                "Gmail (Personal) <bry@gmail.com>",
                "Gmail (Work) <brayan@work.com>",
                "iCloud <brayan@icloud.com>",
            ]))
            .build();
        pref_group.add(&from_combo);

        let to_row = adw::ActionRow::builder().title("To").build();
        let to_entry = gtk::Entry::builder()
            .placeholder_text("recipient@example.com")
            .hexpand(true)
            .build();
        to_entry.add_css_class("flat");
        to_row.add_suffix(&to_entry);
        pref_group.add(&to_row);

        let subject_row = adw::ActionRow::builder().title("Subject").build();
        let subject_entry = gtk::Entry::builder()
            .placeholder_text("Subject")
            .hexpand(true)
            .build();
        subject_entry.add_css_class("flat");
        subject_row.add_suffix(&subject_entry);
        pref_group.add(&subject_row);

        vbox.append(&pref_group);

        // Body Editor
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(12)
            .build();

        let body_view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::Word)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .build();
        body_view.add_css_class("iris-compose-body");

        scrolled.set_child(Some(&body_view));
        vbox.append(&scrolled);

        dialog.set_child(Some(&vbox));

        // Connect Send Button
        let to_clone = to_entry.clone();
        let sub_clone = subject_entry.clone();
        let body_clone = body_view.clone();
        let diag_clone = dialog.clone();
        send_btn.connect_clicked(move |_| {
            let to = to_clone.text().to_string();
            let subject = sub_clone.text().to_string();
            let buffer = body_clone.buffer();
            let (start, end) = buffer.bounds();
            let body = buffer.text(&start, &end, false).to_string();

            on_send(to, subject, body);
            diag_clone.close();
        });

        Self {
            dialog,
            to_entry,
            subject_entry,
            body_view,
            from_combo,
        }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.to_entry.set_text("");
        self.subject_entry.set_text("");
        self.body_view.buffer().set_text("");
        self.dialog.present(Some(parent));
        self.to_entry.grab_focus();
    }
}
