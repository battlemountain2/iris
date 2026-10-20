#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use mail_parser::{MessageParser, MimeHeaders};

use crate::model::{Account, AttachmentMeta, Message, Thread};

pub struct MaildirScanner {
    root_path: PathBuf,
}

impl MaildirScanner {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    /// Discovers accounts and their mailboxes in the root directory.
    pub fn discover_accounts(&self) -> Vec<Account> {
        let mut accounts = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.root_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') {
                            accounts.push(Account {
                                id: name.clone(),
                                display_name: format_account_name(&name),
                                email: format!("{name}@mail.local"),
                                backend_type: crate::model::AccountBackend::MaildirNotmuch,
                                maildir_path: Some(entry.path().to_string_lossy().to_string()),
                            });
                        }
                    }
                }
            }
        }
        accounts
    }

    /// Scans a specific folder (e.g. INBOX) for messages and groups them into threads.
    pub fn scan_folder(&self, folder_path: &Path) -> Vec<Thread> {
        let mut messages = Vec::new();

        // Standard maildir subdirectories
        for subdir in &["cur", "new"] {
            let path = folder_path.join(subdir);
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let file_path = entry.path();
                    if file_path.is_file() {
                        let path_str = file_path.to_string_lossy();
                        let is_unread = *subdir == "new" || !path_str.contains(":2,") || !path_str.contains('S');
                        let is_flagged = path_str.contains('F');

                        if let Ok(bytes) = fs::read(&file_path) {
                            if let Some(msg) = parse_rfc822(&bytes, &file_path, is_unread, is_flagged) {
                                messages.push(msg);
                            }
                        }
                    }
                }
            }
        }

        group_into_threads(messages)
    }

    /// Automatically seeds demo maildir emails if the directory is empty.
    pub fn ensure_demo_data(&self) -> std::io::Result<()> {
        let inbox_new = self.root_path.join("demo").join("INBOX").join("new");
        if !inbox_new.exists() {
            fs::create_dir_all(&inbox_new)?;
            fs::create_dir_all(self.root_path.join("demo").join("INBOX").join("cur"))?;
            fs::create_dir_all(self.root_path.join("demo").join("INBOX").join("tmp"))?;

            // Email 1
            let email1 = "From: Alex Rivera <alex@hermes.dev>\r\n\
                To: Brayan <bry@omarchy.org>\r\n\
                Subject: Hermes v0.4.0 Release & Audio Overhaul\r\n\
                Date: Mon, 19 Oct 2026 10:42:00 -0600\r\n\
                Message-ID: <hermes-release-040@hermes.dev>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Hey Brayan,\r\n\
                \r\n\
                The Cairo vinyl disc hero and micro-segment visualizer look insane! Just pushed the branch to upstream. \
                The 32-band Goertzel filter bank runs with zero memory allocations directly tapping the stereo F32LE 48 kHz PCM stream.\r\n\
                \r\n\
                Next up, we should ensure the Hermes <-> Iris attachment bridge works smoothly. If someone drags a song or \
                document from Hermes directly into Iris compose, it should immediately attach with zero friction.\r\n\
                \r\n\
                Best,\r\nAlex\r\n";
            fs::write(inbox_new.join("1792464000.1.mail:2,"), email1)?;

            // Email 2
            let email2 = "From: GitHub Notifications <notifications@github.com>\r\n\
                To: Brayan <bry@omarchy.org>\r\n\
                Subject: [battlemountain2/iris] Issue #1: Add JMAP protocol support\r\n\
                Date: Mon, 19 Oct 2026 09:15:00 -0600\r\n\
                Message-ID: <iris-issue-1@github.com>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Issue #1 opened by contributor:\r\n\
                Iris would benefit immensely from native Fastmail JMAP syncing alongside Notmuch Maildir.\r\n";
            fs::write(inbox_new.join("1792464001.2.mail:2,"), email2)?;

            // Email 3
            let email3 = "From: Apple Support <support@apple.com>\r\n\
                To: Brayan <bry@omarchy.org>\r\n\
                Subject: Your iCloud Storage Receipt\r\n\
                Date: Sun, 18 Oct 2026 18:30:00 -0600\r\n\
                Message-ID: <icloud-receipt-oct2026@apple.com>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Your monthly receipt for 2TB iCloud+ storage is now available for download. Total: $9.99.\r\n";
            fs::write(inbox_new.join("1792464002.3.mail:2,S"), email3)?;
        }
        Ok(())
    }
}

