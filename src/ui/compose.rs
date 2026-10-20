#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;

pub struct ComposeDialog {
    dialog: adw::Dialog,
    to_entry: gtk::Entry,
    subject_entry: gtk::Entry,
    body_view: gtk::TextView,
    from_dropdown: gtk::DropDown,
}

impl ComposeDialog {
    pub fn new<F>(on_send: F) -> Self
    where
        F: Fn(String, String, String) + 'static,
    {
        let dialog = adw::Dialog::builder()
            .title("New Message")
            .content_width(700)
            .content_height(560)
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

        let send_btn = gtk::Button::builder()
            .label("Send")
            .tooltip_text("Save Draft (SMTP transport queue in development)")
            .build();
        send_btn.add_css_class("suggested-action");
        header.pack_end(&send_btn);

        vbox.append(&header);

        // Compact Address Header (From, To, Subject)
        let fields_vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
        fields_vbox.add_css_class("iris-compose-fields");
        fields_vbox.set_margin_start(16);
        fields_vbox.set_margin_end(16);
        fields_vbox.set_margin_top(8);
        fields_vbox.set_margin_bottom(8);

        // From line
        let from_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        from_hbox.add_css_class("iris-compose-row");
        let from_lbl = gtk::Label::builder()
            .label("From:")
            .xalign(0.0)
            .width_chars(7)
            .build();
        from_lbl.add_css_class("caption");
        from_lbl.add_css_class("dim-label");
        from_hbox.append(&from_lbl);

        let from_dropdown = gtk::DropDown::from_strings(&[
            "Gmail (Personal) <bry@gmail.com>",
            "Gmail (Work) <brayan@work.com>",
            "iCloud <brayan@icloud.com>",
        ]);
        from_dropdown.set_hexpand(true);
        from_dropdown.add_css_class("flat");
        from_hbox.append(&from_dropdown);
        fields_vbox.append(&from_hbox);

        // To line
        let to_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        to_hbox.add_css_class("iris-compose-row");
        let to_lbl = gtk::Label::builder()
            .label("To:")
            .xalign(0.0)
            .width_chars(7)
            .build();
        to_lbl.add_css_class("caption");
        to_lbl.add_css_class("dim-label");
        to_hbox.append(&to_lbl);

        let to_entry = gtk::Entry::builder()
            .placeholder_text("recipient@example.com")
            .hexpand(true)
            .build();
        to_entry.add_css_class("flat");
        to_hbox.append(&to_entry);
        fields_vbox.append(&to_hbox);

        // Subject line
        let subject_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        subject_hbox.add_css_class("iris-compose-row");
        let subject_lbl = gtk::Label::builder()
            .label("Subject:")
            .xalign(0.0)
            .width_chars(7)
            .build();
        subject_lbl.add_css_class("caption");
        subject_lbl.add_css_class("dim-label");
        subject_hbox.append(&subject_lbl);

        let subject_entry = gtk::Entry::builder()
            .placeholder_text("Subject")
            .hexpand(true)
            .build();
        subject_entry.add_css_class("flat");
        subject_hbox.append(&subject_entry);
        fields_vbox.append(&subject_hbox);

        vbox.append(&fields_vbox);

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
            from_dropdown,
        }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.to_entry.set_text("");
        self.subject_entry.set_text("");
        self.body_view.buffer().set_text("");
        self.dialog.present(Some(parent));
        self.to_entry.grab_focus();
    }

    pub fn present_reply(
        &self,
        parent: &impl IsA<gtk::Widget>,
        to: &str,
        subject: &str,
        quote_body: &str,
    ) {
        self.to_entry.set_text(to);
        let reply_subject = if subject.to_lowercase().starts_with("re:") {
            subject.to_string()
        } else {
            format!("Re: {subject}")
        };
        self.subject_entry.set_text(&reply_subject);

        let quote = if !quote_body.is_empty() {
            format!("\n\n> {}", quote_body.replace('\n', "\n> "))
        } else {
            String::new()
        };
        self.body_view.buffer().set_text(&quote);

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }
}
