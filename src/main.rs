// SPDX-License-Identifier: GPL-3.0-or-later

mod model;
mod services;
mod ui;

use std::path::PathBuf;
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
    provider.load_from_data(include_str!("style.css"));
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
        .default_width(1150)
        .default_height(760)
        .build();

    window.add_css_class("iris-window");

    // Initialize Maildir Scanner & Seed Demo Emails if directory is empty
    let mail_root = get_mail_root();
    let scanner = services::MaildirScanner::new(mail_root.clone());
    let _ = scanner.ensure_demo_data();

    // Main 3-column responsive split view (Sidebar -> Threads -> Reading Pane)
    let outer_split = adw::NavigationSplitView::new();
    outer_split.set_min_sidebar_width(220.0);
    outer_split.set_max_sidebar_width(280.0);

    let sidebar_view = ui::SidebarView::new();
    let sidebar_page = adw::NavigationPage::builder()
        .title("Mailboxes")
        .tag("sidebar")
        .child(&sidebar_view.container)
        .build();
    outer_split.set_sidebar(Some(&sidebar_page));

    // Nested split view for Thread List + Reading View
    let inner_split = adw::NavigationSplitView::new();
    inner_split.set_min_sidebar_width(320.0);
    inner_split.set_max_sidebar_width(440.0);

    let thread_view = ui::ThreadListView::new();

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

    window.set_content(Some(&outer_split));
    window.present();
}
