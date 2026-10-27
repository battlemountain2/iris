// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use adw::prelude::*;

use crate::model::{Message, Thread};
use crate::services::html_renderer::render_body_markup;

pub struct ReadingPaneView {
    pub container: gtk::Box,
    pub back_btn: gtk::Button,
    pub archive_btn: gtk::Button,
    pub trash_btn: gtk::Button,
    pub reply_btn: gtk::Button,
    pub reply_all_btn: gtk::Button,
    pub forward_btn: gtk::Button,
    pub edit_draft_btn: gtk::Button,
    pub star_btn: gtk::Button,
    pub tag_btn: gtk::Button,
    pub subject_label: gtk::Label,
    pub privacy_banner: gtk::Box,
    pub load_images_btn: gtk::Button,
    pub always_load_btn: gtk::Button,
    pub tags_strip: gtk::Box,
    pub timeline_strip: gtk::Box,
    pub messages_container: gtk::Box,
    pub allow_remote_images: Cell<bool>,
    pub current_thread: RefCell<Option<Thread>>,
    pub toast_overlay: RefCell<Option<adw::ToastOverlay>>,
}

impl ReadingPaneView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("iris-reading-pane");

        // Action Headerbar (Back, Archive, Trash, Forward, Reply All, Reply, Star)
        let header = adw::HeaderBar::new();

        let back_btn = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Back to Inbox (Esc)")
            .build();
        back_btn.add_css_class("flat");
        header.pack_start(&back_btn);

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

        let forward_btn = gtk::Button::builder()
            .icon_name("mail-forward-symbolic")
            .tooltip_text("Forward (f)")
            .build();
        forward_btn.add_css_class("flat");
        header.pack_end(&forward_btn);

        let reply_all_btn = gtk::Button::builder()
            .icon_name("mail-reply-all-symbolic")
            .tooltip_text("Reply All (a)")
            .build();
        reply_all_btn.add_css_class("flat");
        header.pack_end(&reply_all_btn);

        let reply_btn = gtk::Button::builder()
            .icon_name("mail-reply-sender-symbolic")
            .tooltip_text("Reply (r)")
            .build();
        reply_btn.add_css_class("flat");
        header.pack_end(&reply_btn);

        let edit_draft_btn = gtk::Button::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text("Edit Draft")
            .label("Edit Draft")
            .build();
        edit_draft_btn.add_css_class("suggested-action");
        edit_draft_btn.set_visible(false);
        header.pack_end(&edit_draft_btn);

        let tag_btn = gtk::Button::builder()
            .icon_name("tag-symbolic")
            .tooltip_text("Labels & Tags (l)")
            .build();
        tag_btn.add_css_class("flat");
        header.pack_end(&tag_btn);

        let star_btn = gtk::Button::builder()
            .icon_name("non-starred-symbolic")
            .tooltip_text("Star / Flag (s)")
            .build();
        star_btn.add_css_class("flat");
        header.pack_end(&star_btn);

        container.append(&header);

        // Privacy Protection Banner
        let (privacy_banner, load_images_btn, always_load_btn) = build_privacy_banner();
        privacy_banner.set_visible(false);
        container.append(&privacy_banner);

        // Scrollable reading area
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content_box.set_margin_start(28);
        content_box.set_margin_end(28);
        content_box.set_margin_top(20);
        content_box.set_margin_bottom(28);

        // Subject Title
        let subject_label = gtk::Label::builder()
            .label("Select a message to read")
            .xalign(0.0)
            .wrap(true)
            .build();
        subject_label.add_css_class("title-1");
        content_box.append(&subject_label);

        // Thread Tags Strip
        let tags_strip = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        tags_strip.set_margin_top(4);
        tags_strip.set_visible(false);
        content_box.append(&tags_strip);

        // Visual Conversation Timeline
        let timeline_strip = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        timeline_strip.set_margin_top(4);
        timeline_strip.set_margin_bottom(8);
        timeline_strip.set_visible(false);
        content_box.append(&timeline_strip);

        // Multi-Message Conversation Container
        let messages_container = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content_box.append(&messages_container);

        let clamp = adw::Clamp::builder()
            .maximum_size(820)
            .tightening_threshold(620)
            .child(&content_box)
            .build();

        scrolled.set_child(Some(&clamp));
        container.append(&scrolled);

        let current_thread = RefCell::new(None);
        let allow_remote_images = Cell::new(false);
        let toast_overlay = RefCell::new(None);

        let view = Self {
            container,
            back_btn,
            archive_btn,
            trash_btn,
            reply_btn,
            reply_all_btn,
            forward_btn,
            edit_draft_btn,
            star_btn,
            tag_btn,
            subject_label,
            privacy_banner,
            load_images_btn,
            always_load_btn,
            tags_strip,
            timeline_strip,
            messages_container,
            allow_remote_images,
            current_thread,
            toast_overlay,
        };
        view.clear();
        view
    }

    /// Attaches the main window's toast overlay for save notifications.
    pub fn set_toast_overlay(&self, overlay: &adw::ToastOverlay) {
        *self.toast_overlay.borrow_mut() = Some(overlay.clone());
    }

    /// Dynamically loads and renders a thread's messages into the reading pane.
    pub fn display_thread(&self, thread: &Thread) {
        self.subject_label.set_visible(true);
        self.subject_label.set_text(&thread.subject);

        self.star_btn.set_visible(true);
        self.tag_btn.set_visible(true);
        self.archive_btn.set_visible(true);
        self.trash_btn.set_visible(true);

        // Update Star icon
        if thread.is_flagged {
            self.star_btn.set_icon_name("starred-symbolic");
            self.star_btn.set_tooltip_text(Some("Unstar (s)"));
            self.star_btn.add_css_class("accent");
        } else {
            self.star_btn.set_icon_name("non-starred-symbolic");
            self.star_btn.set_tooltip_text(Some("Star / Flag (s)"));
            self.star_btn.remove_css_class("accent");
        }

        // Toggle Edit Draft vs Reply/Forward buttons if viewing a draft
        let is_draft = thread.messages.iter().any(|m| {
            m.file_path
                .as_ref()
                .map(|p| p.contains("/Drafts/") || (p.contains(":2,") && p.split(":2,").nth(1).unwrap_or("").contains('D')))
                .unwrap_or(false)
        });

        if is_draft {
            self.edit_draft_btn.set_visible(true);
            self.reply_btn.set_visible(false);
            self.reply_all_btn.set_visible(false);
            self.forward_btn.set_visible(false);
        } else {
            self.edit_draft_btn.set_visible(false);
            self.reply_btn.set_visible(true);
            self.reply_all_btn.set_visible(true);
            self.forward_btn.set_visible(true);
        }

        // Remote image privacy protection banner check across all messages
        let allow = self.allow_remote_images.get();
        let any_remote = thread.messages.iter().any(|m| m.has_remote_images);
        self.privacy_banner.set_visible(any_remote && !allow);

        // Thread Tags Strip
        while let Some(child) = self.tags_strip.first_child() {
            self.tags_strip.remove(&child);
        }
        for tag in &thread.tags {
            let pill = gtk::Label::builder().label(tag).build();
            pill.add_css_class("iris-tag-pill");
            self.tags_strip.append(&pill);
        }
        self.tags_strip.set_visible(!thread.tags.is_empty());

        let mut chronological_msgs = thread.messages.clone();
        chronological_msgs.sort_by_key(|m| m.date);

        // Timeline strip
        while let Some(child) = self.timeline_strip.first_child() {
            self.timeline_strip.remove(&child);
        }
        if chronological_msgs.len() > 1 {
            for (i, msg) in chronological_msgs.iter().enumerate() {
                let s_name = parse_display_name(&msg.from);
                let local_time = msg.date.with_timezone(&chrono::Local).format("%-I:%M %p");
                let pill = gtk::Label::builder()
                    .label(format!("● {} ({local_time})", s_name))
                    .build();
                pill.add_css_class("caption");
                pill.add_css_class("badge");
                self.timeline_strip.append(&pill);

                if i + 1 < chronological_msgs.len() {
                    let arrow = gtk::Label::builder().label("→").build();
                    arrow.add_css_class("dim-label");
                    self.timeline_strip.append(&arrow);
                }
            }
            self.timeline_strip.set_visible(true);
        } else {
            self.timeline_strip.set_visible(false);
        }

        // Render each message card in the conversation in chronological order
        while let Some(child) = self.messages_container.first_child() {
            self.messages_container.remove(&child);
        }

        let toast_ov = self.toast_overlay.borrow().clone();
        let count = chronological_msgs.len();

        for (idx, msg) in chronological_msgs.iter().enumerate() {
            let is_latest = idx == count - 1;
            let card = build_conversation_card(msg, is_latest, allow, toast_ov.clone());
            self.messages_container.append(&card);
        }

        *self.current_thread.borrow_mut() = Some(thread.clone());
    }

    /// Resets the reading pane to an empty placeholder state when no thread is selected.
    pub fn clear(&self) {
        *self.current_thread.borrow_mut() = None;
        self.allow_remote_images.set(false);
        self.subject_label.set_visible(false);
        self.star_btn.set_visible(false);
        self.tag_btn.set_visible(false);
        self.archive_btn.set_visible(false);
        self.trash_btn.set_visible(false);
        self.star_btn.set_icon_name("non-starred-symbolic");
        self.edit_draft_btn.set_visible(false);
        self.reply_btn.set_visible(false);
        self.reply_all_btn.set_visible(false);
        self.forward_btn.set_visible(false);
        self.privacy_banner.set_visible(false);
        self.tags_strip.set_visible(false);
        while let Some(child) = self.tags_strip.first_child() {
            self.tags_strip.remove(&child);
        }
        self.timeline_strip.set_visible(false);
        while let Some(child) = self.timeline_strip.first_child() {
            self.timeline_strip.remove(&child);
        }
        while let Some(child) = self.messages_container.first_child() {
            self.messages_container.remove(&child);
        }

        let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        empty_box.set_halign(gtk::Align::Center);
        empty_box.set_valign(gtk::Align::Center);
        empty_box.set_vexpand(true);
        empty_box.set_margin_top(80);
        empty_box.set_margin_bottom(80);
        empty_box.add_css_class("iris-reading-empty");

        let icon = gtk::Image::from_icon_name("mail-read-symbolic");
        icon.set_pixel_size(64);
        icon.add_css_class("dim-label");
        empty_box.append(&icon);

        let title = gtk::Label::builder()
            .label("No Conversation Selected")
            .build();
        title.add_css_class("title-2");
        title.add_css_class("dim-label");
        empty_box.append(&title);

        let subtitle = gtk::Label::builder()
            .label("Select an email from the list or press 'c' to compose")
            .build();
        subtitle.add_css_class("caption");
        subtitle.add_css_class("dim-label");
        empty_box.append(&subtitle);

        self.messages_container.append(&empty_box);
    }
}