fn parse_rfc822(bytes: &[u8], path: &Path, is_unread: bool, is_flagged: bool) -> Option<Message> {
    let parsed = MessageParser::default().parse(bytes)?;

    let subject = parsed.subject().unwrap_or("(No Subject)").to_string();
    let from = parsed
        .from()
        .and_then(|f| f.first())
        .map(|a| {
            if let Some(name) = a.name() {
                format!("{name} <{}>", a.address().unwrap_or_default())
            } else {
                a.address().unwrap_or_default().to_string()
            }
        })
        .unwrap_or_else(|| "Unknown Sender".to_string());

    let to = parsed
        .to()
        .map(|addrs| {
            addrs
                .iter()
                .map(|a| a.address().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();

    let cc = parsed
        .cc()
        .map(|addrs| {
            addrs
                .iter()
                .map(|a| a.address().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();

    let date = parsed
        .date()
        .and_then(|d| DateTime::from_timestamp(d.to_timestamp(), 0))
        .unwrap_or_else(Utc::now);

    let body_plain = parsed.body_text(0).map(|s| s.to_string());
    let body_html = parsed.body_html(0).map(|s| s.to_string());
    let has_remote_images = body_html.as_ref().is_some_and(|html| {
        html.contains("http://") || html.contains("https://")
    });

    let mut attachments = Vec::new();
    for part in parsed.attachments() {
        attachments.push(AttachmentMeta {
            filename: part.attachment_name().unwrap_or("attachment").to_string(),
            content_type: part
                .content_type()
                .map(|c| c.c_type.to_string())
                .unwrap_or_default(),
            size_bytes: part.contents().len() as u64,
            local_path: None,
        });
    }

    let id = parsed
        .message_id()
        .map(String::from)
        .or_else(|| path.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown-id".to_string());

    Some(Message {
        id: id.clone(),
        thread_id: id,
        from,
        to,
        cc,
        date,
        subject,
        body_plain,
        body_html,
        has_remote_images,
        attachments,
        is_unread,
        is_flagged,
    })
}

fn group_into_threads(mut messages: Vec<Message>) -> Vec<Thread> {
    messages.sort_by(|a, b| b.date.cmp(&a.date));

    let mut thread_map: HashMap<String, Vec<Message>> = HashMap::new();

    for msg in messages {
        let key = normalize_subject(&msg.subject);
        thread_map.entry(key).or_default().push(msg);
    }

    let mut threads = Vec::new();
    for (_subject_key, mut msgs) in thread_map {
        msgs.sort_by(|a, b| b.date.cmp(&a.date));
        let latest = &msgs[0];
        let has_unread = msgs.iter().any(|m| m.is_unread);
        let has_flagged = msgs.iter().any(|m| m.is_flagged);
        let senders: Vec<String> = msgs.iter().map(|m| m.from.clone()).collect();
        let snippet = latest
            .body_plain
            .clone()
            .unwrap_or_else(|| latest.subject.clone());

        threads.push(Thread {
            id: latest.id.clone(),
            subject: latest.subject.clone(),
            snippet,
            senders,
            latest_timestamp: latest.date,
            message_count: msgs.len() as u32,
            is_unread: has_unread,
            is_flagged: has_flagged,
            tags: Vec::new(),
            messages: msgs,
        });
    }

    threads.sort_by(|a, b| b.latest_timestamp.cmp(&a.latest_timestamp));
    threads
}

fn normalize_subject(subject: &str) -> String {
    let mut s = subject.trim();
    loop {
        let lower = s.to_lowercase();
        if lower.starts_with("re:") {
            s = s[3..].trim();
        } else if lower.starts_with("fwd:") {
            s = s[4..].trim();
        } else {
            break;
        }
    }
    s.to_lowercase()
}

fn format_account_name(raw: &str) -> String {
    let parts: Vec<&str> = raw.split(&['-', '_'][..]).collect();
    parts
        .iter()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_subject() {
        assert_eq!(normalize_subject("Re: Hello World"), "hello world");
        assert_eq!(normalize_subject("fwd: Re: [Hermes] Update"), "[hermes] update");
        assert_eq!(normalize_subject("Plain Subject"), "plain subject");
    }

    #[test]
    fn test_format_account_name() {
        assert_eq!(format_account_name("gmail-work"), "Gmail Work");
        assert_eq!(format_account_name("icloud_personal"), "Icloud Personal");
    }

    #[test]
    fn test_parse_sample_rfc822() {
        let raw = b"From: Sender <sender@example.com>\r\n\
                    To: Recipient <recip@example.com>\r\n\
                    Subject: Test Subject\r\n\
                    Date: Mon, 19 Oct 2026 12:00:00 +0000\r\n\
                    \r\n\
                    Test email body content.";
        let msg = parse_rfc822(raw, Path::new("dummy.eml"), true, false).expect("valid parse");
        assert_eq!(msg.subject, "Test Subject");
        assert!(msg.from.contains("sender@example.com"));
        assert_eq!(msg.body_plain.as_deref(), Some("Test email body content."));
        assert!(msg.is_unread);
        assert!(!msg.is_flagged);
    }
}
