// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use adw::prelude::*;

use crate::services::config::AccountConfig;

/// Preset configurations for standard email providers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmailProvider {
    Gmail,
    ICloud,
    Fastmail,
    Outlook,
    Custom,
}

impl EmailProvider {
    pub fn smtp_host(self) -> &'static str {
        match self {
            Self::Gmail => "smtp.gmail.com",
            Self::ICloud => "smtp.mail.me.com",
            Self::Fastmail => "smtp.fastmail.com",
            Self::Outlook => "smtp.office365.com",
            Self::Custom => "",
        }
    }

    pub fn smtp_port(self) -> u16 {
        match self {
            Self::Gmail | Self::Fastmail | Self::Outlook => 587,
            Self::ICloud => 587,
            Self::Custom => 587,
        }
    }

    pub fn default_folder_name(self) -> &'static str {
        match self {
            Self::Gmail => "gmail",
            Self::ICloud => "icloud",
            Self::Fastmail => "fastmail",
            Self::Outlook => "outlook",
            Self::Custom => "mail",
        }
    }

    pub fn detect_from_email(email: &str) -> Self {
        let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();
        if domain.contains("gmail") || domain.contains("googlemail") {
            Self::Gmail
        } else if domain.contains("icloud") || domain.contains("me.com") || domain.contains("mac.com") {
            Self::ICloud
        } else if domain.contains("fastmail") {
            Self::Fastmail
        } else if domain.contains("outlook") || domain.contains("hotmail") || domain.contains("live.com") {
            Self::Outlook
        } else {
            Self::Custom
        }
    }
}

pub struct AccountDialog {
    dialog: adw::Dialog,
    name_entry: adw::EntryRow,
    email_entry: adw::EntryRow,
    maildir_entry: adw::EntryRow,
    smtp_host_entry: adw::EntryRow,
    smtp_port_entry: adw::EntryRow,
    smtp_user_entry: adw::EntryRow,
    smtp_pass_entry: adw::PasswordEntryRow,
    sig_entry: adw::EntryRow,
    error_banner: gtk::Label,
    on_account_added: Rc<RefCell<Option<Rc<dyn Fn(AccountConfig)>>>>,
}

