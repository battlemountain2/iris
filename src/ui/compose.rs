#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use adw::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountIdentity {
    pub id: String,
    pub display: String,
    pub email: String,
}

#[derive(Clone, Debug)]
pub struct ComposePayload {
    pub account_id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub draft_file_path: Option<PathBuf>,
}

pub struct ComposeDialog {
    dialog: adw::Dialog,
    to_entry: gtk::Entry,
    subject_entry: gtk::Entry,
    body_view: gtk::TextView,
    from_dropdown: gtk::DropDown,
    pub accounts: Vec<AccountIdentity>,
    in_reply_to: Rc<RefCell<Option<String>>>,
    references: Rc<RefCell<Vec<String>>>,
    draft_file_path: Rc<RefCell<Option<PathBuf>>>,
    drafts_dir: Rc<RefCell<Option<PathBuf>>>,
    debounce_timer: Rc<RefCell<Option<gtk::glib::SourceId>>>,
}

impl ComposeDialog {
    pub fn new<F>(on_send: F) -> Self
    where
        F: Fn(ComposePayload) + 'static,
    {
        Self::with_accounts(&[], on_send)
    }

    pub fn with_accounts<F>(accounts_in: &[crate::services::config::AccountConfig], on_send: F) -> Self
    where
        F: Fn(ComposePayload) + 'static,
    {
        let accounts: Vec<AccountIdentity> = if accounts_in.is_empty() {
            vec![
                AccountIdentity {
                    id: "demo".to_string(),
                    display: "Gmail (Personal)".to_string(),
                    email: "bry@gmail.com".to_string(),
                },
                AccountIdentity {
                    id: "work".to_string(),
                    display: "Gmail (Work)".to_string(),
                    email: "brayan@work.com".to_string(),
                },
                AccountIdentity {
                    id: "icloud".to_string(),
                    display: "iCloud".to_string(),
                    email: "brayan@icloud.com".to_string(),
                },
            ]
        } else {
            accounts_in
                .iter()
                .map(|a| AccountIdentity {
                    id: a.id.clone(),
                    display: a.display_name.clone(),
                    email: a.email.clone(),
                })
                .collect()
        };

        let dialog = adw::Dialog::builder()
            .title("New Message")
            .content_width(700)
            .content_height(560)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.add_css_class("iris-compose");

        // Headerbar with Discard, Cancel and Send
        let header = adw::HeaderBar::new();
        header.set_show_start_title_buttons(false);
        header.set_show_end_title_buttons(false);

        let draft_file_path = Rc::new(RefCell::new(None::<PathBuf>));
        let drafts_dir = Rc::new(RefCell::new(None::<PathBuf>));
        let debounce_timer = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));

        let discard_btn = gtk::Button::builder()
            .label("Discard")
            .tooltip_text("Discard draft")
            .build();
        discard_btn.add_css_class("flat");
        let dialog_discard = dialog.clone();
        let draft_path_discard = draft_file_path.clone();
        let timer_discard = debounce_timer.clone();
        discard_btn.connect_clicked(move |_| {
            if let Some(src) = timer_discard.borrow_mut().take() {
                src.remove();
            }
            if let Some(ref path) = *draft_path_discard.borrow() {
                let _ = crate::services::draft::delete_draft(path);
            }
            *draft_path_discard.borrow_mut() = None;
            dialog_discard.close();
        });
        header.pack_start(&discard_btn);

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        cancel_btn.add_css_class("flat");
        let dialog_cancel = dialog.clone();
        let timer_cancel = debounce_timer.clone();
        cancel_btn.connect_clicked(move |_| {
            if let Some(src) = timer_cancel.borrow_mut().take() {
                src.remove();
            }
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

        let account_labels: Vec<String> = accounts
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
        let accounts_for_send = accounts.clone();
        let draft_path_send = draft_file_path.clone();
        let debounce_timer_send = debounce_timer.clone();

        send_btn.connect_clicked(move |_| {
            if let Some(src) = debounce_timer_send.borrow_mut().take() {
                src.remove();
            }
            let to = to_clone.text().trim().to_string();
            let subject = sub_clone.text().trim().to_string();
            let buffer = body_clone.buffer();
            let (start, end) = buffer.bounds();
            let body = buffer.text(&start, &end, false).to_string();

            let acct_idx = from_clone.selected() as usize;
            let acct = accounts_for_send
                .get(acct_idx)
                .unwrap_or(&accounts_for_send[0]);

            let draft_path = draft_path_send.borrow_mut().take();

            let payload = ComposePayload {
                account_id: acct.id.to_string(),
                from: acct.email.to_string(),
                to,
                subject,
                body,
                in_reply_to: in_reply_to_clone.borrow().clone(),
                references: references_clone.borrow().clone(),
                draft_file_path: draft_path,
            };

            diag_clone.close();
            on_send(payload);
        });

        // Background Auto-Save after 1.5s idle typing
        let connect_change = {
            let drafts_dir = drafts_dir.clone();
            let draft_path = draft_file_path.clone();
            let debounce = debounce_timer.clone();
            let to_e = to_entry.clone();
            let sub_e = subject_entry.clone();
            let body_v = body_view.clone();
            let from_d = from_dropdown.clone();
            let accts = accounts.clone();
            let in_rep = in_reply_to.clone();
            let refs = references.clone();

            Rc::new(move || {
                if let Some(src) = debounce.borrow_mut().take() {
                    src.remove();
                }
                let d_dir_c = drafts_dir.clone();
                let d_path_c = draft_path.clone();
                let timer_c = debounce.clone();
                let to_c = to_e.clone();
                let sub_c = sub_e.clone();
                let body_c = body_v.clone();
                let from_c = from_d.clone();
                let accts_c = accts.clone();
                let in_rep_c = in_rep.clone();
                let refs_c = refs.clone();

                let source = gtk::glib::timeout_add_local_once(
                    std::time::Duration::from_millis(1500),
                    move || {
                        timer_c.borrow_mut().take();
                        if let Some(ref d_dir) = *d_dir_c.borrow() {
                            let to = to_c.text().trim().to_string();
                            let subject = sub_c.text().trim().to_string();
                            let buf = body_c.buffer();
                            let (start, end) = buf.bounds();
                            let body = buf.text(&start, &end, false).to_string();

                            let acct_idx = from_c.selected() as usize;
                            let acct = accts_c.get(acct_idx).unwrap_or(&accts_c[0]);
                            let existing = d_path_c.borrow().clone();

                            if let Ok(new_path) = crate::services::draft::save_draft(
                                d_dir,
                                existing.as_deref(),
                                &acct.id,
                                &acct.email,
                                &to,
                                &subject,
                                &body,
                                in_rep_c.borrow().as_deref(),
                                &refs_c.borrow(),
                            ) {
                                *d_path_c.borrow_mut() = new_path;
                            }
                        }
                    },
                );
                *debounce.borrow_mut() = Some(source);
            })
        };

        {
            let on_chg = connect_change.clone();
            to_entry.connect_changed(move |_| on_chg());
        }
        {
            let on_chg = connect_change.clone();
            subject_entry.connect_changed(move |_| on_chg());
        }
        {
            let on_chg = connect_change.clone();
            body_view.buffer().connect_changed(move |_| on_chg());
        }

        Self {
            dialog,
            to_entry,
            subject_entry,
            body_view,
            from_dropdown,
            accounts,
            in_reply_to,
            references,
            draft_file_path,
            drafts_dir,
            debounce_timer,
        }
    }

    pub fn set_drafts_dir(&self, dir: PathBuf) {
        *self.drafts_dir.borrow_mut() = Some(dir);
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        *self.draft_file_path.borrow_mut() = None;
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
        *self.draft_file_path.borrow_mut() = None;
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
            if let Some(pos) = self.accounts.iter().position(|a| a.id == acct) {
                self.from_dropdown.set_selected(pos as u32);
            }
        }

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }

    pub fn present_edit_draft(
        &self,
        parent: &impl IsA<gtk::Widget>,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<String>,
        references: Vec<String>,
        account_id: Option<&str>,
        draft_path: Option<PathBuf>,
    ) {
        self.to_entry.set_text(to);
        self.subject_entry.set_text(subject);
        self.body_view.buffer().set_text(body);
        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;
        *self.draft_file_path.borrow_mut() = draft_path;

        if let Some(acct) = account_id {
            if let Some(pos) = self.accounts.iter().position(|a| a.id == acct) {
                self.from_dropdown.set_selected(pos as u32);
            }
        }

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }

    pub fn present_forward(
        &self,
        parent: &impl IsA<gtk::Widget>,
        subject: &str,
        from: &str,
        date_str: &str,
        to_str: &str,
        original_body: &str,
        account_id: Option<&str>,
    ) {
        *self.draft_file_path.borrow_mut() = None;
        self.to_entry.set_text("");
        let fwd_subject = if subject.to_lowercase().starts_with("fwd:") {
            subject.to_string()
        } else {
            format!("Fwd: {subject}")
        };
        self.subject_entry.set_text(&fwd_subject);

        let body_content = format_forward_body(from, date_str, subject, to_str, original_body);
        self.body_view.buffer().set_text(&body_content);

        *self.in_reply_to.borrow_mut() = None;
        self.references.borrow_mut().clear();

        if let Some(acct) = account_id {
            if let Some(pos) = self.accounts.iter().position(|a| a.id == acct) {
                self.from_dropdown.set_selected(pos as u32);
            }
        }

        self.dialog.present(Some(parent));
        self.to_entry.grab_focus();
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
        *self.draft_file_path.borrow_mut() = None;
        self.to_entry.set_text(to);
        self.subject_entry.set_text(subject);
        self.body_view.buffer().set_text(body);
        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }
}

