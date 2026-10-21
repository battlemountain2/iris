#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;

#[derive(Clone, Debug)]
pub struct AccountIdentity {
    pub id: &'static str,
    pub display: &'static str,
    pub email: &'static str,
}

pub const CONFIGURED_ACCOUNTS: &[AccountIdentity] = &[
    AccountIdentity {
        id: "demo",
        display: "Gmail (Personal)",
        email: "bry@gmail.com",
    },
    AccountIdentity {
        id: "work",
        display: "Gmail (Work)",
        email: "brayan@work.com",
    },
    AccountIdentity {
        id: "icloud",
        display: "iCloud",
        email: "brayan@icloud.com",
    },
];

#[derive(Clone, Debug)]
pub struct ComposePayload {
    pub account_id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
}

pub struct ComposeDialog {
    dialog: adw::Dialog,
    to_entry: gtk::Entry,
    subject_entry: gtk::Entry,
    body_view: gtk::TextView,
    from_dropdown: gtk::DropDown,
    in_reply_to: Rc<RefCell<Option<String>>>,
    references: Rc<RefCell<Vec<String>>>,
}

impl ComposeDialog {
    pub fn new<F>(on_send: F) -> Self
    where
        F: Fn(ComposePayload) + 'static,
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
            .tooltip_text("Send message (10-second Undo window)")
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

        let account_labels: Vec<String> = CONFIGURED_ACCOUNTS
            .iter()
            .map(|a| format!("{} <{}>", a.display, a.email))
            .collect();
        let account_str_slices: Vec<&str> = account_labels.iter().map(|s| s.as_str()).collect();

        let from_dropdown = gtk::DropDown::from_strings(&account_str_slices);
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

        let in_reply_to = Rc::new(RefCell::new(None::<String>));
        let references = Rc::new(RefCell::new(Vec::<String>::new()));

        // Connect Send Button
        let to_clone = to_entry.clone();
        let sub_clone = subject_entry.clone();
        let body_clone = body_view.clone();
        let from_clone = from_dropdown.clone();
        let diag_clone = dialog.clone();
        let in_reply_to_clone = in_reply_to.clone();
        let references_clone = references.clone();

        send_btn.connect_clicked(move |_| {
            let to = to_clone.text().trim().to_string();
            let subject = sub_clone.text().trim().to_string();
            let buffer = body_clone.buffer();
            let (start, end) = buffer.bounds();
            let body = buffer.text(&start, &end, false).to_string();

            let acct_idx = from_clone.selected() as usize;
            let acct = CONFIGURED_ACCOUNTS
                .get(acct_idx)
                .unwrap_or(&CONFIGURED_ACCOUNTS[0]);

            let payload = ComposePayload {
                account_id: acct.id.to_string(),
                from: acct.email.to_string(),
                to,
                subject,
                body,
                in_reply_to: in_reply_to_clone.borrow().clone(),
                references: references_clone.borrow().clone(),
            };

            diag_clone.close();
            on_send(payload);
        });

        Self {
            dialog,
            to_entry,
            subject_entry,
            body_view,
            from_dropdown,
            in_reply_to,
            references,
        }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.to_entry.set_text("");
        self.subject_entry.set_text("");
        self.body_view.buffer().set_text("");
        *self.in_reply_to.borrow_mut() = None;
        self.references.borrow_mut().clear();
        self.from_dropdown.set_selected(0);
        self.dialog.present(Some(parent));
        self.to_entry.grab_focus();
    }

    pub fn present_reply(
        &self,
        parent: &impl IsA<gtk::Widget>,
        to: &str,
        subject: &str,
        quote_body: &str,
        in_reply_to: Option<String>,
        references: Vec<String>,
        account_id: Option<&str>,
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

        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;

        if let Some(acct) = account_id {
            if let Some(pos) = CONFIGURED_ACCOUNTS.iter().position(|a| a.id == acct) {
                self.from_dropdown.set_selected(pos as u32);
            }
        }

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }

    pub fn reopen_draft(
        &self,
        parent: &impl IsA<gtk::Widget>,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<String>,
        references: Vec<String>,
    ) {
        self.to_entry.set_text(to);
        self.subject_entry.set_text(subject);
        self.body_view.buffer().set_text(body);
        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }
}