impl AccountDialog {
    pub fn new() -> Self {
        let dialog = adw::Dialog::builder()
            .title("Add Email Account")
            .content_width(460)
            .content_height(580)
            .build();

        let root_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let header = adw::HeaderBar::new();
        header.set_show_end_title_buttons(false);
        header.set_show_start_title_buttons(false);

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        cancel_btn.add_css_class("flat");
        let dlg_for_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            dlg_for_cancel.close();
        });
        header.pack_start(&cancel_btn);

        let add_btn = gtk::Button::builder().label("Add Account").build();
        add_btn.add_css_class("suggested-action");
        header.pack_end(&add_btn);

        root_box.append(&header);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(20);

        // Error message banner
        let error_banner = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .visible(false)
            .build();
        error_banner.add_css_class("error");
        content_box.append(&error_banner);

        // Provider Preset Chips
        let preset_group = adw::PreferencesGroup::builder()
            .title("Account Provider")
            .description("Choose your provider for automatic server configuration")
            .build();

        let preset_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        preset_box.set_homogeneous(true);

        let btn_gmail = gtk::Button::with_label("Gmail");
        btn_gmail.add_css_class("flat");
        let btn_icloud = gtk::Button::with_label("iCloud");
        btn_icloud.add_css_class("flat");
        let btn_fastmail = gtk::Button::with_label("Fastmail");
        btn_fastmail.add_css_class("flat");
        let btn_outlook = gtk::Button::with_label("Outlook");
        btn_outlook.add_css_class("flat");
        let btn_custom = gtk::Button::with_label("Custom");
        btn_custom.add_css_class("flat");

        preset_box.append(&btn_gmail);
        preset_box.append(&btn_icloud);
        preset_box.append(&btn_fastmail);
        preset_box.append(&btn_outlook);
        preset_box.append(&btn_custom);
        preset_group.add(&preset_box);
        content_box.append(&preset_group);

        // Credentials Group
        let creds_group = adw::PreferencesGroup::builder()
            .title("Identity & Credentials")
            .build();

        let name_entry = adw::EntryRow::builder()
            .title("Account Name")
            .build();
        creds_group.add(&name_entry);

        let email_entry = adw::EntryRow::builder()
            .title("Email Address")
            .build();
        creds_group.add(&email_entry);

        let maildir_entry = adw::EntryRow::builder()
            .title("Local Maildir Path")
            .build();
        creds_group.add(&maildir_entry);

        content_box.append(&creds_group);

        // Outbound SMTP Server Group
        let server_group = adw::PreferencesGroup::builder()
            .title("Outgoing Server (SMTP)")
            .description("Required for sending emails and replies")
            .build();

        let smtp_host_entry = adw::EntryRow::builder()
            .title("SMTP Server Host")
            .build();
        server_group.add(&smtp_host_entry);

        let smtp_port_entry = adw::EntryRow::builder()
            .title("SMTP Port")
            .text("587")
            .build();
        server_group.add(&smtp_port_entry);

        let smtp_user_entry = adw::EntryRow::builder()
            .title("SMTP Username")
            .build();
        server_group.add(&smtp_user_entry);

        let smtp_pass_entry = adw::PasswordEntryRow::builder()
            .title("Password / App Password")
            .build();
        server_group.add(&smtp_pass_entry);

        let sig_entry = adw::EntryRow::builder()
            .title("Email Signature")
            .text("Sent with Iris — Liquid Glass Mail")
            .build();
        server_group.add(&sig_entry);

        content_box.append(&server_group);

        // Preset button handlers
        {
            let host = smtp_host_entry.clone();
            let port = smtp_port_entry.clone();
            let maildir = maildir_entry.clone();
            btn_gmail.connect_clicked(move |_| {
                host.set_text(EmailProvider::Gmail.smtp_host());
                port.set_text(&EmailProvider::Gmail.smtp_port().to_string());
                if maildir.text().is_empty() {
                    maildir.set_text("~/Mail/gmail");
                }
            });
        }
        {
            let host = smtp_host_entry.clone();
            let port = smtp_port_entry.clone();
            let maildir = maildir_entry.clone();
            btn_icloud.connect_clicked(move |_| {
                host.set_text(EmailProvider::ICloud.smtp_host());
                port.set_text(&EmailProvider::ICloud.smtp_port().to_string());
                if maildir.text().is_empty() {
                    maildir.set_text("~/Mail/icloud");
                }
            });
        }
        {
            let host = smtp_host_entry.clone();
            let port = smtp_port_entry.clone();
            let maildir = maildir_entry.clone();
            btn_fastmail.connect_clicked(move |_| {
                host.set_text(EmailProvider::Fastmail.smtp_host());
                port.set_text(&EmailProvider::Fastmail.smtp_port().to_string());
                if maildir.text().is_empty() {
                    maildir.set_text("~/Mail/fastmail");
                }
            });
        }
        {
            let host = smtp_host_entry.clone();
            let port = smtp_port_entry.clone();
            let maildir = maildir_entry.clone();
            btn_outlook.connect_clicked(move |_| {
                host.set_text(EmailProvider::Outlook.smtp_host());
                port.set_text(&EmailProvider::Outlook.smtp_port().to_string());
                if maildir.text().is_empty() {
                    maildir.set_text("~/Mail/outlook");
                }
            });
        }
        {
            let host = smtp_host_entry.clone();
            let port = smtp_port_entry.clone();
            btn_custom.connect_clicked(move |_| {
                host.set_text("");
                port.set_text("587");
            });
        }

        // Auto-detect provider and autofill username from email address
        {
            let host = smtp_host_entry.clone();
            let port = smtp_port_entry.clone();
            let user = smtp_user_entry.clone();
            let name = name_entry.clone();
            let maildir = maildir_entry.clone();

            email_entry.connect_changed(move |entry| {
                let text = entry.text().trim().to_string();
                if text.contains('@') {
                    if user.text().is_empty() {
                        user.set_text(&text);
                    }
                    if name.text().is_empty() {
                        let local_part = text.split('@').next().unwrap_or("");
                        let capitalized = local_part
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string() + &local_part[1..])
                            .unwrap_or_default();
                        name.set_text(&capitalized);
                    }

                    let provider = EmailProvider::detect_from_email(&text);
                    if provider != EmailProvider::Custom && host.text().is_empty() {
                        host.set_text(provider.smtp_host());
                        port.set_text(&provider.smtp_port().to_string());
                        if maildir.text().is_empty() {
                            maildir.set_text(&format!("~/Mail/{}", provider.default_folder_name()));
                        }
                    }
                }
            });
        }

        scrolled.set_child(Some(&content_box));
        root_box.append(&scrolled);
        dialog.set_child(Some(&root_box));

        let on_account_added: Rc<RefCell<Option<Rc<dyn Fn(AccountConfig)>>>> =
            Rc::new(RefCell::new(None));

        // Validation and Submission Handler
        {
            let dlg_close = dialog.clone();
            let name_e = name_entry.clone();
            let email_e = email_entry.clone();
            let maildir_e = maildir_entry.clone();
            let host_e = smtp_host_entry.clone();
            let port_e = smtp_port_entry.clone();
            let user_e = smtp_user_entry.clone();
            let pass_e = smtp_pass_entry.clone();
            let sig_e = sig_entry.clone();
            let err_lbl = error_banner.clone();
            let cb_ref = on_account_added.clone();

            add_btn.connect_clicked(move |_| {
                let name = name_e.text().trim().to_string();
                let email = email_e.text().trim().to_string();

                if name.is_empty() {
                    err_lbl.set_label("Please provide an account name.");
                    err_lbl.set_visible(true);
                    return;
                }
                if email.is_empty() || !email.contains('@') || !email.contains('.') {
                    err_lbl.set_label("Please enter a valid email address.");
                    err_lbl.set_visible(true);
                    return;
                }

                err_lbl.set_visible(false);

                // Generate ID from name or email prefix
                let id = sanitize_account_id(&name, &email);

                let maildir_path = {
                    let raw = maildir_e.text().trim().to_string();
                    if raw.is_empty() {
                        None
                    } else if raw.starts_with("~/") {
                        if let Ok(home) = std::env::var("HOME") {
                            Some(raw.replacen('~', &home, 1))
                        } else {
                            Some(raw)
                        }
                    } else {
                        Some(raw)
                    }
                };

                let smtp_host = {
                    let h = host_e.text().trim().to_string();
                    if h.is_empty() { None } else { Some(h) }
                };

                let smtp_port = port_e.text().trim().parse::<u16>().ok();

                let smtp_username = {
                    let u = user_e.text().trim().to_string();
                    if u.is_empty() { None } else { Some(u) }
                };

                let smtp_password = {
                    let p = pass_e.text().trim().to_string();
                    if p.is_empty() { None } else { Some(p) }
                };

                let signature = {
                    let s = sig_e.text().trim().to_string();
                    if s.is_empty() { None } else { Some(s) }
                };

                // Initialize maildir directory structure on disk
                if let Some(ref mpath) = maildir_path {
                    let root = PathBuf::from(mpath);
                    let _ = crate::services::maildir::ensure_maildir_folder(&root.join("INBOX"));
                    let _ = crate::services::maildir::ensure_maildir_folder(&root.join("Sent"));
                    let _ = crate::services::maildir::ensure_maildir_folder(&root.join("Archive"));
                    let _ = crate::services::maildir::ensure_maildir_folder(&root.join("Trash"));
                    let _ = crate::services::maildir::ensure_maildir_folder(&root.join("Drafts"));
                }

                let new_acct = AccountConfig {
                    id,
                    display_name: name,
                    email,
                    maildir_path,
                    smtp_host,
                    smtp_port,
                    smtp_username,
                    smtp_password,
                    signature,
                };

                if let Some(ref cb) = *cb_ref.borrow() {
                    cb(new_acct);
                }

                dlg_close.close();
            });
        }

        Self {
            dialog,
            name_entry,
            email_entry,
            maildir_entry,
            smtp_host_entry,
            smtp_port_entry,
            smtp_user_entry,
            smtp_pass_entry,
            sig_entry,
            error_banner,
            on_account_added,
        }
    }

    pub fn set_on_account_added<F>(&self, callback: F)
    where
        F: Fn(AccountConfig) + 'static,
    {
        *self.on_account_added.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.error_banner.set_visible(false);
        self.name_entry.set_text("");
        self.email_entry.set_text("");
        self.maildir_entry.set_text("");
        self.smtp_host_entry.set_text("");
        self.smtp_port_entry.set_text("587");
        self.smtp_user_entry.set_text("");
        self.smtp_pass_entry.set_text("");
        self.sig_entry.set_text("Sent with Iris — Liquid Glass Mail");
        self.dialog.present(Some(parent));
    }
}

pub fn sanitize_account_id(name: &str, email: &str) -> String {
    let base = if !name.trim().is_empty() {
        name.trim()
    } else {
        email.split('@').next().unwrap_or("account")
    };
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in base.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && (c == ' ' || c == '-' || c == '_' || c == '(' || c == ')') {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "account".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_provider_detection() {
        assert_eq!(EmailProvider::detect_from_email("user@gmail.com"), EmailProvider::Gmail);
        assert_eq!(EmailProvider::detect_from_email("me@icloud.com"), EmailProvider::ICloud);
        assert_eq!(EmailProvider::detect_from_email("me@fastmail.com"), EmailProvider::Fastmail);
        assert_eq!(EmailProvider::detect_from_email("user@outlook.com"), EmailProvider::Outlook);
        assert_eq!(EmailProvider::detect_from_email("user@customdomain.io"), EmailProvider::Custom);
    }

    #[test]
    fn test_sanitize_account_id() {
        assert_eq!(sanitize_account_id("Gmail (Personal)", "user@gmail.com"), "gmail-personal");
        assert_eq!(sanitize_account_id("Work Email", "alex@work.org"), "work-email");
        assert_eq!(sanitize_account_id("", "test.user@org.net"), "testuser");
    }
}
