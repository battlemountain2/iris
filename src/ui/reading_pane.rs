#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use adw::prelude::*;

use crate::model::Thread;

pub struct ReadingPaneView {
    pub container: gtk::Box,
    pub subject_label: gtk::Label,
    pub avatar: adw::Avatar,
    pub name_label: gtk::Label,
    pub email_label: gtk::Label,
    pub date_label: gtk::Label,
    pub body_label: gtk::Label,
    pub privacy_banner: gtk::Box,
    pub timeline_strip: gtk::Box,
    pub attachment_box: gtk::Box,
    pub current_thread: RefCell<Option<Thread>>,
}

impl ReadingPaneView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-reading-pane");

        // Action Headerbar (Archive, Trash, Reply, Star)
        let header = adw::HeaderBar::new();

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
        privacy_banner.set_visible(false);
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
            .label("Select a message to read")
            .xalign(0.0)
            .wrap(true)
            .build();
        subject_label.add_css_class("title-1");
        content_box.append(&subject_label);

        // Sender Info Header Card
        let (sender_card, avatar, name_label, email_label, date_label) = build_sender_header();
        content_box.append(&sender_card);

        // Visual Conversation Timeline
        let timeline_strip = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        timeline_strip.set_margin_top(4);
        timeline_strip.set_margin_bottom(8);
        timeline_strip.set_visible(false);
        content_box.append(&timeline_strip);

        // Message Body
        let body_label = gtk::Label::builder()
            .label("")
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .build();
        body_label.add_css_class("body");
        content_box.append(&body_label);

        // Attachment Container
        let attachment_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        attachment_box.set_visible(false);
        content_box.append(&attachment_box);

        scrolled.set_child(Some(&content_box));
        container.append(&scrolled);

        Self {
            container,
            subject_label,
            avatar,
            name_label,
            email_label,
            date_label,
            body_label,
            privacy_banner,
            timeline_strip,
            attachment_box,
            current_thread: RefCell::new(None),
        }
    }

    /// Dynamically loads and renders a thread's messages into the reading pane.
    pub fn display_thread(&self, thread: &Thread) {
        self.subject_label.set_text(&thread.subject);

        let latest_msg = thread.messages.first();

        let sender = thread
            .senders
            .first()
            .map(|s| s.as_str())
            .unwrap_or("Unknown");
        let clean_name = parse_display_name(sender);
        self.avatar.set_text(Some(&clean_name));
        self.name_label.set_text(&clean_name);
        self.email_label.set_text(&format!("From: {sender} · To: me"));

        let date_str = thread
            .latest_timestamp
            .format("%b %d, %Y at %H:%M")
            .to_string();
        self.date_label.set_text(&date_str);

        // Body text
        let body_text = latest_msg
            .and_then(|m| m.body_plain.as_deref())
            .unwrap_or(&thread.snippet);
        self.body_label.set_text(body_text);

        // Remote image privacy banner
        let has_remote = latest_msg.map(|m| m.has_remote_images).unwrap_or(false);
        self.privacy_banner.set_visible(has_remote);

        // Timeline strip
        while let Some(child) = self.timeline_strip.first_child() {
            self.timeline_strip.remove(&child);
        }
        if thread.messages.len() > 1 {
            for (i, msg) in thread.messages.iter().enumerate() {
                let s_name = parse_display_name(&msg.from);
                let pill = gtk::Label::builder()
                    .label(format!("● {} ({})", s_name, msg.date.format("%H:%M")))
                    .build();
                pill.add_css_class("caption");
                pill.add_css_class("badge");
                self.timeline_strip.append(&pill);

                if i + 1 < thread.messages.len() {
                    let arrow = gtk::Label::builder().label("→").build();
                    arrow.add_css_class("dim-label");
                    self.timeline_strip.append(&arrow);
                }
            }
            self.timeline_strip.set_visible(true);
        } else {
            self.timeline_strip.set_visible(false);
        }

        // Attachments
        while let Some(child) = self.attachment_box.first_child() {
            self.attachment_box.remove(&child);
        }
        let attachments = latest_msg.map(|m| &m.attachments[..]).unwrap_or(&[]);
        if !attachments.is_empty() {
            for att in attachments {
                let size_str = format!("{:.1} KB", att.size_bytes as f64 / 1024.0);
                let row = build_attachment_row(&att.filename, &size_str);
                self.attachment_box.append(&row);
            }
            self.attachment_box.set_visible(true);
        } else {
            self.attachment_box.set_visible(false);
        }

        *self.current_thread.borrow_mut() = Some(thread.clone());
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

    let load_once_btn = gtk::Button::builder().label("Load Images").build();
    load_once_btn.add_css_class("flat");
    banner.append(&load_once_btn);

    let always_btn = gtk::Button::builder().label("Always Load").build();
    always_btn.add_css_class("flat");
    banner.append(&always_btn);

    banner
}

fn build_sender_header() -> (
    gtk::Box,
    adw::Avatar,
    gtk::Label,
    gtk::Label,
    gtk::Label,
) {
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 12);

    let avatar = adw::Avatar::builder()
        .size(42)
        .show_initials(true)
        .build();
    hbox.append(&avatar);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let name_label = gtk::Label::builder()
        .label("")
        .xalign(0.0)
        .build();
    name_label.add_css_class("heading");
    vbox.append(&name_label);

    let email_label = gtk::Label::builder()
        .label("")
        .xalign(0.0)
        .build();
    email_label.add_css_class("caption");
    email_label.add_css_class("dim-label");
    vbox.append(&email_label);

    hbox.append(&vbox);

    let date_label = gtk::Label::builder().label("").xalign(1.0).build();
    date_label.add_css_class("caption");
    date_label.add_css_class("dim-label");
    hbox.append(&date_label);

    (hbox, avatar, name_label, email_label, date_label)
}

fn build_attachment_row(filename: &str, size: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    card.add_css_class("card");
    card.set_margin_top(8);

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

    let size_label = gtk::Label::builder().label(size).xalign(0.0).build();
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

fn parse_display_name(raw: &str) -> String {
    if let Some(pos) = raw.find('<') {
        let name = raw[..pos].trim().trim_matches('"');
        if !name.is_empty() {
            return name.to_string();
        }
    }
    raw.split('@').next().unwrap_or(raw).to_string()
}
