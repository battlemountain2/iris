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
        let demo_root = self.root_path.join("demo");
        ensure_maildir_folder(&demo_root.join("INBOX"))?;
        ensure_maildir_folder(&demo_root.join("Archive"))?;
        ensure_maildir_folder(&demo_root.join("Trash"))?;
        ensure_maildir_folder(&demo_root.join("Sent"))?;

        let inbox_new = demo_root.join("INBOX").join("new");
        let inbox_cur = demo_root.join("INBOX").join("cur");
        let archive_cur = demo_root.join("Archive").join("cur");
        let sent_cur = demo_root.join("Sent").join("cur");

        // Seed Inbox if empty
        let inbox_empty = fs::read_dir(&inbox_new).map(|mut d| d.next().is_none()).unwrap_or(true)
            && fs::read_dir(&inbox_cur).map(|mut d| d.next().is_none()).unwrap_or(true);

        if inbox_empty {
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

            // Email 4 (Rich HTML with Remote Tracking Image & Links)
            let email4 = "From: GitHub Highlights <notifications@github.com>\r\n\
                To: Brayan <bry@omarchy.org>\r\n\
                Subject: [Iris & Hermes] Weekly Community Highlights\r\n\
                Date: Mon, 19 Oct 2026 15:30:00 -0600\r\n\
                Message-ID: <highlights-oct2026@github.com>\r\n\
                Content-Type: text/html; charset=utf-8\r\n\
                \r\n\
                <h2>Weekly Community Highlights</h2>\r\n\
                <p>Hey <b>Brayan</b>,</p>\r\n\
                <p>Iris now features <b>liquid-glass translucency</b> for Hyprland, rich HTML email rendering, and full <b>Mail Privacy Protection</b>.</p>\r\n\
                <ul>\r\n\
                  <li>Explore the repo: <a href=\"https://github.com/battlemountain2/iris\">battlemountain2/iris on GitHub</a></li>\r\n\
                  <li>Sister client: <a href=\"https://github.com/battlemountain2/hermes\">battlemountain2/hermes</a></li>\r\n\
                </ul>\r\n\
                <blockquote>\"Fast, liquid-glass, keyboard-driven Linux email client designed for Arch Linux & Hyprland.\"</blockquote>\r\n\
                <p>Tracking pixel beacon: <img src=\"https://github.com/images/beacon-pixel.png\" alt=\"Telemetry Beacon\" /></p>\r\n";
            fs::write(inbox_new.join("1792464003.4.mail:2,"), email4)?;
        }

        // Seed Archive if empty
        let archive_empty = fs::read_dir(&archive_cur).map(|mut d| d.next().is_none()).unwrap_or(true);
        if archive_empty {
            let email_arch = "From: Iris Design <design@iris.local>\r\n\
                To: Brayan <bry@omarchy.org>\r\n\
                Subject: Welcome to Iris — Liquid Glass Mail\r\n\
                Date: Sat, 17 Oct 2026 14:10:00 -0600\r\n\
                Message-ID: <iris-welcome-1@iris.local>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Welcome to Iris!\r\n\
                \r\n\
                Iris is designed to be the fastest, most beautiful keyboard-driven mail client for Linux & Hyprland. \
                Featuring dual-kawase translucent sidebar blur, Vim modal navigation (j/k/e/#/c), and seamless \
                integration with Hermes.\r\n";
            fs::write(archive_cur.join("1792460000.arch.mail:2,S"), email_arch)?;
        }

        // Seed Sent if empty
        let sent_empty = fs::read_dir(&sent_cur).map(|mut d| d.next().is_none()).unwrap_or(true);
        if sent_empty {
            let email_sent = "From: Brayan <bry@omarchy.org>\r\n\
                To: Alex Rivera <alex@hermes.dev>\r\n\
                Subject: Re: Hermes v0.4.0 Release & Audio Overhaul\r\n\
                Date: Mon, 19 Oct 2026 11:05:00 -0600\r\n\
                Message-ID: <reply-hermes-040@omarchy.org>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Alex,\r\n\
                \r\n\
                The 32-band Goertzel filter bank sounds incredible. I'm adding full attachment integration in Iris \
                so music files can be sent straight to Hermes playlists.\r\n\
                \r\n\
                Best,\r\nBrayan\r\n";
            fs::write(sent_cur.join("1792465000.sent.mail:2,S"), email_sent)?;
        }

        Ok(())
    }

    /// Quickly counts unread messages in a maildir folder without parsing bodies.
    pub fn count_unread_in_folder(&self, folder_path: &Path) -> u32 {
        let mut count = 0;
        for subdir in &["new", "cur"] {
            let dir = folder_path.join(subdir);
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_file() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if *subdir == "new" || !name.contains(":2,") || !name.contains('S') {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    /// Counts total messages in a maildir folder.
    pub fn count_total_in_folder(&self, folder_path: &Path) -> u32 {
        let mut count = 0;
        for subdir in &["new", "cur"] {
            let dir = folder_path.join(subdir);
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_file() {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }
}

/// Ensures the standard maildir subdirectories exist.
pub fn ensure_maildir_folder(folder_path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(folder_path.join("cur"))?;
    fs::create_dir_all(folder_path.join("new"))?;
    fs::create_dir_all(folder_path.join("tmp"))?;
    Ok(())
}

/// Moves a message file to the target maildir folder (into its `cur` directory).
/// Returns `(original_path, destination_path)` on success.
pub fn move_message_to_folder(
    file_path: &Path,
    target_folder: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    if !file_path.exists() {
        return Err(format!("Source message file does not exist: {}", file_path.display()));
    }

    let cur_dir = target_folder.join("cur");
    if !cur_dir.exists() {
        fs::create_dir_all(&cur_dir)
            .map_err(|e| format!("Failed to create folder {}: {e}", cur_dir.display()))?;
    }

    let file_name = file_path
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?;

    let dest_path = cur_dir.join(file_name);
    if file_path == dest_path {
        return Ok((file_path.to_path_buf(), dest_path));
    }

    fs::rename(file_path, &dest_path).map_err(|e| {
        format!(
            "Failed to move {} to {}: {e}",
            file_path.display(),
            dest_path.display()
        )
    })?;

    Ok((file_path.to_path_buf(), dest_path))
}

/// Moves all messages in a thread to the `Archive` folder of the account.
/// Returns a list of `(source_path, dest_path)` pairs that can be reverted by `restore_moved_files`.
pub fn archive_thread(
    thread: &Thread,
    account_root: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let archive_folder = account_root.join("Archive");
    let mut moves = Vec::new();

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let src = PathBuf::from(path_str);
            match move_message_to_folder(&src, &archive_folder) {
                Ok(moved) => moves.push(moved),
                Err(err) => {
                    let _ = restore_moved_files(&moves);
                    return Err(format!("Archive error: {err}"));
                }
            }
        }
    }

    Ok(moves)
}

/// Moves all messages in a thread to the `Trash` folder of the account.
/// Returns a list of `(source_path, dest_path)` pairs that can be reverted by `restore_moved_files`.
pub fn trash_thread(
    thread: &Thread,
    account_root: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let trash_folder = account_root.join("Trash");
    let mut moves = Vec::new();

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let src = PathBuf::from(path_str);
            match move_message_to_folder(&src, &trash_folder) {
                Ok(moved) => moves.push(moved),
                Err(err) => {
                    let _ = restore_moved_files(&moves);
                    return Err(format!("Trash error: {err}"));
                }
            }
        }
    }

    Ok(moves)
}

