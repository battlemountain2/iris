// SPDX-License-Identifier: GPL-3.0-or-later

pub mod attachment;
pub mod config;
pub mod html_renderer;
pub mod maildir;
pub mod outbox;
pub mod smtp;
pub mod sync;

pub use maildir::MaildirScanner;
