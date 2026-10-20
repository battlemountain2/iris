// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;
use gtk::glib;

pub struct ReadingPaneView {
    pub container: gtk::Box,
}

impl ReadingPaneView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-reading-pane");

        // Action Headerbar (Archive, Trash, Reply, Star, More)
        let header = adw::HeaderBar::new();

        // Right side actions
        let archive_btn = gtk::Button::builder()
            .icon_name("mail-archive-symbolic")
            .tooltip_text("Archive (e)")
            .build();
        archive_btn.add_css_class("flat");
        header.pack_end(&archive_btn);

        let trash_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text("Delete (#)")
            .build();
        trash_btn.add_css_class("flat");
        header.pack_end(&trash_btn);

        let reply_btn = gtk::Button::builder()
            .icon_name("mail-reply-sender-symbolic")
            .tooltip_text("Reply (r)")
            .build();
        reply_btn.add_css_class("flat");
        header.pack_end(&reply_btn);

        let star_btn = gtk::Button::builder()
            .icon_name("starred-symbolic")
            .tooltip_text("Star / Flag (s)")
            .build();
        star_btn.add_css_class("flat");
        header.pack_end(&star_btn);

        container.append(&header);

        // Privacy Protection Banner
        let privacy_banner = build_privacy_banner();
        container.append(&privacy_banner);

        // Scrollable reading area
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(18);
        content_box.set_margin_bottom(24);

        // Subject Title
        let subject_label = gtk::Label::builder()
            .label("Hermes v0.4.0 Release & Audio Overhaul")
            .xalign(0.0)
            .wrap(true)
            .build();
        subject_label.add_css_class("title-1");
        content_box.append(&subject_label);

        // Sender Info Header Card
        let sender_card = build_sender_header(
            "Alex Rivera",
            "alex@hermes.dev",
            "Today at 10:42 AM",
        );
        content_box.append(&sender_card);

        // Visual Conversation Timeline placeholder (Roadmap item #2)
        let timeline_strip = build_timeline_strip();
        content_box.append(&timeline_strip);

        // Message Body
        let body_label = gtk::Label::builder()
            .label(
                "Hey Brayan,\n\n\
                The Cairo vinyl disc hero and micro-segment visualizer look insane! Just pushed the branch \
                to upstream. The 32-band Goertzel filter bank runs with zero memory allocations directly \
                tapping the stereo F32LE 48 kHz PCM stream.\n\n\
                Next up, we should ensure the Hermes <-> Iris attachment bridge works smoothly. If someone \
                drags a song or document from Hermes directly into Iris compose, it should immediately attach \
                with zero friction.\n\n\
                Let me know what you think of the latest build!\n\n\
                Best,\nAlex"
            )
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .build();
        body_label.add_css_class("body");
        content_box.append(&body_label);

        // Attachment Card
        let attachment_card = build_attachment_row("hermes-v0.4.0-preview.tar.gz", "14.2 MB");
        content_box.append(&attachment_card);

        scrolled.set_child(Some(&content_box));
        container.append(&scrolled);

        Self { container }
    }
}

fn build_privacy_banner() -> gtk::Box {
    let banner = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    banner.add_css_class("iris-privacy-banner");

    let shield_icon = gtk::Image::from_icon_name("security-high-symbolic");
    banner.append(&shield_icon);

    let text_label = gtk::Label::builder()
        .label("Remote images hidden to protect your privacy and block tracking.")
        .hexpand(true)
        .xalign(0.0)
        .build();
    text_label.add_css_class("caption");
    banner.append(&text_label);

    let load_once_btn = gtk::Button::builder()
        .label("Load Images")
        .build();
    load_once_btn.add_css_class("flat");
    banner.append(&load_once_btn);

    let always_btn = gtk::Button::builder()
        .label("Always Load")
        .build();
    always_btn.add_css_class("flat");
    banner.append(&always_btn);

    banner
}

fn build_sender_header(name: &str, email: &str, date: &str) -> gtk::Box {
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 12);

    // Avatar Circle (Initials "AR")
    let avatar = adw::Avatar::builder()
        .size(42)
        .text(name)
        .show_initials(true)
        .build();
    hbox.append(&avatar);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let name_label = gtk::Label::builder()
        .label(name)
        .xalign(0.0)
        .build();
    name_label.add_css_class("heading");
    vbox.append(&name_label);

    let email_label = gtk::Label::builder()
        .label(&format!("From: {email} · To: me"))
        .xalign(0.0)
        .build();
    email_label.add_css_class("caption");
    email_label.add_css_class("dim-label");
    vbox.append(&email_label);

    hbox.append(&vbox);

    let date_label = gtk::Label::builder()
        .label(date)
        .xalign(1.0)
        .build();
    date_label.add_css_class("caption");
    date_label.add_css_class("dim-label");
    hbox.append(&date_label);

    hbox
}

fn build_timeline_strip() -> gtk::Box {
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    strip.set_margin_top(4);
    strip.set_margin_bottom(8);

    let pill1 = gtk::Label::builder()
        .label("● Alex Rivera (10:42 AM)")
        .build();
    pill1.add_css_class("caption");
    pill1.add_css_class("badge");
    strip.append(&pill1);

    let arrow = gtk::Label::builder().label("→").build();
    arrow.add_css_class("dim-label");
    strip.append(&arrow);

    let pill2 = gtk::Label::builder()
        .label("You (Draft)")
        .build();
    pill2.add_css_class("caption");
    pill2.add_css_class("dim-label");
    strip.append(&pill2);

    strip
}

fn build_attachment_row(filename: &str, size: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    card.add_css_class("card");
    card.set_margin_top(12);

    let file_icon = gtk::Image::from_icon_name("package-x-generic-symbolic");
    card.append(&file_icon);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let name_label = gtk::Label::builder()
        .label(filename)
        .xalign(0.0)
        .build();
    name_label.add_css_class("body");
    vbox.append(&name_label);

    let size_label = gtk::Label::builder()
        .label(size)
        .xalign(0.0)
        .build();
    size_label.add_css_class("caption");
    size_label.add_css_class("dim-label");
    vbox.append(&size_label);

    card.append(&vbox);

    let save_btn = gtk::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Save to Hermes Downloads")
        .build();
    save_btn.add_css_class("flat");
    card.append(&save_btn);

    card
}
