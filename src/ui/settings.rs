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
        on_add_account: Option<Rc<dyn Fn()>>,
    ) -> Self {
        let live_config = Rc::new(RefCell::new(config.clone()));
        let dialog = adw::Dialog::builder()
            .title("Preferences & Accounts")
            .content_width(880)
            .content_height(620)
            .build();
        dialog.add_css_class("iris-dialog");
        dialog.add_css_class("iris-settings-dialog");

        // Two-pane horizontal split (Hermes & macOS Mail architecture)
        let main_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        // -----------------------------------------------------------------
        // LEFT PANE: Navigation Sidebar
        // -----------------------------------------------------------------
        let nav_vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);
        nav_vbox.add_css_class("settings-navigation");
        nav_vbox.set_size_request(220, -1);

        let heading = gtk::Label::builder()
            .label("PREFERENCES")
            .xalign(0.0)
            .build();
        heading.add_css_class("settings-nav-heading");
        nav_vbox.append(&heading);

        let filter_entry = gtk::SearchEntry::builder()
            .placeholder_text("Filter settings...")
            .build();
        filter_entry.add_css_class("settings-nav-filter");
        nav_vbox.append(&filter_entry);

        let nav_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
        nav_list.set_vexpand(true);
        nav_vbox.append(&nav_list);

        main_hbox.append(&nav_vbox);

        // -----------------------------------------------------------------
        // RIGHT PANE: Headerbar & Stack
        // -----------------------------------------------------------------
        let right_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        right_vbox.set_hexpand(true);
        right_vbox.set_vexpand(true);

        let headerbar = adw::HeaderBar::builder()
            .show_end_title_buttons(true)
            .build();
        right_vbox.append(&headerbar);

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Navigation state tracker
        let nav_buttons: Rc<RefCell<Vec<(String, gtk::Button, String)>>> = Rc::new(RefCell::new(Vec::new()));
        let stack_ref = stack.clone();
        let headerbar_ref = headerbar.clone();
        let nav_buttons_ref = nav_buttons.clone();

        let select_tab = Rc::new(move |target_id: &str, title: &str| {
            stack_ref.set_visible_child_name(target_id);
            let win_title = adw::WindowTitle::builder()
                .title(title)
                .build();
            headerbar_ref.set_title_widget(Some(&win_title));

            for (id, btn, _) in nav_buttons_ref.borrow().iter() {
                if id == target_id {
                    btn.add_css_class("settings-nav-active");
                } else {
                    btn.remove_css_class("settings-nav-active");
                }
            }
        });

        fn make_nav_button(
            id: &str,
            label: &str,
            icon_name: &str,
            select_tab: Rc<dyn Fn(&str, &str)>,
        ) -> gtk::Button {
            let btn = gtk::Button::new();
            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);

            let icon = gtk::Image::from_icon_name(icon_name);
            icon.set_icon_size(gtk::IconSize::Normal);
            icon.add_css_class("settings-nav-icon");
            hbox.append(&icon);

            let lbl = gtk::Label::builder()
                .label(label)
                .xalign(0.0)
                .hexpand(true)
                .build();
            hbox.append(&lbl);

            btn.set_child(Some(&hbox));
            btn.add_css_class("flat");
            btn.add_css_class("settings-nav-btn");

            let tab_id = id.to_string();
            let tab_title = label.to_string();
            btn.connect_clicked(move |_| {
                select_tab(&tab_id, &tab_title);
            });
            btn
        }

        fn wrap_scroll(widget: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
            let scrolled = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vscrollbar_policy(gtk::PolicyType::Automatic)
                .vexpand(true)
                .hexpand(true)
                .child(widget)
                .build();
            scrolled
        }

        // =================================================================
        // 1. ACCOUNTS PAGE
        // =================================================================
        let page_accounts = adw::PreferencesPage::new();
        page_accounts.add_css_class("settings-page-content");

        let accts_group = adw::PreferencesGroup::builder()
            .title("Email Accounts")
            .description("Configured email accounts from ~/.config/iris/config.toml")
            .build();

        let add_row = adw::ActionRow::builder()
            .title("Add Email Account...")
            .subtitle("Connect a new Gmail, iCloud, Fastmail, Outlook, or IMAP account")
            .activatable(true)
            .build();
        add_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::plus_icon()));
        let add_chevron = gtk::Image::from_icon_name(crate::ui::icons::chevron_right_icon());
        add_chevron.add_css_class("dim-label");
        add_row.add_suffix(&add_chevron);

        if let Some(ref on_add) = on_add_account {
            let on_add_c = on_add.clone();
            add_row.connect_activated(move |_| {
                on_add_c();
            });
        }
        accts_group.add(&add_row);

        if config.accounts.is_empty() {
            let empty_row = adw::ActionRow::builder()
                .title("No Custom Accounts")
                .subtitle("Click above to add your Gmail, iCloud, or work email account")
                .build();
            empty_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::account_icon("demo", "")));
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

                let icon_name = crate::ui::icons::account_icon(&acct.display_name, &acct.email);
                exp_row.add_prefix(&gtk::Image::from_icon_name(icon_name));

                let badge = gtk::Label::new(Some("Active"));
                badge.add_css_class("iris-key-badge");
                exp_row.add_suffix(&badge);

                let sig_entry = adw::EntryRow::builder()
                    .title("Email Signature")
                    .text(acct.signature.as_deref().unwrap_or(""))
                    .show_apply_button(true)
                    .build();
                sig_entry.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::drafts_icon()));

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

                let remove_btn = gtk::Button::builder()
                    .label("Remove Account")
                    .icon_name(crate::ui::icons::trash_icon())
                    .valign(gtk::Align::Center)
                    .build();
                remove_btn.add_css_class("destructive-action");
                remove_btn.add_css_class("flat");

                let cfg_clone_del = live_config.clone();
                let on_update_del = on_config_updated.clone();
                let acct_id_del = acct.id.clone();
                let exp_row_del = exp_row.clone();
                let accts_group_del = accts_group.clone();

                remove_btn.connect_clicked(move |_| {
                    let mut cfg = cfg_clone_del.borrow_mut();
                    cfg.accounts.retain(|a| a.id != acct_id_del);
                    if let Err(e) = crate::services::config::save_config(&cfg) {
                        tracing::error!("Failed to save config after removing account: {e}");
                    } else {
                        tracing::info!("Removed account '{acct_id_del}'");
                    }
                    accts_group_del.remove(&exp_row_del);
                    if let Some(ref cb) = on_update_del {
                        cb(cfg.clone());
                    }
                });

                let remove_row = adw::ActionRow::builder()
                    .title("Remove from Iris")
                    .subtitle("Disconnects this account and removes it from config.toml")
                    .build();
                remove_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::trash_icon()));
                remove_row.add_suffix(&remove_btn);
                exp_row.add_row(&remove_row);

                accts_group.add(&exp_row);
            }
        }
        page_accounts.add(&accts_group);
        stack.add_named(&wrap_scroll(&page_accounts), Some("accounts"));

        let btn_accounts = make_nav_button("accounts", "Accounts", crate::ui::icons::inbox_icon(), select_tab.clone());
        nav_list.append(&btn_accounts);
        nav_buttons.borrow_mut().push(("accounts".to_string(), btn_accounts, "Accounts".to_string()));

        // =================================================================
        // 2. GENERAL & SYNC PAGE
        // =================================================================
        let page_sync = adw::PreferencesPage::new();
        page_sync.add_css_class("settings-page-content");

        let sync_group = adw::PreferencesGroup::builder()
            .title("Synchronization & Schedule")
            .description("Background mailbox reconciliation & mbsync integration")
            .build();

        let sync_cmd_row = adw::EntryRow::builder()
            .title("Sync Command")
            .text(&config.sync.command)
            .show_apply_button(true)
            .build();
        sync_cmd_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::server_icon()));

        let cfg_clone_sync = live_config.clone();
        let on_update_sync = on_config_updated.clone();
        sync_cmd_row.connect_apply(move |entry| {
            let cmd = entry.text().trim().to_string();
            let mut cfg = cfg_clone_sync.borrow_mut();
            cfg.sync.command = cmd;
            let _ = crate::services::config::save_config(&cfg);
            if let Some(ref cb) = on_update_sync {
                cb(cfg.clone());
            }
        });
        sync_group.add(&sync_cmd_row);

        let interval_mins = config.sync.interval_seconds / 60;
        let interval_row = adw::ActionRow::builder()
            .title("Auto-Sync Interval")
            .subtitle(format!("Every {} minutes ({}s)", interval_mins, config.sync.interval_seconds))
            .build();
        interval_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::refresh_icon()));
        sync_group.add(&interval_row);

        let sync_now_row = adw::ActionRow::builder()
            .title("Synchronize Now")
            .subtitle("Trigger immediate background mail fetch")
            .activatable(true)
            .build();
        sync_now_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::refresh_icon()));

        let on_sync_c = on_trigger_sync.clone();
        let diag_close = dialog.clone();
        sync_now_row.connect_activated(move |_| {
            on_sync_c();
            diag_close.close();
        });
        sync_group.add(&sync_now_row);
        page_sync.add(&sync_group);

        let notif_group = adw::PreferencesGroup::builder()
            .title("Notifications")
            .description("Desktop alerts on arrival of new emails")
            .build();

        let notif_switch = adw::SwitchRow::builder()
            .title("Desktop Notifications")
            .subtitle("Show notification badge and banner when mail arrives")
            .active(true)
            .build();
        notif_switch.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::feed_icon()));
        notif_group.add(&notif_switch);
        page_sync.add(&notif_group);

        stack.add_named(&wrap_scroll(&page_sync), Some("sync"));

        let btn_sync = make_nav_button("sync", "General & Sync", crate::ui::icons::refresh_icon(), select_tab.clone());
        nav_list.append(&btn_sync);
        nav_buttons.borrow_mut().push(("sync".to_string(), btn_sync, "General & Sync".to_string()));

        // =================================================================
        // 3. COMPOSING PAGE
        // =================================================================
        let page_composing = adw::PreferencesPage::new();
        page_composing.add_css_class("settings-page-content");

        let compose_group = adw::PreferencesGroup::builder()
            .title("Composing")
            .description("Settings for creating and sending emails")
            .build();

        let bcc_switch = adw::SwitchRow::builder()
            .title("Always Bcc Myself")
            .subtitle("Automatically add your email address to the Bcc recipient field")
            .active(false)
            .build();
        bcc_switch.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::user_icon()));
        compose_group.add(&bcc_switch);

        let undo_send_row = adw::ActionRow::builder()
            .title("Undo Send Delay")
            .subtitle("10 seconds (gives you a window to cancel sending)")
            .build();
        undo_send_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::drafts_icon()));
        compose_group.add(&undo_send_row);

        let markdown_switch = adw::SwitchRow::builder()
            .title("Markdown & Rich Text Formatting")
            .subtitle("Render bold, italics, links, and codeblocks in outgoing messages")
            .active(true)
            .build();
        markdown_switch.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::compose_icon()));
        compose_group.add(&markdown_switch);

        page_composing.add(&compose_group);
        stack.add_named(&wrap_scroll(&page_composing), Some("composing"));

        let btn_composing = make_nav_button("composing", "Composing", crate::ui::icons::drafts_icon(), select_tab.clone());
        nav_list.append(&btn_composing);
        nav_buttons.borrow_mut().push(("composing".to_string(), btn_composing, "Composing".to_string()));

        // =================================================================
        // 4. MESSAGE LIST PAGE
        // =================================================================
        let page_messages = adw::PreferencesPage::new();
        page_messages.add_css_class("settings-page-content");

        let list_group = adw::PreferencesGroup::builder()
            .title("Message List")
            .description("Appearance and layout of conversation threads")
            .build();

        let preview_lines_row = adw::ActionRow::builder()
            .title("Preview Snippet")
            .subtitle("2 lines (iOS Mail default)")
            .build();
        preview_lines_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::filter_icon()));
        list_group.add(&preview_lines_row);

        let avatars_switch = adw::SwitchRow::builder()
            .title("Show Contact Monograms")
            .subtitle("Display circular sender initials avatar alongside messages")
            .active(true)
            .build();
        avatars_switch.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::user_icon()));
        list_group.add(&avatars_switch);

        let swipe_group = adw::PreferencesGroup::builder()
            .title("Swipe Actions")
            .description("Gesture interactions on touchpad and mouse")
            .build();

        let swipe_right_row = adw::ActionRow::builder()
            .title("Swipe Right")
            .subtitle("Archive message (Royal Blue)")
            .build();
        swipe_right_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::archive_icon()));
        swipe_group.add(&swipe_right_row);

        let swipe_left_row = adw::ActionRow::builder()
            .title("Swipe Left")
            .subtitle("Move to Trash (Apple Crimson)")
            .build();
        swipe_left_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::trash_icon()));
        swipe_group.add(&swipe_left_row);

        page_messages.add(&list_group);
        page_messages.add(&swipe_group);
        stack.add_named(&wrap_scroll(&page_messages), Some("messages"));

        let btn_messages = make_nav_button("messages", "Message List", crate::ui::icons::filter_icon(), select_tab.clone());
        nav_list.append(&btn_messages);
        nav_buttons.borrow_mut().push(("messages".to_string(), btn_messages, "Message List".to_string()));

        // =================================================================
        // 5. VIEWING & PRIVACY PAGE
        // =================================================================
        let page_privacy = adw::PreferencesPage::new();
        page_privacy.add_css_class("settings-page-content");

        let privacy_group = adw::PreferencesGroup::builder()
            .title("Mail Privacy Protection")
            .description("Apple iOS Mail Privacy Protection prevents senders from tracking you")
            .build();

        let privacy_switch = adw::SwitchRow::builder()
            .title("Protect Mail Activity")
            .subtitle("Hide IP address and block invisible 1x1 tracking beacons")
            .active(true)
            .build();
        privacy_switch.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::shield_icon()));
        privacy_group.add(&privacy_switch);

        let confirm_delete = adw::SwitchRow::builder()
            .title("Ask Before Emptying Trash")
            .subtitle("Present confirmation modal before permanently deleting purged mail")
            .active(true)
            .build();
        confirm_delete.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::trash_icon()));
        privacy_group.add(&confirm_delete);

        page_privacy.add(&privacy_group);
        stack.add_named(&wrap_scroll(&page_privacy), Some("privacy"));

        let btn_privacy = make_nav_button("privacy", "Viewing & Privacy", crate::ui::icons::shield_icon(), select_tab.clone());
        nav_list.append(&btn_privacy);
        nav_buttons.borrow_mut().push(("privacy".to_string(), btn_privacy, "Viewing & Privacy".to_string()));

        // =================================================================
        // 6. APPEARANCE & THEMES PAGE
        // =================================================================
        let page_themes = adw::PreferencesPage::new();
        page_themes.add_css_class("settings-page-content");

        let theme_group = adw::PreferencesGroup::builder()
            .title("Appearance & Color Palette")
            .description("Liquid-glass theme presets inspired by Greek antiquity & modern palettes")
            .build();

        let themes = [
            ("iris-dark", "iOS Dark (Default)", "Apple iOS Mail dark palette with pure charcoal canvas & sapphire blue", crate::ui::icons::palette_icon()),
            ("iris-light", "iOS Light", "Apple iOS Mail crisp paper canvas with vibrant system blue", crate::ui::icons::palette_icon()),
            ("attic-olive", "Attic Olive Glass", "Default liquid glass with golden olive accent", crate::ui::icons::palette_icon()),
            ("corinthian-bronze", "Corinthian Bronze", "Warm metallic copper and burnished bronze accents", crate::ui::icons::palette_icon()),
            ("spartan-crimson", "Spartan Crimson", "Deep lacquer crimson with gold contrast", crate::ui::icons::palette_icon()),
            ("byzantine-bronze", "Byzantine Bronze", "Imperial gold and regal bronze sheen", crate::ui::icons::palette_icon()),
            ("attic-amber", "Attic Amber", "Luminous honey and classical amber warmth", crate::ui::icons::palette_icon()),
            ("catppuccin", "Catppuccin Mocha", "Cozy soothing pastel dark theme with lavender accents", crate::ui::icons::palette_icon()),
            ("tokyo-night", "Tokyo Night", "Vibrant neon cyberpunk blue & purple accents", crate::ui::icons::palette_icon()),
            ("everforest", "Everforest", "Comforting natural green forest palette", crate::ui::icons::palette_icon()),
            ("rose-pine", "Rosé Pine", "Earthy warm dark palette with soft rose highlights", crate::ui::icons::palette_icon()),
            ("midnight", "Midnight OLED", "Pure ink black canvas optimized for OLED displays", crate::ui::icons::palette_icon()),
        ];

        let current_theme_id = theme_manager.get_current_theme();
        let check_images: Rc<RefCell<Vec<(String, gtk::Image)>>> = Rc::new(RefCell::new(Vec::new()));

        for (theme_id, title, desc, icon_name) in themes {
            let row = adw::ActionRow::builder()
                .title(title)
                .subtitle(desc)
                .activatable(true)
                .build();

            row.add_prefix(&gtk::Image::from_icon_name(icon_name));

            let check_icon = gtk::Image::from_icon_name("iris-check-symbolic");
            check_icon.add_css_class("accent");
            check_icon.set_visible(current_theme_id == theme_id);
            row.add_suffix(&check_icon);

            check_images.borrow_mut().push((theme_id.to_string(), check_icon.clone()));

            let tm = theme_manager.clone();
            let tid = theme_id.to_string();
            let check_images_clone = check_images.clone();
            row.connect_activated(move |_| {
                tm.set_theme(&tid);
                tracing::info!("Activated theme: {tid}");
                for (id, img) in check_images_clone.borrow().iter() {
                    img.set_visible(id == &tid);
                }
            });

            theme_group.add(&row);
        }
        page_themes.add(&theme_group);
        stack.add_named(&wrap_scroll(&page_themes), Some("themes"));

        let btn_themes = make_nav_button("themes", "Appearance & Themes", crate::ui::icons::palette_icon(), select_tab.clone());
        nav_list.append(&btn_themes);
        nav_buttons.borrow_mut().push(("themes".to_string(), btn_themes, "Appearance & Themes".to_string()));

        // =================================================================
        // 7. SHORTCUTS & ABOUT PAGE
        // =================================================================
        let page_about = adw::PreferencesPage::new();
        page_about.add_css_class("settings-page-content");

        let storage_group = adw::PreferencesGroup::builder()
            .title("Filesystem & Storage")
            .description("Hermes & Maildir paths on disk")
            .build();

        let config_path_row = adw::ActionRow::builder()
            .title("Configuration File")
            .subtitle("~/.config/iris/config.toml")
            .activatable(true)
            .build();
        config_path_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::server_icon()));
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
        maildir_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::folder_icon()));
        maildir_row.connect_activated(|_| {
            if let Ok(home) = std::env::var("HOME") {
                let p = std::path::PathBuf::from(home).join(".local/share/iris/maildir");
                let _ = crate::services::attachment::open_in_hermes(&p);
            }
        });
        storage_group.add(&maildir_row);
        page_about.add(&storage_group);

        let about_group = adw::PreferencesGroup::builder()
            .title("About Iris")
            .description("Iris 0.1.0 • Liquid-Glass Email Client for Arch Linux & Hyprland")
            .build();

        let license_row = adw::ActionRow::builder()
            .title("License")
            .subtitle("GPL-3.0-or-later • Battle Mountain 2")
            .build();
        license_row.add_prefix(&gtk::Image::from_icon_name(crate::ui::icons::shield_icon()));
        about_group.add(&license_row);

        page_about.add(&about_group);
        stack.add_named(&wrap_scroll(&page_about), Some("about"));

        let btn_about = make_nav_button("about", "Shortcuts & About", crate::ui::icons::keyboard_icon(), select_tab.clone());
        nav_list.append(&btn_about);
        nav_buttons.borrow_mut().push(("about".to_string(), btn_about, "Shortcuts & About".to_string()));

        // Navigation search filter hook
        let nav_buttons_filter = nav_buttons.clone();
        filter_entry.connect_search_changed(move |entry| {
            let query = entry.text().trim().to_lowercase();
            for (_, btn, title) in nav_buttons_filter.borrow().iter() {
                if query.is_empty() || title.to_lowercase().contains(&query) {
                    btn.set_visible(true);
                } else {
                    btn.set_visible(false);
                }
            }
        });

        // Activate first tab by default
        select_tab("accounts", "Accounts");

        right_vbox.append(&stack);
        main_hbox.append(&right_vbox);

        dialog.set_child(Some(&main_hbox));

        Self { dialog }
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        self.dialog.present(Some(parent));
    }
}
