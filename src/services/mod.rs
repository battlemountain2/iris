// SPDX-License-Identifier: GPL-3.0-or-later

pub mod attachment;
pub mod config;
pub mod contact;
pub mod draft;
pub mod html_renderer;
pub mod maildir;
pub mod markdown;
pub mod notmuch;
pub mod outbox;
pub mod search;
pub mod smtp;
pub mod sync;

pub use maildir::MaildirScanner;
