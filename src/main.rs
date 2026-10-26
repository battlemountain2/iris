// SPDX-License-Identifier: GPL-3.0-or-later

mod model;
mod services;
mod ui;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use adw::prelude::*;
use gtk::glib;

use crate::model::Thread;

const APPLICATION_ID: &str = "io.github.battlemountain2.Iris";

fn send_desktop_notification(app: &adw::Application, id: Option<&str>, title: &str, body: &str) {
    let notification = gio::Notification::new(title);
    notification.set_body(Some(body));
    let icon = gio::ThemedIcon::new("mail-unread-symbolic");
    notification.set_icon(&icon);
    app.send_notification(id, &notification);
}

fn main() -> glib::ExitCode {
    if let Err(error) = tracing_subscriber::fmt::try_init() {
        eprintln!("Unable to initialize logging: {error}");
    }

    let application = adw::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    application.connect_startup(|_| {
        load_css();
    });

    application.connect_activate(build_ui);
    application.run()
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn get_mail_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("IRIS_MAIL_DIR") {
        PathBuf::from(custom)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join("Mail")
    } else {
        PathBuf::from("/home/bry/Mail")
    }
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Iris (Ctrl+K for Command Palette)")
        .default_width(1280)
        .default_height(750)
        .build();

    println!("✉️  Iris Mail v0.1.0 initialized — Ctrl+K = Command Palette, c = Compose, j/k = Navigate, e = Archive, # = Trash");

    window.add_css_class("iris-window");

    // Initialize Theme Manager (Noctalia, Greek themes, Subtle mode)
    let theme_manager = ui::ThemeManager::new();

    // Load or seed user accounts from ~/.config/iris/config.toml
    let iris_config = Rc::new(services::config::load_or_create_config());

    // Initialize Maildir Scanner & Seed Demo Emails if directory is empty
    let mail_root = get_mail_root();
    let scanner = Rc::new(services::MaildirScanner::new(mail_root.clone()));
    let _ = scanner.ensure_demo_data();

    // Authoritative Application State
    struct AppState {
        mail_root: PathBuf,
        current_folder: RefCell<PathBuf>,
        current_title: RefCell<String>,
        selected_thread: RefCell<Option<Thread>>,
    }

    let initial_acct_dir = iris_config
        .accounts
        .first()
        .and_then(|a| a.maildir_path.as_ref().map(PathBuf::from))
        .unwrap_or_else(|| mail_root.join("demo"));
    let initial_inbox = initial_acct_dir.join("INBOX");
    let initial_title = iris_config
        .accounts
        .first()
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| "Inbox".to_string());

    let demo_account = mail_root.join("demo");
    let app_state = Rc::new(AppState {
        mail_root: mail_root.clone(),
        current_folder: RefCell::new(initial_inbox),
        current_title: RefCell::new(initial_title),
        selected_thread: RefCell::new(None),
    });

    // Toast Overlay for 10-Second Undo Actions & Undo Send
    let toast_overlay = adw::ToastOverlay::new();

    // Outer split view (Sidebar -> Content)
    let outer_split = adw::NavigationSplitView::new();
    outer_split.set_min_sidebar_width(180.0);
    outer_split.set_max_sidebar_width(240.0);

    let sidebar_view = Rc::new(ui::SidebarView::with_accounts(&iris_config.accounts));
    let sidebar_page = adw::NavigationPage::builder()
        .title("Mailboxes")
        .tag("sidebar")
        .child(&sidebar_view.container)
        .build();
    outer_split.set_sidebar(Some(&sidebar_page));

    // Inner split view (Thread List -> Reading Pane)
    let inner_split = adw::NavigationSplitView::new();
    inner_split.set_min_sidebar_width(260.0);
    inner_split.set_max_sidebar_width(340.0);

    let thread_view = Rc::new(ui::ThreadListView::new());
    thread_view.setup_tab_handlers();

    // Hook up sidebar toggle button to outer split view
    let outer_split_ref = outer_split.clone();
    thread_view.sidebar_toggle_btn.connect_clicked(move |_| {
        let is_collapsed = outer_split_ref.is_collapsed();
        outer_split_ref.set_collapsed(!is_collapsed);
    });

    let reading_view = Rc::new(ui::ReadingPaneView::new());
    reading_view.set_toast_overlay(&toast_overlay);

    // Adaptive Back Button on Narrow / Collapsed Layouts
    {
        let inner_split_back = inner_split.clone();
        reading_view.back_btn.connect_clicked(move |_| {
            inner_split_back.set_show_content(false);
        });

        let back_btn_clone = reading_view.back_btn.clone();
        inner_split.connect_collapsed_notify(move |split| {
            back_btn_clone.set_visible(split.is_collapsed());
        });
        reading_view.back_btn.set_visible(inner_split.is_collapsed());
    }

    // Dynamic selection: clicking or j/k immediately updates authoritative state and reading view
    {
        let tv = thread_view.clone();
        let rv = reading_view.clone();
        let state = app_state.clone();
        let inner_split_sel = inner_split.clone();
        thread_view.list_box.connect_row_selected(move |_list_box, row_opt| {
            if let Some(row) = row_opt {
                let index = row.index() as usize;
                if let Some(thread) = tv.get_thread_at_index(index) {
                    *state.selected_thread.borrow_mut() = Some(thread.clone());
                    rv.display_thread(&thread);
                    if inner_split_sel.is_collapsed() {
                        inner_split_sel.set_show_content(true);
                    }
                }
            } else {
                *state.selected_thread.borrow_mut() = None;
            }
        });
    }

    let thread_page = adw::NavigationPage::builder()
        .title("Inbox")
        .tag("threads")
        .child(&thread_view.container)
        .build();
    inner_split.set_sidebar(Some(&thread_page));

    let reading_page = adw::NavigationPage::builder()
        .title("Message")
        .tag("message")
        .child(&reading_view.container)
        .build();
    inner_split.set_content(Some(&reading_page));

    let content_page = adw::NavigationPage::builder()
        .title("Mail View")
        .tag("content")
        .child(&inner_split)
        .build();
    outer_split.set_content(Some(&content_page));

    // Adaptive Breakpoints: Automatic folding on narrow / floating Hyprland splits
    let bp_tablet = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        960.0,
        adw::LengthUnit::Sp,
    ));
    bp_tablet.add_setter(&outer_split, "collapsed", Some(&true.into()));
    window.add_breakpoint(bp_tablet);

    let bp_compact = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        620.0,
        adw::LengthUnit::Sp,
    ));
    bp_compact.add_setter(&inner_split, "collapsed", Some(&true.into()));
    window.add_breakpoint(bp_compact);

    toast_overlay.set_child(Some(&outer_split));
    window.set_content(Some(&toast_overlay));

    // Reactive Refresh Helper: Rescans active folder, updates thread list & live counters
    let refresh_view: Rc<dyn Fn()> = {
        let scanner = scanner.clone();
        let state = app_state.clone();
        let thread_view = thread_view.clone();
        let reading_view = reading_view.clone();
        let sidebar_view = sidebar_view.clone();
        let iris_config = iris_config.clone();

        Rc::new(move || {
            let folder = state.current_folder.borrow().clone();
            let title = state.current_title.borrow().clone();
            thread_view.title.set_title(&title);

            let prev_index = thread_view.list_box.selected_row().map(|r| r.index() as usize).unwrap_or(0);
            let prev_id = state.selected_thread.borrow().as_ref().map(|t| t.id.clone());

            let threads = scanner.scan_folder(&folder);
            thread_view.set_threads(&threads);

            let visible = thread_view.get_visible_threads();
            if visible.is_empty() {
                reading_view.clear();
                *state.selected_thread.borrow_mut() = None;
            } else {
                // If previously selected thread is still present in folder, keep it selected.
                // If it was removed (e.g. archived or deleted), select neighbor at prev_index.
                let target_index = if let Some(ref pid) = prev_id {
                    visible.iter().position(|t| &t.id == pid).unwrap_or_else(|| {
                        prev_index.min(visible.len() - 1)
                    })
                } else {
                    prev_index.min(visible.len() - 1)
                };

                thread_view.select_index(target_index);
                if let Some(target_thread) = visible.get(target_index) {
                    *state.selected_thread.borrow_mut() = Some(target_thread.clone());
                    reading_view.display_thread(target_thread);
                }
            }

            // Live Mailbox Counter updates across configured accounts
            let mut total_unread = 0;
            for (idx, acct) in iris_config.accounts.iter().enumerate() {
                let acct_dir = acct
                    .maildir_path
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| state.mail_root.join(&acct.id));
                let unread = scanner.count_unread_in_folder(&acct_dir.join("INBOX"));
                sidebar_view.update_badge(&format!("account:{}", acct.id), unread);
                total_unread += unread;

                if idx == 0 {
                    let archive_count = scanner.count_total_in_folder(&acct_dir.join("Archive"));
                    let trash_count = scanner.count_total_in_folder(&acct_dir.join("Trash"));
                    let sent_count = scanner.count_total_in_folder(&acct_dir.join("Sent"));
                    let drafts_count = scanner.count_total_in_folder(&acct_dir.join("Drafts"));
                    sidebar_view.update_badge("folder:archive", archive_count);
                    sidebar_view.update_badge("folder:trash", trash_count);
                    sidebar_view.update_badge("folder:sent", sent_count);
                    sidebar_view.update_badge("folder:drafts", drafts_count);
                }
            }
            sidebar_view.update_badge("inbox", total_unread);
        })
    };

    // Sidebar Mailbox / Folder Switching
    {
        let sidebar_view_ref = sidebar_view.clone();
        let state = app_state.clone();
        let refresh_view = refresh_view.clone();
        let demo_account = demo_account.clone();
        let mail_root = mail_root.clone();
        let outer_split = outer_split.clone();
        let cfg = iris_config.clone();

        sidebar_view.list_box.connect_row_selected(move |_list, row_opt| {
            let Some(row) = row_opt else { return; };
            let Some(id) = sidebar_view_ref.get_row_id(row) else { return; };

            if id == "inbox" {
                let initial_dir = cfg
                    .accounts
                    .first()
                    .and_then(|a| a.maildir_path.as_ref().map(PathBuf::from))
                    .unwrap_or_else(|| mail_root.join("demo"));
                *state.current_folder.borrow_mut() = initial_dir.join("INBOX");
                *state.current_title.borrow_mut() = "All Inboxes".to_string();
            } else if let Some(acct_id) = id.strip_prefix("account:") {
                if let Some(acct) = cfg.accounts.iter().find(|a| a.id == acct_id) {
                    let acct_dir = acct
                        .maildir_path
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| mail_root.join(&acct.id));
                    let inbox = acct_dir.join("INBOX");
                    let _ = services::maildir::ensure_maildir_folder(&inbox);
                    *state.current_folder.borrow_mut() = inbox;
                    *state.current_title.borrow_mut() = acct.display_name.clone();
                }
            } else {
                match id.as_str() {
                    "folder:drafts" => {
                        let drafts_dir = demo_account.join("Drafts");
                        let _ = services::maildir::ensure_maildir_folder(&drafts_dir);
                        *state.current_folder.borrow_mut() = drafts_dir;
                        *state.current_title.borrow_mut() = "Drafts".to_string();
                    }
                    "folder:sent" => {
                        *state.current_folder.borrow_mut() = demo_account.join("Sent");
                        *state.current_title.borrow_mut() = "Sent".to_string();
                    }
                    "folder:archive" => {
                        *state.current_folder.borrow_mut() = demo_account.join("Archive");
                        *state.current_title.borrow_mut() = "Archive".to_string();
                    }
                    "folder:trash" => {
                        *state.current_folder.borrow_mut() = demo_account.join("Trash");
                        *state.current_title.borrow_mut() = "Trash".to_string();
                    }
                    "view:feed" => {
                        *state.current_folder.borrow_mut() = demo_account.join("INBOX");
                        *state.current_title.borrow_mut() = "Feed & Newsletters".to_string();
                    }
                    "view:flagged" => {
                        *state.current_folder.borrow_mut() = demo_account.join("INBOX");
                        *state.current_title.borrow_mut() = "Flagged".to_string();
                    }
                    _ => {}
                }
            }
            refresh_view();
            if outer_split.is_collapsed() {
                outer_split.set_show_content(true);
            }
        });
    }

    // Compose View & Durable Delayed Outbox Queue (10s Undo Send)
    let compose_holder: Rc<RefCell<Option<Rc<ui::ComposeDialog>>>> = Rc::new(RefCell::new(None));
    let holder_for_send = compose_holder.clone();
    let toast_overlay_send = toast_overlay.clone();
    let win_send = window.clone();
    let mail_root_send = mail_root.clone();
    let refresh_send = refresh_view.clone();
    let cfg_dispatch = iris_config.clone();
    let app_for_send = app.clone();

    let compose_dialog = Rc::new(ui::ComposeDialog::with_accounts(&iris_config.accounts, move |payload| {
        if payload.to.is_empty() {
            let toast = adw::Toast::builder()
                .title("Cannot send: Recipient address is empty")
                .timeout(3)
                .build();
            toast_overlay_send.add_toast(toast);
            if let Some(ref dlg) = *holder_for_send.borrow() {
                dlg.reopen_draft(
                    &win_send,
                    &payload.to,
                    &payload.subject,
                    &payload.body,
                    payload.in_reply_to,
                    payload.references,
                );
            }
            return;
        }

        // Clean up draft file from disk if message was being edited from Drafts
        if let Some(ref draft_path) = payload.draft_file_path {
            let _ = services::draft::delete_draft(draft_path);
        }

        // 1. Create OutboxMessage
        let outbox_msg = services::outbox::OutboxMessage::new(
            &payload.account_id,
            &payload.from,
            &payload.to,
            &payload.subject,
            &payload.body,
            payload.in_reply_to.clone(),
            payload.references.clone(),
        );

        // 2. Durably enqueue to disk before dispatch
        if let Err(e) = services::outbox::enqueue_outbox_message(&outbox_msg) {
            tracing::error!("Failed to enqueue message to outbox disk: {e}");
        }

        // 3. Prepare 10-second Undo Send timer
        let cancelled = Rc::new(Cell::new(false));
        let cancelled_undo = cancelled.clone();
        let msg_id = outbox_msg.id.clone();
        let payload_undo = payload.clone();
        let holder_undo = holder_for_send.clone();
        let win_undo = win_send.clone();
        let toast_overlay_undo = toast_overlay_send.clone();

        let toast = adw::Toast::builder()
            .title(format!("Sending to {}... (10s Undo)", payload.to))
            .button_label("Undo")
            .timeout(10)
            .priority(adw::ToastPriority::High)
            .build();

        toast.connect_button_clicked(move |_| {
            cancelled_undo.set(true);
            services::outbox::remove_from_outbox(&msg_id);
            if let Some(ref dlg) = *holder_undo.borrow() {
                dlg.reopen_draft(
                    &win_undo,
                    &payload_undo.to,
                    &payload_undo.subject,
                    &payload_undo.body,
                    payload_undo.in_reply_to.clone(),
                    payload_undo.references.clone(),
                );
            }
            let cancel_toast = adw::Toast::builder()
                .title("Send cancelled. Draft restored.")
                .timeout(3)
                .build();
            toast_overlay_undo.add_toast(cancel_toast);
        });

        toast_overlay_send.add_toast(toast);

        // 4. Background dispatch after 10 seconds if not cancelled
        let mail_root_dispatch = mail_root_send.clone();
        let refresh_dispatch = refresh_send.clone();
        let toast_overlay_dispatch = toast_overlay_send.clone();
        let holder_dispatch = holder_for_send.clone();
        let win_dispatch = win_send.clone();
        let payload_err = payload.clone();
        let cfg_dispatch = cfg_dispatch.clone();
        let app_dispatch = app_for_send.clone();

        gtk::glib::timeout_add_local_once(std::time::Duration::from_secs(10), move || {
            if cancelled.get() {
                tracing::info!("Outbox dispatch aborted: user requested Undo.");
                return;
            }

            tracing::info!(
                "10s Undo window elapsed for {}. Dispatching transport...",
                outbox_msg.id
            );

            let matching_smtp = cfg_dispatch
                .accounts
                .iter()
                .find(|a| a.id == outbox_msg.account_name)
                .and_then(|acct| {
                    acct.smtp_host.as_ref().map(|host| services::smtp::SmtpAccountConfig {
                        id: acct.id.clone(),
                        name: acct.display_name.clone(),
                        email: acct.email.clone(),
                        host: host.clone(),
                        port: acct.smtp_port.unwrap_or(587),
                        username: acct.smtp_username.clone().unwrap_or_else(|| acct.email.clone()),
                        password: acct.smtp_password.clone(),
                    })
                });

            services::smtp::dispatch_in_background(
                mail_root_dispatch,
                outbox_msg,
                matching_smtp,
                move |res| match res {
                    Ok(success_msg) => {
                        refresh_dispatch();
                        send_desktop_notification(&app_dispatch, None, "Email Sent", &success_msg);
                        let ok_toast = adw::Toast::builder().title(&success_msg).timeout(4).build();
                        toast_overlay_dispatch.add_toast(ok_toast);
                    }
                    Err(err) => {
                        let fail_toast = adw::Toast::builder()
                            .title(format!("Delivery failed: {err}. Preserved in drafts."))
                            .button_label("Reopen")
                            .timeout(12)
                            .priority(adw::ToastPriority::High)
                            .build();

                        let holder_reopen = holder_dispatch.clone();
                        let win_reopen = win_dispatch.clone();
                        let p_reopen = payload_err.clone();
                        fail_toast.connect_button_clicked(move |_| {
                            if let Some(ref dlg) = *holder_reopen.borrow() {
                                dlg.reopen_draft(
                                    &win_reopen,
                                    &p_reopen.to,
                                    &p_reopen.subject,
                                    &p_reopen.body,
                                    p_reopen.in_reply_to.clone(),
                                    p_reopen.references.clone(),
                                );
                            }
                        });
                        toast_overlay_dispatch.add_toast(fail_toast);
                    }
                },
            );
        });
    }));

    compose_dialog.set_drafts_dir(demo_account.join("Drafts"));
    *compose_holder.borrow_mut() = Some(compose_dialog.clone());

    let compose_ref = compose_dialog.clone();
    let win_for_compose = window.clone();
    thread_view.compose_btn.connect_clicked(move |_| {
        compose_ref.present(&win_for_compose);
    });

    let edit_draft_ref = compose_dialog.clone();
    let win_for_draft = window.clone();
    let reading_view_draft = reading_view.clone();
    reading_view.edit_draft_btn.connect_clicked(move |_| {
        let thread_opt = reading_view_draft.current_thread.borrow().clone();
        if let Some(thread) = thread_opt {
            if let Some(msg) = thread.messages.first() {
                let to_str = msg.to.join(", ");
                let subject = msg.subject.clone();
                let body = msg.body_plain.clone().unwrap_or_default();
                let in_reply_to = msg.in_reply_to.clone();
                let references = msg.references.clone();
                let draft_path = msg.file_path.as_ref().map(PathBuf::from);

                edit_draft_ref.present_edit_draft(
                    &win_for_draft,
                    &to_str,
                    &subject,
                    &body,
                    in_reply_to,
                    references,
                    None,
                    draft_path,
                );
            }
        }
    });

    // Sidebar Sync Button (Background worker with animated spinner)
    {
        let sync_btn = sidebar_view.sync_btn.clone();
        let mail_root = mail_root.clone();
        let toast_overlay = toast_overlay.clone();
        let refresh_view = refresh_view.clone();
        let app_sync = app.clone();
        let is_syncing = Rc::new(Cell::new(false));

        sync_btn.clone().connect_clicked(move |_| {
            if is_syncing.get() {
                return;
            }
            is_syncing.set(true);

            let spinner = gtk::Spinner::builder().spinning(true).build();
            sync_btn.set_child(Some(&spinner));
            sync_btn.set_sensitive(false);

            let btn_done = sync_btn.clone();
            let sync_flag = is_syncing.clone();
            let toast_ov = toast_overlay.clone();
            let refresh = refresh_view.clone();
            let app_notif = app_sync.clone();

            services::sync::run_sync_in_background(mail_root.clone(), move |res| {
                sync_flag.set(false);
                btn_done.set_child(None::<&gtk::Widget>);
                btn_done.set_icon_name("view-refresh-symbolic");
                btn_done.set_sensitive(true);

                match res {
                    services::sync::SyncResult::SyncedRemote(msg) => {
                        refresh();
                        send_desktop_notification(&app_notif, None, "Mailboxes Synced", &msg);
                        let toast = adw::Toast::builder()
                            .title(msg)
                            .timeout(3)
                            .build();
                        toast_ov.add_toast(toast);
                    }
                    services::sync::SyncResult::RefreshedLocal(count) => {
                        refresh();
                        let toast = adw::Toast::builder()
                            .title(format!("Mailboxes refreshed ({count} messages)"))
                            .timeout(2)
                            .build();
                        toast_ov.add_toast(toast);
                    }
                    services::sync::SyncResult::Error(err) => {
                        let toast = adw::Toast::builder()
                            .title(format!("Sync error: {err}"))
                            .timeout(4)
                            .build();
                        toast_ov.add_toast(toast);
                    }
                }
            });
        });
    }

    // Action Handlers: Archive, Trash, Reply, Star with Undo Toasts
    let do_archive = {
        let state = app_state.clone();
        let toast_overlay = toast_overlay.clone();
        let refresh_view = refresh_view.clone();

        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let subject = thread.subject.clone();
                let folder = state.current_folder.borrow().clone();
                let account_fallback = folder.parent().unwrap_or(&folder).to_path_buf();
                match services::maildir::archive_thread(&thread, &account_fallback) {
                    Ok(moves) => {
                        tracing::info!("Archived thread: '{}' ({} files moved)", subject, moves.len());
                        refresh_view();
                        let refresh_undo = refresh_view.clone();
                        ui::show_undo_action_toast(
                            &toast_overlay,
                            &format!("Archived: {subject}"),
                            move || {
                                if let Err(e) = services::maildir::restore_moved_files(&moves) {
                                    tracing::error!("Failed to undo archive: {e}");
                                } else {
                                    tracing::info!("Restored archived thread: '{subject}'");
                                    refresh_undo();
                                }
                            },
                        );
                    }
                    Err(err) => {
                        tracing::error!("Archive failed: {err}");
                    }
                }
            }
        })
    };

    let do_trash = {
        let state = app_state.clone();
        let toast_overlay = toast_overlay.clone();
        let refresh_view = refresh_view.clone();

        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let subject = thread.subject.clone();
                let folder = state.current_folder.borrow().clone();
                let account_fallback = folder.parent().unwrap_or(&folder).to_path_buf();
                match services::maildir::trash_thread(&thread, &account_fallback) {
                    Ok(moves) => {
                        tracing::info!("Moved thread to trash: '{}' ({} files moved)", subject, moves.len());
                        refresh_view();
                        let refresh_undo = refresh_view.clone();
                        ui::show_undo_action_toast(
                            &toast_overlay,
                            &format!("Deleted: {subject}"),
                            move || {
                                if let Err(e) = services::maildir::restore_moved_files(&moves) {
                                    tracing::error!("Failed to undo trash: {e}");
                                } else {
                                    tracing::info!("Restored trashed thread: '{subject}'");
                                    refresh_undo();
                                }
                            },
                        );
                    }
                    Err(err) => {
                        tracing::error!("Trash failed: {err}");
                    }
                }
            }
        })
    };

    let do_reply = {
        let state = app_state.clone();
        let compose_dialog = compose_dialog.clone();
        let win = window.clone();

        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let latest_msg = thread.messages.last();
                let to = latest_msg
                    .and_then(|m| m.reply_to.clone())
                    .or_else(|| latest_msg.map(|m| m.from.clone()))
                    .or_else(|| thread.senders.first().cloned())
                    .unwrap_or_default();
                let body = latest_msg.and_then(|m| m.body_plain.clone()).unwrap_or_default();

                let folder_str = state.current_folder.borrow().to_string_lossy().to_string();
                let account_id = if folder_str.contains("/work/") {
                    "work"
                } else if folder_str.contains("/icloud/") {
                    "icloud"
                } else {
                    "demo"
                };

                let in_reply_to = latest_msg.map(|m| m.id.clone());
                let mut references = latest_msg.map(|m| m.references.clone()).unwrap_or_default();
                if let Some(ref mid) = in_reply_to {
                    if !references.contains(mid) {
                        references.push(mid.clone());
                    }
                }

                compose_dialog.present_reply(
                    &win,
                    &to,
                    &thread.subject,
                    &body,
                    in_reply_to,
                    references,
                    Some(account_id),
                );
            } else {
                compose_dialog.present(&win);
            }
        })
    };

    let do_reply_all = {
        let state = app_state.clone();
        let compose_dialog = compose_dialog.clone();
        let win = window.clone();

        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let latest_msg = thread.messages.last();
                let sender = latest_msg
                    .and_then(|m| m.reply_to.clone())
                    .or_else(|| latest_msg.map(|m| m.from.clone()))
                    .or_else(|| thread.senders.first().cloned())
                    .unwrap_or_default();
                let body = latest_msg.and_then(|m| m.body_plain.clone()).unwrap_or_default();

                let folder_str = state.current_folder.borrow().to_string_lossy().to_string();
                let account_id = if folder_str.contains("/work/") {
                    "work"
                } else if folder_str.contains("/icloud/") {
                    "icloud"
                } else {
                    "demo"
                };

                let user_emails = ["bry@gmail.com", "brayan@work.com", "brayan@icloud.com"];
                let empty_vec = Vec::new();
                let to_addrs = latest_msg.map(|m| &m.to).unwrap_or(&empty_vec);
                let cc_addrs = latest_msg.map(|m| &m.cc).unwrap_or(&empty_vec);
                let reply_to_ref = latest_msg.and_then(|m| m.reply_to.as_deref());

                let recipients = ui::compose::filter_reply_all_recipients(
                    reply_to_ref,
                    &sender,
                    to_addrs,
                    cc_addrs,
                    &user_emails,
                );

                let to_field = recipients.join(", ");

                let in_reply_to = latest_msg.map(|m| m.id.clone());
                let mut references = latest_msg.map(|m| m.references.clone()).unwrap_or_default();
                if let Some(ref mid) = in_reply_to {
                    if !references.contains(mid) {
                        references.push(mid.clone());
                    }
                }

                compose_dialog.present_reply(
                    &win,
                    &to_field,
                    &thread.subject,
                    &body,
                    in_reply_to,
                    references,
                    Some(account_id),
                );
            } else {
                compose_dialog.present(&win);
            }
        })
    };

    let do_forward = {
        let state = app_state.clone();
        let compose_dialog = compose_dialog.clone();
        let win = window.clone();

        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let latest_msg = thread.messages.last();
                let from = latest_msg
                    .map(|m| m.from.clone())
                    .or_else(|| thread.senders.first().cloned())
                    .unwrap_or_default();

                let date_str = latest_msg
                    .map(|m| m.date.format("%a, %b %-d, %Y at %-I:%M %p UTC").to_string())
                    .unwrap_or_else(|| "Unknown date".to_string());

                let to_str = latest_msg
                    .map(|m| m.to.join(", "))
                    .unwrap_or_default();

                let body = latest_msg.and_then(|m| m.body_plain.clone()).unwrap_or_default();

                let folder_str = state.current_folder.borrow().to_string_lossy().to_string();
                let account_id = if folder_str.contains("/work/") {
                    "work"
                } else if folder_str.contains("/icloud/") {
                    "icloud"
                } else {
                    "demo"
                };

                compose_dialog.present_forward(
                    &win,
                    &thread.subject,
                    &from,
                    &date_str,
                    &to_str,
                    &body,
                    Some(account_id),
                );
            } else {
                compose_dialog.present(&win);
            }
        })
    };

    let do_star = {
        let state = app_state.clone();
        let refresh_view = refresh_view.clone();
        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                if let Ok(new_flag) = services::maildir::toggle_thread_flag(&thread) {
                    tracing::info!("Thread flag toggled: is_flagged={new_flag}");
                    refresh_view();
                }
            }
        })
    };

    let do_toggle_read = {
        let state = app_state.clone();
        let refresh_view = refresh_view.clone();
        Rc::new(move || {
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                if let Ok(new_unread) = services::maildir::toggle_thread_read(&thread) {
                    tracing::info!("Thread unread toggled: is_unread={new_unread}");
                    refresh_view();
                }
            }
        })
    };

    let do_focus_search = {
        let thread_view = thread_view.clone();
        Rc::new(move || {
            thread_view.search_entry.grab_focus();
        })
    };

    // Connect Action Buttons in Reading Pane Header
    {
        let do_archive = do_archive.clone();
        reading_view.archive_btn.connect_clicked(move |_| {
            do_archive();
        });
    }
    {
        let do_trash = do_trash.clone();
        reading_view.trash_btn.connect_clicked(move |_| {
            do_trash();
        });
    }
    {
        let do_reply = do_reply.clone();
        reading_view.reply_btn.connect_clicked(move |_| {
            do_reply();
        });
    }
    {
        let do_reply_all = do_reply_all.clone();
        reading_view.reply_all_btn.connect_clicked(move |_| {
            do_reply_all();
        });
    }
    {
        let do_forward = do_forward.clone();
        reading_view.forward_btn.connect_clicked(move |_| {
            do_forward();
        });
    }
    {
        let do_star = do_star.clone();
        reading_view.star_btn.connect_clicked(move |_| {
            do_star();
        });
    }

    // Connect Privacy Banner Action Buttons (with clean borrow guards)
    {
        let rv = reading_view.clone();
        let state = app_state.clone();
        reading_view.load_images_btn.connect_clicked(move |_| {
            rv.allow_remote_images.set(true);
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                rv.display_thread(&thread);
            }
        });
    }
    {
        let rv = reading_view.clone();
        let state = app_state.clone();
        reading_view.always_load_btn.connect_clicked(move |_| {
            rv.allow_remote_images.set(true);
            let thread_opt = state.selected_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                rv.display_thread(&thread);
            }
        });
    }

    // Connect Live Filter in Thread List
    {
        let tv_filter = thread_view.clone();
        thread_view.search_entry.connect_search_changed(move |entry| {
            tv_filter.apply_filter(&entry.text());
        });
    }

    // Command Palette (Ctrl+K) with full action routing
    let palette = {
        let sidebar_view = sidebar_view.clone();
        let compose_dialog = compose_dialog.clone();
        let win = window.clone();
        let do_archive = do_archive.clone();
        let do_trash = do_trash.clone();
        let do_reply = do_reply.clone();
        let do_reply_all = do_reply_all.clone();
        let do_forward = do_forward.clone();
        let do_star = do_star.clone();

        Rc::new(ui::CommandPalette::with_action_handler(
            theme_manager.clone(),
            move |action| match action {
                "compose" => compose_dialog.present(&win),
                "nav_inbox" => sidebar_view.select_folder("inbox"),
                "nav_sent" => sidebar_view.select_folder("folder:sent"),
                "nav_archive" => sidebar_view.select_folder("folder:archive"),
                "nav_feed" => sidebar_view.select_folder("view:feed"),
                "reply" => do_reply(),
                "reply_all" => do_reply_all(),
                "forward" => do_forward(),
                "archive" => do_archive(),
                "trash" => do_trash(),
                "star" => do_star(),
                _ => {}
            },
        ))
    };

    let palette_ref = palette.clone();
    let win_for_palette = window.clone();
    thread_view.palette_btn.connect_clicked(move |_| {
        palette_ref.present(&win_for_palette);
    });

    let do_back = {
        let inner_split = inner_split.clone();
        let outer_split = outer_split.clone();
        Rc::new(move || {
            if inner_split.is_collapsed() && inner_split.shows_content() {
                inner_split.set_show_content(false);
            } else if outer_split.is_collapsed() && outer_split.shows_content() {
                outer_split.set_show_content(false);
            }
        })
    };

    // Global Vim & Modal Keybindings (Ctrl+K, j, k, c, e, #, r, a, f, s, u, /, Esc)
    let key_actions = ui::KeybindingActions {
        on_archive: do_archive,
        on_trash: do_trash,
        on_reply: do_reply,
        on_reply_all: do_reply_all,
        on_forward: do_forward,
        on_star: do_star,
        on_toggle_read: do_toggle_read,
        on_focus_search: do_focus_search,
        on_back: do_back,
    };

    ui::setup_window_keybindings(
        &window,
        palette,
        compose_dialog,
        thread_view.list_box.clone(),
        key_actions,
    );

    // Initial folder load and counter synchronization
    refresh_view();
    sidebar_view.select_folder("inbox");

    // Startup: Recover any interrupted outbox messages from previous sessions into drafts
    let recovered_drafts = services::outbox::recover_stale_outbox_to_drafts();
    if !recovered_drafts.is_empty() {
        let count = recovered_drafts.len();
        let toast = adw::Toast::builder()
            .title(format!("Recovered {count} unsent draft(s)"))
            .timeout(5)
            .build();
        toast_overlay.add_toast(toast);
    }

    window.present();
}
