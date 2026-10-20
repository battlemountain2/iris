// SPDX-License-Identifier: GPL-3.0-or-later

mod model;
mod services;
mod ui;

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
        .title("Iris")
        .default_width(1060)
        .default_height(720)
        .build();

    window.add_css_class("iris-window");

    // Initialize Theme Manager (Noctalia, Greek themes, Subtle mode)
    let theme_manager = ui::ThemeManager::new();

    // Initialize Maildir Scanner & Seed Demo Emails if directory is empty
    let mail_root = get_mail_root();
    let scanner = services::MaildirScanner::new(mail_root.clone());
    let _ = scanner.ensure_demo_data();

    // Toast Overlay for 10-Second Undo Send
    let toast_overlay = adw::ToastOverlay::new();

    // Outer split view (Sidebar -> Content)
    let outer_split = adw::NavigationSplitView::new();
    outer_split.set_min_sidebar_width(200.0);
    outer_split.set_max_sidebar_width(250.0);

    let sidebar_view = ui::SidebarView::new();
    let sidebar_page = adw::NavigationPage::builder()
        .title("Mailboxes")
        .tag("sidebar")
        .child(&sidebar_view.container)
        .build();
    outer_split.set_sidebar(Some(&sidebar_page));

    // Inner split view (Thread List -> Reading Pane)
    let inner_split = adw::NavigationSplitView::new();
    inner_split.set_min_sidebar_width(280.0);
    inner_split.set_max_sidebar_width(380.0);

    let thread_view = ui::ThreadListView::new();

    // Hook up sidebar toggle button to outer split view
    let outer_split_ref = outer_split.clone();
    thread_view.sidebar_toggle_btn.connect_clicked(move |_| {
        let is_collapsed = outer_split_ref.is_collapsed();
        outer_split_ref.set_collapsed(!is_collapsed);
    });

    // Scan threads from demo/INBOX
    let demo_inbox = mail_root.join("demo").join("INBOX");
    let threads = scanner.scan_folder(&demo_inbox);
    thread_view.set_threads(&threads);

    let thread_page = adw::NavigationPage::builder()
        .title("Inbox")
        .tag("threads")
        .child(&thread_view.container)
        .build();
    inner_split.set_sidebar(Some(&thread_page));

    let reading_view = ui::ReadingPaneView::new();
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

    // Command Palette (Ctrl+K)
    let palette = Rc::new(ui::CommandPalette::new(theme_manager.clone()));
    let palette_ref = palette.clone();
    let win_for_palette = window.clone();
    thread_view.search_button.connect_clicked(move |_| {
        palette_ref.present(&win_for_palette);
    });

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

    // Global Vim & Modal Keybindings (Ctrl+K, j, k, c, /)
    ui::setup_window_keybindings(
        &window,
        palette,
        compose_dialog,
        thread_view.list_box.clone(),
    );

    window.present();
}
