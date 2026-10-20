// SPDX-License-Identifier: GPL-3.0-or-later

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

    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Iris")
        .default_width(1100)
        .default_height(750)
        .build();

    window.add_css_class("iris-window");

    // Main 3-column responsive split view (Sidebar -> Threads -> Reading Pane)
    let outer_split = adw::NavigationSplitView::new();
    outer_split.set_min_sidebar_width(220.0);
    outer_split.set_max_sidebar_width(300.0);

    let sidebar_page = adw::NavigationPage::builder()
        .title("Mailboxes")
        .tag("sidebar")
        .child(&build_sidebar())
        .build();
    outer_split.set_sidebar(Some(&sidebar_page));

    // Nested split view for Thread List + Reading View
    let inner_split = adw::NavigationSplitView::new();
    inner_split.set_min_sidebar_width(320.0);
    inner_split.set_max_sidebar_width(450.0);

    let thread_page = adw::NavigationPage::builder()
        .title("Inbox")
        .tag("threads")
        .child(&build_thread_list())
        .build();
    inner_split.set_sidebar(Some(&thread_page));

    let reading_page = adw::NavigationPage::builder()
        .title("Message")
        .tag("message")
        .child(&build_reading_pane())
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

fn build_sidebar() -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    container.append(&header);

    let status = adw::StatusPage::builder()
        .icon_name("mail-folder-inbox-symbolic")
        .title("Mailboxes")
        .description("Gmail (Personal)\nGmail (Work)\niCloud")
        .build();
    status.set_vexpand(true);
    container.append(&status);

    container.upcast()
}

fn build_thread_list() -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    container.append(&header);

    let status = adw::StatusPage::builder()
        .icon_name("mail-unread-symbolic")
        .title("Inbox")
        .description("Threads will stream here")
        .build();
    status.set_vexpand(true);
    container.append(&status);

    container.upcast()
}

fn build_reading_pane() -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    container.append(&header);

    let status = adw::StatusPage::builder()
        .icon_name("mail-read-symbolic")
        .title("No Message Selected")
        .description("Choose a conversation to view")
        .build();
    status.set_vexpand(true);
    container.append(&status);

    container.upcast()
}
