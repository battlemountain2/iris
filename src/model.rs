#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a configured user email account.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub display_name: String,
    pub email: String,
    pub backend_type: AccountBackend,
    pub maildir_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AccountBackend {
    MaildirNotmuch,
    GmailOAuth2,
    ICloudImap,
}

/// A standard or custom mailbox folder (e.g. Inbox, Sent, Archive, Trash).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Mailbox {
    pub id: String,
    pub account_id: Option<String>,
    pub name: String,
    pub icon_name: String,
    pub unread_count: u32,
    pub total_count: u32,
    pub mailbox_type: MailboxType,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum MailboxType {
    UnifiedInbox,
    AccountInbox,
    Sent,
    Archive,
    Drafts,
    Trash,
    Feed, // Ephemeral Newsletter / Feed mode
    CustomTag,
}

/// A threaded conversation grouping one or more related messages.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Thread {
    pub id: String,
    pub subject: String,
    pub snippet: String,
    pub senders: Vec<String>,
    pub latest_timestamp: DateTime<Utc>,
    pub message_count: u32,
    pub is_unread: bool,
    pub is_flagged: bool,
    pub tags: Vec<String>,
    pub messages: Vec<Message>,
}

/// A single email message.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub date: DateTime<Utc>,
    pub subject: String,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub has_remote_images: bool,
    pub attachments: Vec<AttachmentMeta>,
    pub is_unread: bool,
    pub is_flagged: bool,
}

/// Metadata for an email attachment.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AttachmentMeta {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub local_path: Option<String>,
}

/// User application preferences.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preferences {
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String, // "system", "noctalia", "custom"
    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default = "default_true")]
    pub subtle_mode: bool, // Accent-only vs Full Canvas
    #[serde(default = "default_true")]
    pub privacy_protection: bool, // Block tracking pixels / external images
    #[serde(default = "default_undo_send")]
    pub undo_send_seconds: u32, // 10s default
    #[serde(default = "default_true")]
    pub trackpad_gestures: bool, // 2-finger swipe actions
    #[serde(default = "default_font")]
    pub interface_font: String,
}

fn default_theme_mode() -> String {
    "system".to_string()
}
fn default_theme_name() -> String {
    "catppuccin".to_string()
}
fn default_undo_send() -> u32 {
    10
}
fn default_true() -> bool {
    true
}
fn default_font() -> String {
    "Inter".to_string()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme_mode: default_theme_mode(),
            theme_name: default_theme_name(),
            subtle_mode: true,
            privacy_protection: true,
            undo_send_seconds: 10,
            trackpad_gestures: true,
            interface_font: default_font(),
        }
    }
}