fn build_conversation_card(
    message: &Message,
    is_latest: bool,
    allow_remote_images: bool,
    toast_overlay: Option<adw::ToastOverlay>,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.add_css_class("iris-conversation-card");
    if is_latest {
        card.add_css_class("latest");
    } else {
        card.add_css_class("collapsed");
    }

    // Header Row
    let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header_box.set_hexpand(true);

    let clean_name = parse_display_name(&message.from);
    let avatar = adw::Avatar::builder()
        .size(if is_latest { 40 } else { 32 })
        .text(&clean_name)
        .show_initials(true)
        .build();
    header_box.append(&avatar);

    let text_vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text_vbox.set_hexpand(true);

    let top_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let name_label = gtk::Label::builder()
        .label(&clean_name)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    name_label.add_css_class(if is_latest { "heading" } else { "body" });
    top_row.append(&name_label);

    let local_dt = message.date.with_timezone(&chrono::Local);
    let date_str = local_dt.format("%b %-d, %Y at %-I:%M %p").to_string();
    let date_label = gtk::Label::builder()
        .label(&date_str)
        .xalign(1.0)
        .hexpand(true)
        .build();
    date_label.add_css_class("caption");
    date_label.add_css_class("dim-label");
    top_row.append(&date_label);
    text_vbox.append(&top_row);

    // Subtitle: Email address when expanded, snippet when collapsed
    let email_label = gtk::Label::builder()
        .label(&format!("From: {} · To: {}", message.from, message.to.join(", ")))
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    email_label.add_css_class("caption");
    email_label.add_css_class("dim-label");
    text_vbox.append(&email_label);

    let snippet_text = message
        .body_plain
        .as_deref()
        .unwrap_or("")
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let snippet_label = gtk::Label::builder()
        .label(snippet_text)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    snippet_label.add_css_class("caption");
    snippet_label.add_css_class("dim-label");
    snippet_label.set_visible(!is_latest);
    text_vbox.append(&snippet_label);

    header_box.append(&text_vbox);

    let chevron_btn = if !is_latest {
        let btn = gtk::Button::builder()
            .icon_name("pan-down-symbolic")
            .tooltip_text("Expand message")
            .build();
        btn.add_css_class("flat");
        header_box.append(&btn);
        Some(btn)
    } else {
        None
    };

    card.append(&header_box);

    // Body & Attachments Container
    let body_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body_box.set_visible(is_latest);

    // Message Body
    let body_label = gtk::Label::builder()
        .label("")
        .use_markup(true)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .selectable(true)
        .build();
    body_label.add_css_class("body");
    body_label.add_css_class("iris-reading-body");

    let markup = render_body_markup(
        message.body_html.as_deref(),
        message.body_plain.as_deref(),
        allow_remote_images,
    );
    body_label.set_markup(&markup);

    body_label.connect_activate_link(|_label, uri| {
        if let Err(err) = gio::AppInfo::launch_default_for_uri(uri, None::<&gio::AppLaunchContext>) {
            tracing::error!("Failed to launch link '{uri}': {err}");
        }
        glib::Propagation::Stop
    });

    body_box.append(&body_label);

    // Attachments Container
    if !message.attachments.is_empty() {
        let att_vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
        att_vbox.set_margin_top(8);

        let att_heading = gtk::Label::builder()
            .label(&format!("📎 Attachments ({})", message.attachments.len()))
            .xalign(0.0)
            .build();
        att_heading.add_css_class("caption");
        att_heading.add_css_class("dim-label");
        att_vbox.append(&att_heading);

        for (att_idx, att) in message.attachments.iter().enumerate() {
            let row = build_attachment_row(
                &att.filename,
                att.size_bytes,
                message.file_path.clone(),
                att_idx,
                toast_overlay.clone(),
            );
            att_vbox.append(&row);
        }

        body_box.append(&att_vbox);
    }

    card.append(&body_box);

    // Expand / Collapse Interactivity for Older Messages
    if !is_latest {
        let is_expanded = Rc::new(Cell::new(false));
        let body_box_clone = body_box.clone();
        let card_clone = card.clone();
        let snippet_clone = snippet_label.clone();
        let email_clone = email_label.clone();
        let chevron_clone = chevron_btn.clone();
        let is_exp = is_expanded.clone();

        let toggle = move || {
            let next = !is_exp.get();
            is_exp.set(next);
            body_box_clone.set_visible(next);
            snippet_clone.set_visible(!next);
            email_clone.set_visible(next);

            if next {
                card_clone.remove_css_class("collapsed");
                if let Some(ref btn) = chevron_clone {
                    btn.set_icon_name("pan-up-symbolic");
                    btn.set_tooltip_text(Some("Collapse message"));
                }
            } else {
                card_clone.add_css_class("collapsed");
                if let Some(ref btn) = chevron_clone {
                    btn.set_icon_name("pan-down-symbolic");
                    btn.set_tooltip_text(Some("Expand message"));
                }
            }
        };

        if let Some(ref btn) = chevron_btn {
            let toggle_btn = toggle.clone();
            btn.connect_clicked(move |_| {
                toggle_btn();
            });
        }

        let gesture = gtk::GestureClick::new();
        let toggle_gesture = toggle.clone();
        gesture.connect_released(move |_, n_press, _, _| {
            if n_press == 1 {
                toggle_gesture();
            }
        });
        header_box.add_controller(gesture);
    }

    card
}