/// Extracts reply-all recipients, honoring Reply-To, To, and Cc while excluding own accounts.
pub fn filter_reply_all_recipients(
    reply_to: Option<&str>,
    from: &str,
    to: &[String],
    cc: &[String],
    user_emails: &[&str],
) -> Vec<String> {
    let sender = reply_to.unwrap_or(from).trim();
    let mut recipients = Vec::new();

    let sender_lower = sender.to_lowercase();
    if !sender.is_empty() && !user_emails.iter().any(|u| sender_lower.contains(u)) {
        recipients.push(sender.to_string());
    }

    for addr in to.iter().chain(cc.iter()) {
        let addr_trim = addr.trim();
        let addr_lower = addr_trim.to_lowercase();
        if !addr_trim.is_empty()
            && !user_emails.iter().any(|u| addr_lower.contains(u))
            && !recipients.iter().any(|r| r.to_lowercase() == addr_lower)
        {
            recipients.push(addr_trim.to_string());
        }
    }

    if recipients.is_empty() && !sender.is_empty() {
        recipients.push(sender.to_string());
    }

    recipients
}

/// Formats standard forwarded message body block.
pub fn format_forward_body(
    from: &str,
    date_str: &str,
    subject: &str,
    to_str: &str,
    original_body: &str,
) -> String {
    format!(
        "\n\n---------- Forwarded message ---------\nFrom: {from}\nDate: {date_str}\nSubject: {subject}\nTo: {to_str}\n\n{original_body}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_reply_all_recipients_excludes_user_emails() {
        let user_emails = ["bry@gmail.com", "brayan@work.com", "brayan@icloud.com"];
        let from = "alex@hermes.dev";
        let to = vec![
            "bry@gmail.com".to_string(),
            "elena@noctalia.org".to_string(),
        ];
        let cc = vec![
            "brayan@work.com".to_string(),
            "dev@omarchy.org".to_string(),
        ];

        let recipients = filter_reply_all_recipients(None, from, &to, &cc, &user_emails);
        assert_eq!(recipients, vec![
            "alex@hermes.dev",
            "elena@noctalia.org",
            "dev@omarchy.org",
        ]);
    }

    #[test]
    fn test_filter_reply_all_honors_reply_to() {
        let user_emails = ["bry@gmail.com"];
        let from = "notifications@service.com";
        let reply_to = Some("support@service.com");
        let to = vec!["bry@gmail.com".to_string()];
        let cc = vec![];

        let recipients = filter_reply_all_recipients(reply_to, from, &to, &cc, &user_emails);
        assert_eq!(recipients, vec!["support@service.com"]);
    }

    #[test]
    fn test_format_forward_body() {
        let output = format_forward_body(
            "alex@hermes.dev",
            "Mon, Oct 25, 2026 at 10:30 AM",
            "Project Update",
            "team@hermes.dev",
            "All systems nominal.",
        );
        assert!(output.contains("---------- Forwarded message ---------"));
        assert!(output.contains("From: alex@hermes.dev"));
        assert!(output.contains("Subject: Project Update"));
        assert!(output.contains("All systems nominal."));
    }
}