/// Reverses a set of file moves (e.g. for the 10-second Undo action).
pub fn restore_moved_files(moves: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    for (original_path, current_path) in moves {
        if current_path.exists() {
            if let Some(parent) = original_path.parent() {
                if !parent.exists() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            fs::rename(current_path, original_path).map_err(|e| {
                format!(
                    "Failed to restore {} to {}: {e}",
                    current_path.display(),
                    original_path.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Helper to update Maildir flags encoded after `:2,` in a filename.
pub fn update_maildir_flags(
    filename: &str,
    add_flag: Option<char>,
    remove_flag: Option<char>,
) -> String {
    let parts: Vec<&str> = filename.split(":2,").collect();
    let base = parts[0];
    let mut flags: Vec<char> = if parts.len() > 1 {
        parts[1].chars().collect()
    } else {
        Vec::new()
    };

    if let Some(rf) = remove_flag {
        flags.retain(|&c| c != rf);
    }

    if let Some(af) = add_flag {
        if !flags.contains(&af) {
            flags.push(af);
        }
    }

    flags.sort_unstable();
    let flags_str: String = flags.into_iter().collect();
    format!("{base}:2,{flags_str}")
}

/// Toggles read status for all messages in a thread on disk.
/// If thread is unread, marks as read (adds 'S', moves from `new/` to `cur/`).
/// If thread is read, marks as unread (removes 'S').
/// Returns the new `is_unread` state.
pub fn toggle_thread_read(thread: &Thread) -> Result<bool, String> {
    let target_unread = !thread.is_unread;

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                continue;
            }

            let file_name = path
                .file_name()
                .ok_or_else(|| "Invalid file name".to_string())?
                .to_string_lossy();

            let new_name = if target_unread {
                update_maildir_flags(&file_name, None, Some('S'))
            } else {
                update_maildir_flags(&file_name, Some('S'), None)
            };

            let parent = path.parent().unwrap_or(Path::new(""));
            let parent_str = parent.to_string_lossy();
            let dest_path = if !target_unread && parent_str.ends_with("new") {
                if let Some(grandparent) = parent.parent() {
                    let cur_dir = grandparent.join("cur");
                    let _ = fs::create_dir_all(&cur_dir);
                    cur_dir.join(&new_name)
                } else {
                    parent.join(&new_name)
                }
            } else {
                parent.join(&new_name)
            };

            if path != dest_path {
                fs::rename(&path, &dest_path)
                    .map_err(|e| format!("Failed to update read flag: {e}"))?;
            }
        }
    }

    Ok(target_unread)
}

/// Toggles star/flagged status for all messages in a thread on disk.
/// Returns the new `is_flagged` state.
pub fn toggle_thread_flag(thread: &Thread) -> Result<bool, String> {
    let target_flagged = !thread.is_flagged;

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                continue;
            }

            let file_name = path
                .file_name()
                .ok_or_else(|| "Invalid file name".to_string())?
                .to_string_lossy();

            let new_name = if target_flagged {
                update_maildir_flags(&file_name, Some('F'), None)
            } else {
                update_maildir_flags(&file_name, None, Some('F'))
            };

            let parent = path.parent().unwrap_or(Path::new(""));
            let dest_path = parent.join(&new_name);

            if path != dest_path {
                fs::rename(&path, &dest_path)
                    .map_err(|e| format!("Failed to update flag: {e}"))?;
            }
        }
    }

    Ok(target_flagged)
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
        file_path: Some(path.to_string_lossy().to_string()),
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

    #[test]
    fn test_archive_and_restore_thread() {
        let temp_dir = std::env::temp_dir().join(format!("iris_test_{}", std::process::id()));
        let account_root = temp_dir.join("test_account");
        let inbox_cur = account_root.join("INBOX").join("cur");
        let archive_cur = account_root.join("Archive").join("cur");

        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&inbox_cur).unwrap();
        fs::create_dir_all(&archive_cur).unwrap();

        let mail_file = inbox_cur.join("100.mail:2,S");
        fs::write(&mail_file, b"From: test@iris.local\r\nSubject: Test\r\n\r\nHello").unwrap();

        let msg = Message {
            id: "msg-1".to_string(),
            thread_id: "thread-1".to_string(),
            from: "test@iris.local".to_string(),
            to: vec!["me@iris.local".to_string()],
            cc: vec![],
            date: Utc::now(),
            subject: "Test".to_string(),
            body_plain: Some("Hello".to_string()),
            body_html: None,
            has_remote_images: false,
            attachments: vec![],
            is_unread: false,
            is_flagged: false,
            file_path: Some(mail_file.to_string_lossy().to_string()),
        };

        let thread = Thread {
            id: "thread-1".to_string(),
            subject: "Test".to_string(),
            snippet: "Hello".to_string(),
            senders: vec!["test@iris.local".to_string()],
            latest_timestamp: Utc::now(),
            message_count: 1,
            is_unread: false,
            is_flagged: false,
            tags: vec![],
            messages: vec![msg],
        };

        // Archive thread
        let moves = archive_thread(&thread, &account_root).expect("archive success");
        assert_eq!(moves.len(), 1);
        assert!(!mail_file.exists());
        let archived_file = archive_cur.join("100.mail:2,S");
        assert!(archived_file.exists());

        // Restore thread (Undo action)
        restore_moved_files(&moves).expect("restore success");
        assert!(mail_file.exists());
        assert!(!archived_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_update_maildir_flags() {
        assert_eq!(update_maildir_flags("msg1:2,", Some('S'), None), "msg1:2,S");
        assert_eq!(update_maildir_flags("msg1:2,S", Some('F'), None), "msg1:2,FS");
        assert_eq!(update_maildir_flags("msg1:2,FS", None, Some('S')), "msg1:2,F");
        assert_eq!(update_maildir_flags("msg1:2,F", None, Some('F')), "msg1:2,");
    }
}
