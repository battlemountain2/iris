#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;

use crate::services::config::IrisConfig;
use crate::ui::theme::ThemeManager;

pub struct SettingsDialog {
    dialog: adw::Dialog,
}

impl SettingsDialog {
    pub fn new(
        config: &IrisConfig,
        theme_manager: Rc<ThemeManager>,
        on_trigger_sync: Rc<dyn Fn()>,
        on_config_updated: Option<Rc<dyn Fn(IrisConfig)>>,
    ) -> Self {
        let live_config = Rc::new(RefCell::new(config.clone()));
        let dialog = adw::Dialog::builder()
            .title("Preferences & Accounts")
            .content_width(640)
            .content_height(580)
            .build();

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.add_css_class("iris-settings-dialog");

        // Headerbar with Title & Close button
        let headerbar = adw::HeaderBar::builder()
            .show_end_title_buttons(true)
            .build();
        vbox.append(&headerbar);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let page = adw::PreferencesPage::new();
        page.set_margin_start(16);
        page.set_margin_end(16);
        page.set_margin_top(8);
        page.set_margin_bottom(24);

        // Group 1: Configured Accounts
        let accts_group = adw::PreferencesGroup::builder()
            .title("Accounts")
            .description("Configured email accounts from ~/.config/iris/config.toml")
            .build();

        if config.accounts.is_empty() {
            let empty_row = adw::ActionRow::builder()
                .title("Demo Accounts Active")
                .subtitle("Add real accounts to ~/.config/iris/config.toml to sync personal mail")
                .build();
            empty_row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
            accts_group.add(&empty_row);
        } else {
            for acct in &config.accounts {
                let smtp_info = match (&acct.smtp_host, acct.smtp_port) {
                    (Some(host), Some(port)) => format!(" • SMTP: {}:{}", host, port),
                    (Some(host), None) => format!(" • SMTP: {}:587", host),
                    _ => " • SMTP: Local/Drafts only".to_string(),
                };
                let subtitle = format!("{}{}", acct.email, smtp_info);

                let exp_row = adw::ExpanderRow::builder()
                    .title(&acct.display_name)
                    .subtitle(&subtitle)
                    .expanded(false)
                    .build();

                exp_row.add_prefix(&gtk::Image::from_icon_name("mail-inbox-symbolic"));

                let badge = gtk::Label::new(Some("Active"));
                badge.add_css_class("iris-key-badge");
                exp_row.add_suffix(&badge);

                let sig_entry = adw::EntryRow::builder()
                    .title("Email Signature")
                    .text(acct.signature.as_deref().unwrap_or(""))
                    .show_apply_button(true)
                    .build();

                let cfg_clone = live_config.clone();
                let on_update_c = on_config_updated.clone();
                let acct_id = acct.id.clone();
                sig_entry.connect_apply(move |entry| {
                    let new_text = entry.text().trim().to_string();
                    let mut cfg = cfg_clone.borrow_mut();
                    if let Some(target) = cfg.accounts.iter_mut().find(|a| a.id == acct_id) {
                        target.signature = if new_text.is_empty() { None } else { Some(new_text) };
                    }
                    if let Err(e) = crate::services::config::save_config(&cfg) {
                        tracing::error!("Failed to save signature to config: {e}");
                    } else {
                        tracing::info!("Updated signature for account '{acct_id}'");
                    }
                    if let Some(ref cb) = on_update_c {
                        cb(cfg.clone());
                    }
                });

                exp_row.add_row(&sig_entry);
                accts_group.add(&exp_row);
            }
        }
        page.add(&accts_group);

        // Group 2: Appearance & Theme
        let theme_group = adw::PreferencesGroup::builder()
            .title("Appearance & Color Palette")
            .description("Liquid-glass theme presets inspired by Greek antiquity")
            .build();

        let themes = [
            ("attic_olive", "Attic Olive Glass", "Default liquid glass with golden olive accent"),
            ("spartan_crimson", "Spartan Crimson", "Deep lacquer crimson with gold contrast"),
            ("byzantine_bronze", "Byzantine Bronze", "Imperial bronze and purple glass"),
            ("attic_amber", "Attic Amber", "Warm sunlight amber with olive tones"),
        ];

        for (theme_id, title, desc) in themes {
            let row = adw::ActionRow::builder()
                .title(title)
                .subtitle(desc)
                .activatable(true)
                .build();

            let icon_color = match theme_id {
                "attic_olive" => "emblem-favorite-symbolic",
                "spartan_crimson" => "weather-clear-night-symbolic",
                "byzantine_bronze" => "software-properties-symbolic",
                _ => "weather-clear-symbolic",
            };
            row.add_prefix(&gtk::Image::from_icon_name(icon_color));

            let tm = theme_manager.clone();
            let tid = theme_id.to_string();
            row.connect_activated(move |_| {
                tm.set_theme(&tid);
                tracing::info!("Activated theme: {tid}");
            });

            theme_group.add(&row);
        }
        page.add(&theme_group);

        // Group 3: Sync & Automation
        let sync_group = adw::PreferencesGroup::builder()
            .title("Synchronization")
            .description("Background mailbox reconciliation & mbsync integration")
            .build();

        let sync_cmd_row = adw::ActionRow::builder()
            .title("Sync Command")
            .subtitle(&config.sync.command)
            .build();
        sync_cmd_row.add_prefix(&gtk::Image::from_icon_name("utilities-terminal-symbolic"));
        sync_group.add(&sync_cmd_row);

        let interval_mins = config.sync.interval_seconds / 60;
        let interval_row = adw::ActionRow::builder()
            .title("Auto-Sync Interval")
            .subtitle(format!("Every {} minutes ({}s)", interval_mins, config.sync.interval_seconds))
            .build();
        interval_row.add_prefix(&gtk::Image::from_icon_name("alarm-symbolic"));
        sync_group.add(&interval_row);

        let sync_now_row = adw::ActionRow::builder()
            .title("Synchronize Now")
            .subtitle("Trigger immediate background mail fetch")
            .activatable(true)
            .build();
        sync_now_row.add_prefix(&gtk::Image::from_icon_name("view-refresh-symbolic"));

        let on_sync_c = on_trigger_sync.clone();
        let diag_close = dialog.clone();
        sync_now_row.connect_activated(move |_| {
            on_sync_c();
            diag_close.close();
        });
        sync_group.add(&sync_now_row);

        page.add(&sync_group);

        // Group 4: Storage & Filesystem
        let storage_group = adw::PreferencesGroup::builder()
            .title("Filesystem & Storage")
            .description("Hermes & Maildir paths on disk")
            .build();

        let config_path_row = adw::ActionRow::builder()
            .title("Configuration File")
            .subtitle("~/.config/iris/config.toml")
            .activatable(true)
            .build();
        config_path_row.add_prefix(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
        config_path_row.connect_activated(|_| {
            if let Ok(home) = std::env::var("HOME") {
                let p = std::path::PathBuf::from(home).join(".config/iris");
                let _ = crate::services::attachment::open_in_hermes(&p);
            }
        });
        storage_group.add(&config_path_row);

        let maildir_row = adw::ActionRow::builder()
            .title("Local Maildir Root")
            .subtitle("~/.local/share/iris/maildir/")
            .activatable(true)
            .build();
        maildir_row.add_prefix(&gtk::Image::from_icon_name("folder-symbolic"));
        maildir_row.connect_activated(|_| {
            if let Ok(home) = std::env::var("HOME") {
                let p = std::path::PathBuf::from(home).join(".local/share/iris/maildir");
                let _ = crate::services::attachment::open_in_hermes(&p);
            }
        });
        storage_group.add(&maildir_row);

        page.add(&storage_group);

        // Group 5: About
        let about_group = adw::PreferencesGroup::builder()
            .title("About Iris")
            .description("Iris 0.1.0 • Liquid-Glass Email Client for Arch Linux & Hyprland")
            .build();

        let license_row = adw::ActionRow::builder()
            .title("License")
            .subtitle("GPL-3.0-or-later • Battle Mountain 2")
            .build();
        license_row.add_prefix(&gtk::Image::from_icon_name("help-about-symbolic"));
        about_group.add(&license_row);

        page.add(&about_group);

        scrolled.set_child(Some(&page));
        vbox.append(&scrolled);

        dialog.set_child(Some(&vbox));

        Self { dialog }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.dialog.present(Some(parent));
    }
}