fn build_attachment_row(
    filename: &str,
    size_bytes: u64,
    msg_file_path: Option<String>,
    attachment_index: usize,
    toast_overlay: Option<adw::ToastOverlay>,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    card.add_css_class("card");
    card.add_css_class("iris-attachment-card");

    // Dynamic icon based on extension
    let icon_name = match Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("pdf") => "x-office-document-symbolic",
        Some("png") | Some("jpg") | Some("jpeg") | Some("webp") | Some("gif") | Some("svg") => {
            "image-x-generic-symbolic"
        }
        Some("mp3") | Some("flac") | Some("wav") | Some("m4a") | Some("ogg") => {
            "audio-x-generic-symbolic"
        }
        Some("mp4") | Some("mkv") | Some("webm") => "video-x-generic-symbolic",
        Some("zip") | Some("tar") | Some("gz") | Some("7z") => "package-x-generic-symbolic",
        _ => "text-x-generic-symbolic",
    };

    let file_icon = gtk::Image::from_icon_name(icon_name);
    card.append(&file_icon);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let name_label = gtk::Label::builder()
        .label(filename)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    name_label.add_css_class("body");
    vbox.append(&name_label);

    let size_str = format!("{:.1} KB", size_bytes as f64 / 1024.0);
    let size_label = gtk::Label::builder().label(&size_str).xalign(0.0).build();
    size_label.add_css_class("caption");
    size_label.add_css_class("dim-label");
    vbox.append(&size_label);

    card.append(&vbox);

    let save_btn = gtk::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Save to ~/Downloads/Hermes")
        .build();
    save_btn.add_css_class("flat");
    card.append(&save_btn);

    // Track saved state
    let saved_path_cell: Rc<RefCell<Option<std::path::PathBuf>>> = Rc::new(RefCell::new(None));

    let filename_owned = filename.to_string();
    let save_btn_clone = save_btn.clone();
    let saved_cell = saved_path_cell.clone();

    save_btn.connect_clicked(move |_| {
        if let Some(saved) = saved_cell.borrow().as_ref() {
            // Already saved: open in Hermes or system file manager
            let _ = crate::services::attachment::open_in_hermes(saved);
            return;
        }

        if let Some(ref path_str) = msg_file_path {
            let path = Path::new(path_str);
            match crate::services::attachment::extract_and_save_attachment(path, attachment_index, None) {
                Ok((saved_path, fname)) => {
                    crate::services::attachment::notify_attachment_saved(&fname, &saved_path);
                    *saved_cell.borrow_mut() = Some(saved_path.clone());

                    save_btn_clone.set_icon_name("emblem-ok-symbolic");
                    save_btn_clone.set_tooltip_text(Some("Saved! Click to Open in Hermes"));

                    if let Some(ref toast_ov) = toast_overlay {
                        let toast = adw::Toast::builder()
                            .title(format!("Saved {fname} to ~/Downloads/Hermes"))
                            .button_label("Open in Hermes")
                            .timeout(5)
                            .build();

                        let saved_open = saved_path.clone();
                        toast.connect_button_clicked(move |_| {
                            let _ = crate::services::attachment::open_in_hermes(&saved_open);
                        });
                        toast_ov.add_toast(toast);
                    }
                }
                Err(err) => {
                    if let Some(ref toast_ov) = toast_overlay {
                        let toast = adw::Toast::builder()
                            .title(format!("Failed to save: {err}"))
                            .timeout(4)
                            .build();
                        toast_ov.add_toast(toast);
                    }
                }
            }
        } else {
            if let Some(ref toast_ov) = toast_overlay {
                let toast = adw::Toast::builder()
                    .title(format!("Attachment '{filename_owned}' has no local file source"))
                    .timeout(3)
                    .build();
                toast_ov.add_toast(toast);
            }
        }
    });

    card
}

fn build_privacy_banner() -> (gtk::Box, gtk::Button, gtk::Button) {
    let banner = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    banner.add_css_class("iris-privacy-banner");

    let shield_icon = gtk::Image::from_icon_name("security-high-symbolic");
    shield_icon.add_css_class("iris-shield-icon");
    banner.append(&shield_icon);

    let text_label = gtk::Label::builder()
        .label("Remote images hidden to protect your privacy and block tracking.")
        .hexpand(true)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .build();
    text_label.add_css_class("caption");
    banner.append(&text_label);

    let load_once_btn = gtk::Button::builder().label("Load Images").build();
    load_once_btn.add_css_class("flat");
    load_once_btn.add_css_class("iris-pill-btn");
    banner.append(&load_once_btn);

    let always_btn = gtk::Button::builder().label("Always Load").build();
    always_btn.add_css_class("flat");
    always_btn.add_css_class("iris-pill-btn");
    banner.append(&always_btn);

    (banner, load_once_btn, always_btn)
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
