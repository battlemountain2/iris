// SPDX-License-Identifier: GPL-3.0-or-later

mod model;
mod services;
mod ui;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use adw::prelude::*;
use gtk::glib;

const APPLICATION_ID: &str = "io.github.battlemountain2.Iris";

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

    // Initialize Maildir Scanner & Seed Demo Emails if directory is empty
    let mail_root = get_mail_root();
    let scanner = Rc::new(services::MaildirScanner::new(mail_root.clone()));
    let _ = scanner.ensure_demo_data();

    let demo_account = mail_root.join("demo");
    let current_folder = Rc::new(RefCell::new(demo_account.join("INBOX")));
    let current_title = Rc::new(RefCell::new("Inbox".to_string()));

    // Toast Overlay for 10-Second Undo Actions & Undo Send
    let toast_overlay = adw::ToastOverlay::new();

    // Outer split view (Sidebar -> Content)
    let outer_split = adw::NavigationSplitView::new();
    outer_split.set_min_sidebar_width(180.0);
    outer_split.set_max_sidebar_width(240.0);

    let sidebar_view = Rc::new(ui::SidebarView::new());
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

    // Hook up sidebar toggle button to outer split view
    let outer_split_ref = outer_split.clone();
    thread_view.sidebar_toggle_btn.connect_clicked(move |_| {
        let is_collapsed = outer_split_ref.is_collapsed();
        outer_split_ref.set_collapsed(!is_collapsed);
    });

    let reading_view = Rc::new(ui::ReadingPaneView::new());

    // Connect dynamic selection: clicking or j/k immediately updates the reading view
    let threads_store = thread_view.threads.clone();
    let reading_update = reading_view.clone();
    thread_view.list_box.connect_row_selected(move |_list_box, row_opt| {
        if let Some(row) = row_opt {
            let index = row.index() as usize;
            let borrowed = threads_store.borrow();
            if let Some(thread) = borrowed.get(index) {
                reading_update.display_thread(thread);
            }
        }
    });

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
        let current_folder = current_folder.clone();
        let current_title = current_title.clone();
        let thread_view = thread_view.clone();
        let reading_view = reading_view.clone();
        let sidebar_view = sidebar_view.clone();
        let demo_account = demo_account.clone();

        Rc::new(move || {
            let folder = current_folder.borrow().clone();
            let title = current_title.borrow().clone();
            thread_view.title.set_title(&title);

            let threads = scanner.scan_folder(&folder);
            thread_view.set_threads(&threads);

            if let Some(first) = threads.first() {
                reading_view.display_thread(first);
                if let Some(first_row) = thread_view.list_box.row_at_index(0) {
                    thread_view.list_box.select_row(Some(&first_row));
                }
            } else {
                reading_view.clear();
            }

            // Live Mailbox Counter updates
            let inbox_unread = scanner.count_unread_in_folder(&demo_account.join("INBOX"));
            let archive_count = scanner.count_total_in_folder(&demo_account.join("Archive"));
            let trash_count = scanner.count_total_in_folder(&demo_account.join("Trash"));
            let sent_count = scanner.count_total_in_folder(&demo_account.join("Sent"));

            sidebar_view.update_badge("inbox", inbox_unread);
            sidebar_view.update_badge("account:demo", inbox_unread);
            sidebar_view.update_badge("folder:archive", archive_count);
            sidebar_view.update_badge("folder:trash", trash_count);
            sidebar_view.update_badge("folder:sent", sent_count);
        })
    };

    // Sidebar Mailbox / Folder Switching
    {
        let sidebar_view_ref = sidebar_view.clone();
        let current_folder = current_folder.clone();
        let current_title = current_title.clone();
        let refresh_view = refresh_view.clone();
        let demo_account = demo_account.clone();
        let mail_root = mail_root.clone();

        sidebar_view.list_box.connect_row_selected(move |_list, row_opt| {
            let Some(row) = row_opt else { return; };
            let Some(id) = sidebar_view_ref.get_row_id(row) else { return; };

            match id.as_str() {
                "inbox" | "account:demo" => {
                    *current_folder.borrow_mut() = demo_account.join("INBOX");
                    *current_title.borrow_mut() = "Inbox".to_string();
                }
                "account:work" => {
                    let work_inbox = mail_root.join("work").join("INBOX");
                    let _ = services::maildir::ensure_maildir_folder(&work_inbox);
                    *current_folder.borrow_mut() = work_inbox;
                    *current_title.borrow_mut() = "Gmail (Work)".to_string();
                }
                "account:icloud" => {
                    let icloud_inbox = mail_root.join("icloud").join("INBOX");
                    let _ = services::maildir::ensure_maildir_folder(&icloud_inbox);
                    *current_folder.borrow_mut() = icloud_inbox;
                    *current_title.borrow_mut() = "iCloud".to_string();
                }
                "folder:sent" => {
                    *current_folder.borrow_mut() = demo_account.join("Sent");
                    *current_title.borrow_mut() = "Sent".to_string();
                }
                "folder:archive" => {
                    *current_folder.borrow_mut() = demo_account.join("Archive");
                    *current_title.borrow_mut() = "Archive".to_string();
                }
                "folder:trash" => {
                    *current_folder.borrow_mut() = demo_account.join("Trash");
                    *current_title.borrow_mut() = "Trash".to_string();
                }
                "view:feed" => {
                    *current_folder.borrow_mut() = demo_account.join("INBOX");
                    *current_title.borrow_mut() = "Feed & Newsletters".to_string();
                }
                "view:flagged" => {
                    *current_folder.borrow_mut() = demo_account.join("INBOX");
                    *current_title.borrow_mut() = "Flagged".to_string();
                }
                _ => {}
            }
            refresh_view();
        });
    }

    // Compose View & Undo Send Toast
    let toast_overlay_ref = toast_overlay.clone();
    let compose_dialog = Rc::new(ui::ComposeDialog::new(move |to, _subject, _body| {
        let to_for_undo = to.clone();
        ui::show_undo_toast(&toast_overlay_ref, &to, move || {
            tracing::info!("Undoing send for message to {to_for_undo}");
        });
    }));

    let compose_ref = compose_dialog.clone();
    let win_for_compose = window.clone();
    thread_view.compose_btn.connect_clicked(move |_| {
        compose_ref.present(&win_for_compose);
    });

    // Action Handlers: Archive, Trash, Reply, Star with Undo Toasts
    let do_archive = {
        let reading_view = reading_view.clone();
        let demo_account = demo_account.clone();
        let toast_overlay = toast_overlay.clone();
        let refresh_view = refresh_view.clone();

        Rc::new(move || {
            let thread_opt = reading_view.current_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let subject = thread.subject.clone();
                match services::maildir::archive_thread(&thread, &demo_account) {
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
        let reading_view = reading_view.clone();
        let demo_account = demo_account.clone();
        let toast_overlay = toast_overlay.clone();
        let refresh_view = refresh_view.clone();

        Rc::new(move || {
            let thread_opt = reading_view.current_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let subject = thread.subject.clone();
                match services::maildir::trash_thread(&thread, &demo_account) {
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
        let reading_view = reading_view.clone();
        let compose_dialog = compose_dialog.clone();
        let win = window.clone();

        Rc::new(move || {
            let thread_opt = reading_view.current_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                let to = thread.senders.first().cloned().unwrap_or_default();
                let body = thread.messages.first().and_then(|m| m.body_plain.clone()).unwrap_or_default();
                compose_dialog.present_reply(&win, &to, &thread.subject, &body);
            } else {
                compose_dialog.present(&win);
            }
        })
    };

    let do_star = {
        let reading_view = reading_view.clone();
        let refresh_view = refresh_view.clone();
        Rc::new(move || {
            let thread_opt = reading_view.current_thread.borrow().clone();
            if let Some(thread) = thread_opt {
                if let Ok(new_flag) = services::maildir::toggle_thread_flag(&thread) {
                    tracing::info!("Thread flag toggled: is_flagged={new_flag}");
                    refresh_view();
                }
            }
        })
    };

    let do_toggle_read = {
        let reading_view = reading_view.clone();
        let refresh_view = refresh_view.clone();
        Rc::new(move || {
            let thread_opt = reading_view.current_thread.borrow().clone();
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
        let do_star = do_star.clone();
        reading_view.star_btn.connect_clicked(move |_| {
            do_star();
        });
    }

    // Connect Privacy Banner Action Buttons
    {
        let rv = reading_view.clone();
        reading_view.load_images_btn.connect_clicked(move |_| {
            rv.allow_remote_images.set(true);
            if let Some(thread) = rv.current_thread.borrow().as_ref() {
                rv.display_thread(thread);
            }
        });
    }
    {
        let rv = reading_view.clone();
        reading_view.always_load_btn.connect_clicked(move |_| {
            rv.allow_remote_images.set(true);
            if let Some(thread) = rv.current_thread.borrow().as_ref() {
                rv.display_thread(thread);
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
        let do_star = do_star.clone();

        Rc::new(ui::CommandPalette::with_action_handler(
            theme_manager.clone(),
            move |action| match action {
                "compose" => compose_dialog.present(&win),
                "nav_inbox" => sidebar_view.select_folder("inbox"),
                "nav_sent" => sidebar_view.select_folder("folder:sent"),
                "nav_archive" => sidebar_view.select_folder("folder:archive"),
                "nav_feed" => sidebar_view.select_folder("view:feed"),
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

    // Global Vim & Modal Keybindings (Ctrl+K, j, k, c, e, #, r, s, u, /)
    let key_actions = ui::KeybindingActions {
        on_archive: do_archive,
        on_trash: do_trash,
        on_reply: do_reply,
        on_star: do_star,
        on_toggle_read: do_toggle_read,
        on_focus_search: do_focus_search,
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

    window.present();
}
