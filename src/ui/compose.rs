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
    pub signature: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboundAttachment {
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub content_type: String,
}

#[derive(Clone, Debug)]
pub struct ComposePayload {
    pub account_id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub draft_file_path: Option<PathBuf>,
    pub attachments: Vec<OutboundAttachment>,
}

pub struct ComposeDialog {
    dialog: adw::Dialog,
    to_entry: gtk::Entry,
    subject_entry: gtk::Entry,
    body_view: gtk::TextView,
    from_dropdown: gtk::DropDown,
    pub accounts: Rc<RefCell<Vec<AccountIdentity>>>,
    current_account_idx: Rc<std::cell::Cell<usize>>,
    in_reply_to: Rc<RefCell<Option<String>>>,
    references: Rc<RefCell<Vec<String>>>,
    draft_file_path: Rc<RefCell<Option<PathBuf>>>,
    drafts_dir: Rc<RefCell<Option<PathBuf>>>,
    debounce_timer: Rc<RefCell<Option<gtk::glib::SourceId>>>,
    pub attachments: Rc<RefCell<Vec<OutboundAttachment>>>,
    attachments_box: gtk::Box,
    #[allow(deprecated)]
    pub contacts_store: gtk::ListStore,
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
                    signature: Some("Brayan\nSent with Iris — Liquid Glass Mail".to_string()),
                },
                AccountIdentity {
                    id: "work".to_string(),
                    display: "Gmail (Work)".to_string(),
                    email: "brayan@work.com".to_string(),
                    signature: Some("Brayan | Hermes Audio Engine\nbrayan@work.com".to_string()),
                },
                AccountIdentity {
                    id: "icloud".to_string(),
                    display: "iCloud".to_string(),
                    email: "brayan@icloud.com".to_string(),
                    signature: Some("Brayan\nSent from Iris on Arch Linux".to_string()),
                },
            ]
        } else {
            accounts_in
                .iter()
                .map(|a| AccountIdentity {
                    id: a.id.clone(),
                    display: a.display_name.clone(),
                    email: a.email.clone(),
                    signature: a.signature.clone(),
                })
                .collect()
        };

        let accounts_rc = Rc::new(RefCell::new(accounts));
        let current_account_idx = Rc::new(std::cell::Cell::new(0usize));

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

        let attach_btn = gtk::Button::builder()
            .icon_name("mail-attachment-symbolic")
            .tooltip_text("Attach Files (Ctrl+Shift+A)")
            .build();
        attach_btn.add_css_class("flat");
        header.pack_end(&attach_btn);

        let preview_btn = gtk::ToggleButton::builder()
            .icon_name("view-reveal-symbolic")
            .tooltip_text("Toggle Markdown Preview (Ctrl+P)")
            .build();
        preview_btn.add_css_class("flat");
        header.pack_end(&preview_btn);

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

        let account_labels: Vec<String> = accounts_rc
            .borrow()
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

        // Contact Autocompletion
        #[allow(deprecated)]
        let completion = gtk::EntryCompletion::new();
        #[allow(deprecated)]
        let contacts_store = gtk::ListStore::new(&[glib::Type::STRING]);
        #[allow(deprecated)]
        {
            completion.set_model(Some(&contacts_store));
            completion.set_text_column(0);
            completion.set_minimum_key_length(1);
            completion.set_popup_completion(true);
            completion.set_inline_completion(true);
            to_entry.set_completion(Some(&completion));
        }

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

        // Attachments pill container
        let attachments = Rc::new(RefCell::new(Vec::<OutboundAttachment>::new()));
        let attachments_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        attachments_box.add_css_class("iris-compose-attachments");
        attachments_box.set_margin_start(8);
        attachments_box.set_margin_end(8);
        attachments_box.set_margin_top(4);
        attachments_box.set_margin_bottom(4);
        attachments_box.set_visible(false);
        fields_vbox.append(&attachments_box);

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

        // Dynamic Signature Swapping on Account Selection
        let body_view_for_drop = body_view.clone();
        let accounts_for_drop = accounts_rc.clone();
        let current_idx_drop = current_account_idx.clone();
        from_dropdown.connect_selected_notify(move |dropdown| {
            let new_idx = dropdown.selected() as usize;
            let old_idx = current_idx_drop.get();
            if new_idx == old_idx {
                return;
            }
            current_idx_drop.set(new_idx);
            let accounts = accounts_for_drop.borrow();
            let old_sig = accounts.get(old_idx).and_then(|a| a.signature.as_deref());
            let new_sig = accounts.get(new_idx).and_then(|a| a.signature.as_deref());
            let buffer = body_view_for_drop.buffer();
            let (start, end) = buffer.bounds();
            let current_text = buffer.text(&start, &end, false).to_string();
            let updated_text = swap_signature(&current_text, old_sig, new_sig);
            if updated_text != current_text {
                buffer.set_text(&updated_text);
            }
        });

        // Live Markdown Preview Panel
        let preview_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(12)
            .visible(false)
            .build();

        let preview_label = gtk::Label::builder()
            .use_markup(true)
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .yalign(0.0)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        preview_scrolled.set_child(Some(&preview_label));

        // Connect Markdown formatting shortcuts (Ctrl+B, Ctrl+I, Ctrl+K, Ctrl+P)
        {
            let body_ctrl = gtk::EventControllerKey::new();
            let preview_btn_key = preview_btn.clone();
            body_ctrl.connect_key_pressed(move |ctrl, key, _code, modifier| {
                if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                    let Some(view) = ctrl.widget().and_downcast::<gtk::TextView>() else {
                        return glib::Propagation::Proceed;
                    };
                    let buffer = view.buffer();

                    match key {
                        gtk::gdk::Key::b | gtk::gdk::Key::B => {
                            wrap_selection(&buffer, "**", "**");
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::i | gtk::gdk::Key::I => {
                            wrap_selection(&buffer, "*", "*");
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::k | gtk::gdk::Key::K => {
                            wrap_selection(&buffer, "[", "](https://)");
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::p | gtk::gdk::Key::P => {
                            preview_btn_key.set_active(!preview_btn_key.is_active());
                            return glib::Propagation::Stop;
                        }
                        _ => {}
                    }
                }
                glib::Propagation::Proceed
            });
            body_view.add_controller(body_ctrl);
        }

        // Connect Preview Toggle
        {
            let body_view_prev = body_view.clone();
            let prev_lbl = preview_label.clone();
            let scr_edit = scrolled.clone();
            let scr_prev = preview_scrolled.clone();

            preview_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    let buffer = body_view_prev.buffer();
                    let (s, e) = buffer.bounds();
                    let text = buffer.text(&s, &e, false).to_string();
                    let markup = crate::services::markdown::markdown_to_pango(&text);
                    prev_lbl.set_markup(&markup);
                    scr_edit.set_visible(false);
                    scr_prev.set_visible(true);
                } else {
                    scr_prev.set_visible(false);
                    scr_edit.set_visible(true);
                    body_view_prev.grab_focus();
                }
            });
        }

        scrolled.set_child(Some(&body_view));
        vbox.append(&scrolled);
        vbox.append(&preview_scrolled);

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
        let accounts_for_send = accounts_rc.clone();
        let draft_path_send = draft_file_path.clone();
        let debounce_timer_send = debounce_timer.clone();
        let attachments_for_send = attachments.clone();

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
            let accts = accounts_for_send.borrow();
            let acct = accts
                .get(acct_idx)
                .or_else(|| accts.first())
                .cloned()
                .unwrap_or_else(|| AccountIdentity {
                    id: "demo".to_string(),
                    display: "Personal".to_string(),
                    email: "bry@gmail.com".to_string(),
                    signature: None,
                });

            let draft_path = draft_path_send.borrow_mut().take();
            let atts = attachments_for_send.borrow().clone();

            let body_html = if crate::services::markdown::contains_markdown(&body) {
                Some(crate::services::markdown::markdown_to_html(&body))
            } else {
                None
            };

            let payload = ComposePayload {
                account_id: acct.id.to_string(),
                from: acct.email.to_string(),
                to,
                subject,
                body,
                body_html,
                in_reply_to: in_reply_to_clone.borrow().clone(),
                references: references_clone.borrow().clone(),
                draft_file_path: draft_path,
                attachments: atts,
            };

            diag_clone.close();
            on_send(payload);
        });

        // Attach Files (Paperclip button & Ctrl+Shift+A)
        let att_list = attachments.clone();
        let att_box = attachments_box.clone();
        let diag_for_file = dialog.clone();
        attach_btn.connect_clicked(move |_| {
            let file_dialog = gtk::FileDialog::builder()
                .title("Attach Files")
                .modal(true)
                .build();

            let list_c = att_list.clone();
            let box_c = att_box.clone();
            let parent_win = diag_for_file.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            file_dialog.open_multiple(parent_win.as_ref(), gio::Cancellable::NONE, move |result| {
                if let Ok(files) = result {
                    let count = files.n_items();
                    for i in 0..count {
                        if let Some(obj) = files.item(i) {
                            if let Ok(file) = obj.downcast::<gio::File>() {
                                if let Some(path) = file.path() {
                                    let filename = path
                                        .file_name()
                                        .and_then(|f| f.to_str())
                                        .unwrap_or("attachment")
                                        .to_string();
                                    let size_bytes = std::fs::metadata(&path)
                                        .map(|m| m.len())
                                        .unwrap_or(0);
                                    let content_type = crate::services::attachment::guess_content_type(&filename).to_string();

                                    list_c.borrow_mut().push(OutboundAttachment {
                                        filename,
                                        path,
                                        size_bytes,
                                        content_type,
                                    });
                                }
                            }
                        }
                    }
                    render_attachments(&list_c, &box_c);
                }
            });
        });

        let key_controller = gtk::EventControllerKey::new();
        let attach_trigger = attach_btn.clone();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            if state.contains(gtk::gdk::ModifierType::CONTROL_MASK)
                && state.contains(gtk::gdk::ModifierType::SHIFT_MASK)
                && (keyval == gtk::gdk::Key::A || keyval == gtk::gdk::Key::a)
            {
                attach_trigger.emit_clicked();
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        dialog.add_controller(key_controller);

        // Background Auto-Save after 1.5s idle typing
        let connect_change = {
            let drafts_dir = drafts_dir.clone();
            let draft_path = draft_file_path.clone();
            let debounce = debounce_timer.clone();
            let to_e = to_entry.clone();
            let sub_e = subject_entry.clone();
            let body_v = body_view.clone();
            let from_d = from_dropdown.clone();
            let accts = accounts_rc.clone();
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
                            let accts = accts_c.borrow();
                            let acct = accts
                                .get(acct_idx)
                                .or_else(|| accts.first())
                                .cloned()
                                .unwrap_or_else(|| AccountIdentity {
                                    id: "demo".to_string(),
                                    display: "Personal".to_string(),
                                    email: "bry@gmail.com".to_string(),
                                    signature: None,
                                });
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
            accounts: accounts_rc,
            current_account_idx,
            in_reply_to,
            references,
            draft_file_path,
            drafts_dir,
            debounce_timer,
            attachments,
            attachments_box,
            contacts_store,
        }
    }

    /// Updates the recipient autocompletion list with known contacts from Maildir.
    #[allow(deprecated)]
    pub fn update_contacts(&self, contacts: &[crate::services::contact::Contact]) {
        self.contacts_store.clear();
        for c in contacts {
            let addr = c.to_address_string();
            let iter = self.contacts_store.append();
            self.contacts_store.set_value(&iter, 0, &addr.to_value());
        }
    }

    pub fn set_drafts_dir(&self, dir: PathBuf) {
        *self.drafts_dir.borrow_mut() = Some(dir);
    }

    pub fn add_attachment(&self, path: PathBuf) {
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("attachment")
            .to_string();
        let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let content_type = crate::services::attachment::guess_content_type(&filename).to_string();

        self.attachments.borrow_mut().push(OutboundAttachment {
            filename,
            path,
            size_bytes,
            content_type,
        });
        render_attachments(&self.attachments, &self.attachments_box);
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        *self.draft_file_path.borrow_mut() = None;
        self.attachments.borrow_mut().clear();
        self.attachments_box.set_visible(false);
        self.to_entry.set_text("");
        self.subject_entry.set_text("");
        self.current_account_idx.set(0);
        self.from_dropdown.set_selected(0);

        let sig = self.accounts.borrow().first().and_then(|a| a.signature.clone());
        if let Some(s) = sig {
            let formatted = format_signature(&s);
            self.body_view.buffer().set_text(&formatted);
            let start_iter = self.body_view.buffer().start_iter();
            self.body_view.buffer().place_cursor(&start_iter);
        } else {
            self.body_view.buffer().set_text("");
        }

        *self.in_reply_to.borrow_mut() = None;
        self.references.borrow_mut().clear();
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
        self.attachments.borrow_mut().clear();
        self.attachments_box.set_visible(false);
        self.to_entry.set_text(to);
        let reply_subject = if subject.to_lowercase().starts_with("re:") {
            subject.to_string()
        } else {
            format!("Re: {subject}")
        };
        self.subject_entry.set_text(&reply_subject);

        let target_pos = if let Some(acct) = account_id {
            self.accounts
                .borrow()
                .iter()
                .position(|a| a.id == acct)
                .unwrap_or(0)
        } else {
            0
        };
        self.current_account_idx.set(target_pos);
        self.from_dropdown.set_selected(target_pos as u32);

        let quote = if !quote_body.is_empty() {
            format!("\n\n> {}", quote_body.replace('\n', "\n> "))
        } else {
            String::new()
        };

        let sig = self
            .accounts
            .borrow()
            .get(target_pos)
            .and_then(|a| a.signature.clone());
        let sig_part = sig.as_deref().map(format_signature).unwrap_or_default();

        self.body_view.buffer().set_text(&format!("{sig_part}{quote}"));
        let start_iter = self.body_view.buffer().start_iter();
        self.body_view.buffer().place_cursor(&start_iter);

        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;

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
        self.attachments.borrow_mut().clear();
        self.attachments_box.set_visible(false);
        self.to_entry.set_text(to);
        self.subject_entry.set_text(subject);
        self.body_view.buffer().set_text(body);
        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;
        *self.draft_file_path.borrow_mut() = draft_path;

        if let Some(acct) = account_id {
            if let Some(pos) = self.accounts.borrow().iter().position(|a| a.id == acct) {
                self.current_account_idx.set(pos);
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
        self.attachments.borrow_mut().clear();
        self.attachments_box.set_visible(false);
        self.to_entry.set_text("");
        let fwd_subject = if subject.to_lowercase().starts_with("fwd:") {
            subject.to_string()
        } else {
            format!("Fwd: {subject}")
        };
        self.subject_entry.set_text(&fwd_subject);

        let target_pos = if let Some(acct) = account_id {
            self.accounts
                .borrow()
                .iter()
                .position(|a| a.id == acct)
                .unwrap_or(0)
        } else {
            0
        };
        self.current_account_idx.set(target_pos);
        self.from_dropdown.set_selected(target_pos as u32);

        let sig = self
            .accounts
            .borrow()
            .get(target_pos)
            .and_then(|a| a.signature.clone());
        let sig_part = sig.as_deref().map(format_signature).unwrap_or_default();

        let body_content = format_forward_body(from, date_str, subject, to_str, original_body);
        let full_body = if !sig_part.is_empty() {
            format!("{sig_part}\n\n{body_content}")
        } else {
            body_content
        };
        self.body_view.buffer().set_text(&full_body);
        let start_iter = self.body_view.buffer().start_iter();
        self.body_view.buffer().place_cursor(&start_iter);

        *self.in_reply_to.borrow_mut() = None;
        self.references.borrow_mut().clear();

        self.dialog.present(Some(parent));
        self.to_entry.grab_focus();
    }

    /// Dynamically updates configured accounts and identity dropdown.
    pub fn set_accounts(&self, new_accounts: &[crate::services::config::AccountConfig]) {
        let mapped: Vec<AccountIdentity> = new_accounts
            .iter()
            .map(|a| AccountIdentity {
                id: a.id.clone(),
                display: a.display_name.clone(),
                email: a.email.clone(),
                signature: a.signature.clone(),
            })
            .collect();
        let labels: Vec<String> = mapped
            .iter()
            .map(|a| format!("{} <{}>", a.display, a.email))
            .collect();
        let slices: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        self.from_dropdown
            .set_model(Some(&gtk::StringList::new(&slices)));
        *self.accounts.borrow_mut() = mapped;
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
        self.attachments.borrow_mut().clear();
        self.attachments_box.set_visible(false);
        self.to_entry.set_text(to);
        self.subject_entry.set_text(subject);
        self.body_view.buffer().set_text(body);
        *self.in_reply_to.borrow_mut() = in_reply_to;
        *self.references.borrow_mut() = references;

        self.dialog.present(Some(parent));
        self.body_view.grab_focus();
    }
}

fn render_attachments(
    attachments: &Rc<RefCell<Vec<OutboundAttachment>>>,
    container: &gtk::Box,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let atts = attachments.borrow().clone();
    if atts.is_empty() {
        container.set_visible(false);
        return;
    }

    container.set_visible(true);
    for (idx, att) in atts.iter().enumerate() {
        let chip = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        chip.add_css_class("iris-attachment-pill");

        let icon = gtk::Image::from_icon_name("mail-attachment-symbolic");
        icon.set_icon_size(gtk::IconSize::Normal);
        chip.append(&icon);

        let size_str = crate::services::attachment::format_file_size(att.size_bytes);
        let label = gtk::Label::builder()
            .label(format!("{} ({size_str})", att.filename))
            .build();
        chip.append(&label);

        let remove_btn = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Remove attachment")
            .has_frame(false)
            .build();
        remove_btn.add_css_class("flat");
        remove_btn.add_css_class("circular");

        let atts_remove = attachments.clone();
        let box_remove = container.clone();
        remove_btn.connect_clicked(move |_| {
            let mut list = atts_remove.borrow_mut();
            if idx < list.len() {
                list.remove(idx);
            }
            drop(list);
            render_attachments(&atts_remove, &box_remove);
        });
        chip.append(&remove_btn);

        container.append(&chip);
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

/// Wraps the current text selection or cursor position with markdown syntax markers.
fn wrap_selection(buffer: &gtk::TextBuffer, prefix: &str, suffix: &str) {
    if let Some((mut start, mut end)) = buffer.selection_bounds() {
        let text = buffer.text(&start, &end, false).to_string();
        buffer.delete(&mut start, &mut end);
        let wrapped = format!("{prefix}{text}{suffix}");
        buffer.insert(&mut start, &wrapped);
    } else {
        let mut iter = buffer.iter_at_mark(&buffer.get_insert());
        let wrapped = format!("{prefix}{suffix}");
        buffer.insert(&mut iter, &wrapped);
    }
}

/// Formats a signature string according to standard email conventions (`\n\n-- \n{sig}`).
pub fn format_signature(sig: &str) -> String {
    let trimmed = sig.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("\n\n-- \n{trimmed}")
    }
}

/// Swaps an existing signature in the text buffer with a new account's signature.
/// If the old signature is present, it is replaced in-place.
/// If no old signature was present, the new signature is appended to the bottom.
pub fn swap_signature(body: &str, old_sig: Option<&str>, new_sig: Option<&str>) -> String {
    let formatted_new = new_sig.map(format_signature).unwrap_or_default();

    if let Some(old) = old_sig {
        let formatted_old = format_signature(old);
        if !formatted_old.is_empty() && body.contains(&formatted_old) {
            return body.replace(&formatted_old, &formatted_new);
        }
    }

    if !formatted_new.is_empty() {
        if body.is_empty() {
            formatted_new
        } else {
            format!("{}{}", body.trim_end(), formatted_new)
        }
    } else {
        body.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_signature() {
        assert_eq!(
            format_signature("Alex Rivera\nHermes"),
            "\n\n-- \nAlex Rivera\nHermes"
        );
        assert_eq!(format_signature("   "), "");
    }

    #[test]
    fn test_swap_signature() {
        let old = "Brayan\nPersonal";
        let new = "Brayan\nWork";
        let body = format!("Hello World!{}", format_signature(old));
        let swapped = swap_signature(&body, Some(old), Some(new));
        assert_eq!(swapped, format!("Hello World!{}", format_signature(new)));

        // Swapping to None removes signature
        let removed = swap_signature(&swapped, Some(new), None);
        assert_eq!(removed, "Hello World!");

        // Adding signature to clean body
        let added = swap_signature("Hello World!", None, Some(new));
        assert_eq!(added, format!("Hello World!{}", format_signature(new)));
    }

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
