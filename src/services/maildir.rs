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
                        let (is_unread, is_flagged) = parse_maildir_flags(&file_path);

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
            // Email 1 (with real MIME Attachment for Hermes bridge)
            let email1 = "From: Alex Rivera <alex@hermes.dev>\r\n\
Reply-To: Alex Rivera <alex@hermes.dev>\r\n\
To: Brayan <bry@omarchy.org>\r\n\
Subject: Hermes v0.4.0 Release & Audio Overhaul\r\n\
Date: Mon, 19 Oct 2026 10:42:00 -0600\r\n\
Message-ID: <hermes-release-040@hermes.dev>\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"HERMES_MIME_BOUNDARY\"\r\n\
\r\n\
--HERMES_MIME_BOUNDARY\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hey Brayan,\r\n\
\r\n\
The Cairo vinyl disc hero and micro-segment visualizer look insane! Just pushed the branch to upstream. The 32-band Goertzel filter bank runs with zero memory allocations directly tapping the stereo F32LE 48 kHz PCM stream.\r\n\
\r\n\
Next up, we should ensure the Hermes <-> Iris attachment bridge works smoothly. I've attached the architecture specification document below.\r\n\
\r\n\
Best,\r\nAlex\r\n\
\r\n\
--HERMES_MIME_BOUNDARY\r\n\
Content-Type: text/plain; name=\"hermes_bridge_spec.txt\"\r\n\
Content-Disposition: attachment; filename=\"hermes_bridge_spec.txt\"\r\n\
Content-Transfer-Encoding: 7bit\r\n\
\r\n\
Hermes <-> Iris Integration Specification\r\n\
Version: 1.0.0\r\n\
- Standard downloads: ~/Downloads/Hermes\r\n\
- Hermes IPC launcher: ~/.local/bin/strata or ~/.local/bin/hermes\r\n\
- MIME types supported: audio/flac, audio/wav, text/plain, application/pdf\r\n\
--HERMES_MIME_BOUNDARY--\r\n";
            fs::write(inbox_new.join("1792464000.1.mail:2,"), email1)?;

            // Email 1B (Reply in the same conversation thread)
            let email1_reply = "From: Brayan <bry@omarchy.org>\r\n\
To: Alex Rivera <alex@hermes.dev>\r\n\
Subject: Re: Hermes v0.4.0 Release & Audio Overhaul\r\n\
Date: Mon, 19 Oct 2026 11:15:00 -0600\r\n\
Message-ID: <reply-hermes-040@omarchy.org>\r\n\
In-Reply-To: <hermes-release-040@hermes.dev>\r\n\
References: <hermes-release-040@hermes.dev>\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hey Alex,\r\n\
\r\n\
Reviewed the spec and wired up the attachment extractor. The save-to-Hermes pipeline writes directly to ~/Downloads/Hermes with collision protection.\r\n\
\r\n\
Testing it right now!\r\n";
            fs::write(inbox_new.join("1792464000.2.mail:2,"), email1_reply)?;

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

        // Seed Drafts if empty
        let drafts_dir = demo_root.join("Drafts");
        ensure_maildir_folder(&drafts_dir)?;
        let drafts_cur = drafts_dir.join("cur");
        let drafts_empty = fs::read_dir(&drafts_cur).map(|mut d| d.next().is_none()).unwrap_or(true);
        if drafts_empty {
            let email_draft = "From: Brayan <bry@omarchy.org>\r\n\
                To: Alex Rivera <alex@hermes.dev>\r\n\
                Subject: Draft: Audio Pipeline & Micro-Visualizer Ideas\r\n\
                Date: Mon, 19 Oct 2026 16:45:00 -0600\r\n\
                Message-ID: <draft-audio-pipeline@omarchy.org>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Hey Alex,\r\n\
                \r\n\
                Drafting some thoughts on the 32-band Goertzel filter bank and Wayland IPC socket bridges...\r\n";
            fs::write(drafts_cur.join("1792468000.draft.mail:2,DS"), email_draft)?;
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
                            let (is_unread, _) = parse_maildir_flags(&entry.path());
                            if is_unread {
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

/// Derives the account root directory dynamically from a message file path.
/// Given e.g. `/home/user/Mail/work/INBOX/cur/123:2,S`, resolves `/home/user/Mail/work`.
pub fn find_account_root_for_message(file_path: &Path) -> Option<PathBuf> {
    let cur_or_new = file_path.parent()?;
    let folder = cur_or_new.parent()?;
    let account = folder.parent()?;
    Some(account.to_path_buf())
}

/// Extracts the Maildir flags from a file path.
/// Flags are only valid after `:2,` in the file's basename.
/// Returns (is_unread, is_flagged).
pub fn parse_maildir_flags(path: &Path) -> (bool, bool) {
    let parent = path.parent();
    let is_new_folder = parent
        .and_then(|p| p.file_name())
        .map(|name| name == "new")
        .unwrap_or(false);

    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return (is_new_folder, false),
    };

    if let Some((_base, flags_part)) = file_name.split_once(":2,") {
        let is_seen = flags_part.contains('S');
        let is_flagged = flags_part.contains('F');
        let is_unread = is_new_folder || !is_seen;
        (is_unread, is_flagged)
    } else {
        (is_new_folder, false)
    }
}

/// Moves a message file to the target maildir folder (into its `cur` directory).
/// If a file with the destination name already exists, generates a collision-safe
/// unique filename preserving the Maildir flags so existing mail is never overwritten.
/// Returns `(original_path, destination_path)` on success.
/// Strips `mbsync` UID parameters (`,U=\d+`) from a Maildir filename when moving across folders.
/// This ensures `mbsync` does not attempt to map stale source folder UIDs into the destination folder,
/// preventing sync rollbacks, UID mismatches, and data corruption.
pub fn sanitize_filename_for_folder_move(file_name: &str) -> String {
    if let Some((base_and_params, flags)) = file_name.split_once(":2,") {
        let sanitized_base = strip_uid_param(base_and_params);
        format!("{sanitized_base}:2,{flags}")
    } else {
        strip_uid_param(file_name)
    }
}

fn strip_uid_param(s: &str) -> String {
    if let Some(pos) = s.find(",U=") {
        let after = &s[pos + 3..];
        let end_idx = after
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| pos + 3 + i)
            .unwrap_or(s.len());
        let mut result = s[..pos].to_string();
        result.push_str(&s[end_idx..]);
        result
    } else {
        s.to_string()
    }
}

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

    let raw_file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid file name".to_string())?;

    let file_name = sanitize_filename_for_folder_move(raw_file_name);

    let mut dest_path = cur_dir.join(&file_name);
    if file_path == dest_path {
        return Ok((file_path.to_path_buf(), dest_path));
    }

    // Collision Protection:
    // If dest_path already exists and is a different file, generate a new unique Maildir filename
    // preserving the Maildir flags after `:2,` so no message is destroyed.
    if dest_path.exists() {
        let flags = if let Some((_, f)) = file_name.split_once(":2,") {
            format!(":2,{f}")
        } else {
            ":2,".to_string()
        };
        let now = Utc::now();
        let timestamp = now.timestamp();
        let micros = now.timestamp_subsec_micros();
        let pid = std::process::id();
        let mut counter = 0;
        loop {
            let unique_name = format!("{timestamp}.{micros}_{pid}_{counter}.iris{flags}");
            let candidate = cur_dir.join(&unique_name);
            if !candidate.exists() {
                dest_path = candidate;
                break;
            }
            counter += 1;
        }
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

/// Moves all messages in a thread to the `Archive` folder of their respective accounts.
/// Dynamically resolves the source account per message; falls back to `default_account_root`.
/// Returns a list of `(source_path, dest_path)` pairs that can be reverted by `restore_moved_files`.
pub fn archive_thread(
    thread: &Thread,
    default_account_root: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut moves = Vec::new();

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let src = PathBuf::from(path_str);
            let account_root = find_account_root_for_message(&src)
                .unwrap_or_else(|| default_account_root.to_path_buf());
            let archive_folder = account_root.join("Archive");
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

/// Moves all messages in a thread to the `Trash` folder of their respective accounts.
/// Dynamically resolves the source account per message; falls back to `default_account_root`.
/// Returns a list of `(source_path, dest_path)` pairs that can be reverted by `restore_moved_files`.
pub fn trash_thread(
    thread: &Thread,
    default_account_root: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut moves = Vec::new();

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let src = PathBuf::from(path_str);
            let account_root = find_account_root_for_message(&src)
                .unwrap_or_else(|| default_account_root.to_path_buf());
            let trash_folder = account_root.join("Trash");
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

/// Moves all messages in a thread to a specified destination folder (e.g. "Work", "Receipts", "Archive").
/// Creates the target folder if it does not yet exist.
/// Returns a list of `(source_path, dest_path)` pairs that can be reverted by `restore_moved_files`.
pub fn move_thread_to_folder(
    thread: &Thread,
    target_folder_name: &str,
    default_account_root: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut moves = Vec::new();

    for msg in &thread.messages {
        if let Some(ref path_str) = msg.file_path {
            let src = PathBuf::from(path_str);
            let account_root = find_account_root_for_message(&src)
                .unwrap_or_else(|| default_account_root.to_path_buf());
            let target_folder = account_root.join(target_folder_name);
            match move_message_to_folder(&src, &target_folder) {
                Ok(moved) => moves.push(moved),
                Err(err) => {
                    let _ = restore_moved_files(&moves);
                    return Err(format!("Move error: {err}"));
                }
            }
        }
    }

    Ok(moves)
}

/// Permanently unlinks/deletes all files in the given Trash folder (`cur`, `new`, and `tmp`).
/// Returns the number of permanently deleted message files.
pub fn empty_trash_folder(trash_folder: &Path) -> std::io::Result<usize> {
    let mut deleted_count = 0;
    for subdir in &["cur", "new", "tmp"] {
        let p = trash_folder.join(subdir);
        if let Ok(entries) = fs::read_dir(&p) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.is_file() {
                    if fs::remove_file(&file_path).is_ok() {
                        deleted_count += 1;
                    }
                }
            }
        }
    }
    Ok(deleted_count)
}

/// Discovers all available Maildir folders in an account directory, combining standard
/// mailboxes with any custom folders discovered on disk.
pub fn discover_folders_for_account(account_root: &Path) -> Vec<String> {
    let mut folders = Vec::new();
    let standard = ["INBOX", "Archive", "Trash", "Sent", "Drafts"];
    for std_f in standard {
        folders.push(std_f.to_string());
    }

    if let Ok(entries) = fs::read_dir(account_root) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') && !standard.iter().any(|&s| s.eq_ignore_ascii_case(&name)) {
                        folders.push(name);
                    }
                }
            }
        }
    }

    folders.sort();
    folders.dedup();
    folders
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

    let reply_to = parsed.reply_to().and_then(|addrs| {
        addrs.iter().next().and_then(|a| {
            if let Some(name) = a.name() {
                let addr = a.address().unwrap_or_default();
                Some(format!("{name} <{addr}>"))
            } else {
                a.address().map(|a| a.to_string())
            }
        })
    });

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

    let in_reply_to = parsed.in_reply_to().as_text().map(String::from);
    let references = parsed
        .references()
        .as_text_list()
        .map(|list| list.into_iter().map(String::from).collect())
        .unwrap_or_default();

    Some(Message {
        id: id.clone(),
        thread_id: id,
        in_reply_to,
        references,
        from,
        reply_to,
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

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self { parent: (0..size).collect() }
    }
    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let p = self.parent[i];
            self.parent[i] = self.find(p);
            self.parent[i]
        }
    }
    fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}

fn extract_email_address(raw: &str) -> String {
    if let Some(start) = raw.find('<') {
        if let Some(end) = raw[start..].find('>') {
            return raw[start + 1..start + end].trim().to_lowercase();
        }
    }
    raw.trim().to_lowercase()
}

fn message_all_addresses(msg: &Message) -> Vec<String> {
    let mut addrs = Vec::new();
    addrs.push(extract_email_address(&msg.from));
    for to in &msg.to {
        addrs.push(extract_email_address(to));
    }
    for cc in &msg.cc {
        addrs.push(extract_email_address(cc));
    }
    addrs
}

fn group_into_threads(mut messages: Vec<Message>) -> Vec<Thread> {
    if messages.is_empty() {
        return Vec::new();
    }

    messages.sort_by(|a, b| b.date.cmp(&a.date));
    let n = messages.len();
    let mut dsu = DisjointSet::new(n);

    // Map Message-ID -> index
    let mut id_to_index: HashMap<String, usize> = HashMap::new();
    for (i, msg) in messages.iter().enumerate() {
        if !msg.id.is_empty() {
            id_to_index.insert(msg.id.clone(), i);
        }
    }

    // 1. Thread by explicit RFC 822 In-Reply-To and References headers
    for (i, msg) in messages.iter().enumerate() {
        if let Some(ref reply_to) = msg.in_reply_to {
            let clean = reply_to.trim().trim_matches(['<', '>']);
            for (known_id, &target_idx) in &id_to_index {
                let clean_known = known_id.trim().trim_matches(['<', '>']);
                if clean == clean_known {
                    dsu.union(i, target_idx);
                    break;
                }
            }
        }
        for ref_id in &msg.references {
            let clean = ref_id.trim().trim_matches(['<', '>']);
            for (known_id, &target_idx) in &id_to_index {
                let clean_known = known_id.trim().trim_matches(['<', '>']);
                if clean == clean_known {
                    dsu.union(i, target_idx);
                    break;
                }
            }
        }
    }

    // 2. Thread by matching normalized subject + participant counterparty
    // Messages without explicit header links only join if they share the exact
    // normalized subject AND at least one email address counterparty.
    for i in 0..n {
        for j in (i + 1)..n {
            if dsu.find(i) == dsu.find(j) {
                continue;
            }
            let sub_i = normalize_subject(&messages[i].subject);
            let sub_j = normalize_subject(&messages[j].subject);
            if !sub_i.is_empty() && sub_i == sub_j {
                let from_i = extract_email_address(&messages[i].from);
                let from_j = extract_email_address(&messages[j].from);
                let participants_i = message_all_addresses(&messages[i]);
                let participants_j = message_all_addresses(&messages[j]);

                let shares_counterparty = from_i == from_j
                    || participants_j.contains(&from_i)
                    || participants_i.contains(&from_j);

                if shares_counterparty {
                    dsu.union(i, j);
                }
            }
        }
    }

    // Collect into thread groups
    let mut groups: HashMap<usize, Vec<Message>> = HashMap::new();
    for (i, msg) in messages.into_iter().enumerate() {
        let root = dsu.find(i);
        groups.entry(root).or_default().push(msg);
    }

    let mut threads = Vec::new();
    for (_root, mut msgs) in groups {
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
            in_reply_to: None,
            references: vec![],
            from: "test@iris.local".to_string(),
            reply_to: None,
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

    #[test]
    fn test_parse_maildir_flags_strict() {
        // Cur without S is unread, with F is flagged (despite 'F' in FastmailAccount and 'S' in Server1)
        let path_cur_unread_flagged = Path::new("/home/bry/Mail/FastmailAccount/INBOX/cur/1792464000.Server1.mail:2,F");
        let (unread, flagged) = parse_maildir_flags(path_cur_unread_flagged);
        assert!(unread, "Without S flag, message is unread");
        assert!(flagged, "F flag after :2, makes it flagged");

        // Cur with S is read, without F is not flagged
        let path_cur_read_not_flagged = Path::new("/home/bry/Mail/FastmailAccount/INBOX/cur/1792464000.Server1.mail:2,S");
        let (unread, flagged) = parse_maildir_flags(path_cur_read_not_flagged);
        assert!(!unread);
        assert!(!flagged);

        // Cur with FS is read and flagged
        let path_cur_read_flagged = Path::new("/home/bry/Mail/FastmailAccount/INBOX/cur/1792464000.Server1.mail:2,FS");
        let (unread, flagged) = parse_maildir_flags(path_cur_read_flagged);
        assert!(!unread);
        assert!(flagged);

        // New folder message is always unread
        let path_new = Path::new("/home/bry/Mail/demo/INBOX/new/1792464000.mail:2,S");
        let (unread, _) = parse_maildir_flags(path_new);
        assert!(unread);
    }

    #[test]
    fn test_collision_safe_move() {
        let temp_dir = std::env::temp_dir().join(format!("iris_collision_test_{}", std::process::id()));
        let inbox_cur = temp_dir.join("INBOX").join("cur");
        let archive_cur = temp_dir.join("Archive").join("cur");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&inbox_cur).unwrap();
        fs::create_dir_all(&archive_cur).unwrap();

        // Existing file in destination
        let dest_existing = archive_cur.join("colliding.mail:2,S");
        fs::write(&dest_existing, b"Existing message content in archive").unwrap();

        // Source file with identical name
        let src_file = inbox_cur.join("colliding.mail:2,S");
        fs::write(&src_file, b"New message content moving from inbox").unwrap();

        // Move to folder - must NOT overwrite
        let (_orig, moved_dest) = move_message_to_folder(&src_file, &temp_dir.join("Archive")).unwrap();
        assert!(moved_dest.exists());
        assert_ne!(moved_dest, dest_existing, "Destination must be assigned a unique name on collision");

        // Existing file must retain original content
        let existing_content = fs::read(&dest_existing).unwrap();
        assert_eq!(existing_content, b"Existing message content in archive");

        // Moved file must retain its content
        let moved_content = fs::read(&moved_dest).unwrap();
        assert_eq!(moved_content, b"New message content moving from inbox");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_thread_grouping_unrelated_senders_remain_separate() {
        let msg_apple = Message {
            id: "msg-apple-1".to_string(),
            thread_id: "msg-apple-1".to_string(),
            in_reply_to: None,
            references: vec![],
            from: "Apple Support <support@apple.com>".to_string(),
            reply_to: None,
            to: vec!["brayan@omarchy.org".to_string()],
            cc: vec![],
            date: Utc::now(),
            subject: "Your Monthly Receipt".to_string(),
            body_plain: Some("iCloud $9.99".to_string()),
            body_html: None,
            has_remote_images: false,
            attachments: vec![],
            is_unread: true,
            is_flagged: false,
            file_path: None,
        };

        let msg_do = Message {
            id: "msg-do-1".to_string(),
            thread_id: "msg-do-1".to_string(),
            in_reply_to: None,
            references: vec![],
            from: "DigitalOcean Billing <billing@digitalocean.com>".to_string(),
            reply_to: None,
            to: vec!["brayan@omarchy.org".to_string()],
            cc: vec![],
            date: Utc::now(),
            subject: "Your Monthly Receipt".to_string(),
            body_plain: Some("Droplet $12.00".to_string()),
            body_html: None,
            has_remote_images: false,
            attachments: vec![],
            is_unread: true,
            is_flagged: false,
            file_path: None,
        };

        // Group into threads
        let threads = group_into_threads(vec![msg_apple, msg_do]);
        assert_eq!(threads.len(), 2, "Unrelated senders with identical subjects must NOT merge into one conversation");
    }

    #[test]
    fn test_select_second_conversation_archive_and_undo() {
        let temp_dir = std::env::temp_dir().join(format!("iris_select_test_{}", std::process::id()));
        let account_root = temp_dir.join("demo");
        let inbox_cur = account_root.join("INBOX").join("cur");
        let archive_cur = account_root.join("Archive").join("cur");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&inbox_cur).unwrap();
        fs::create_dir_all(&archive_cur).unwrap();

        // Create 2 distinct conversations on disk
        let file1 = inbox_cur.join("1001.first.mail:2,S");
        let file2 = inbox_cur.join("1002.second.mail:2,S");
        fs::write(&file1, b"From: Alice <alice@example.com>\r\nSubject: First Project\r\n\r\nHello Alice").unwrap();
        fs::write(&file2, b"From: Bob <bob@example.com>\r\nSubject: Second Project\r\n\r\nHello Bob").unwrap();

        let scanner = MaildirScanner::new(temp_dir.clone());
        let mut threads = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(threads.len(), 2);

        // Sort so we know which is second
        threads.sort_by(|a, b| a.subject.cmp(&b.subject));
        let first_thread = &threads[0];
        let second_thread = &threads[1];
        assert_eq!(first_thread.subject, "First Project");
        assert_eq!(second_thread.subject, "Second Project");

        // Target specifically the second conversation for archive
        let moves = archive_thread(second_thread, &account_root).expect("archive success");
        assert_eq!(moves.len(), 1);

        // Verify: First conversation file in INBOX is COMPLETELY UNTOUCHED
        assert!(file1.exists(), "First conversation must remain in INBOX");
        let first_content = fs::read(&file1).unwrap();
        assert!(first_content.starts_with(b"From: Alice"));

        // Verify: Second conversation moved to Archive
        assert!(!file2.exists(), "Second conversation file must have moved out of INBOX");
        let archived_file = &moves[0].1;
        assert!(archived_file.exists());
        let archived_content = fs::read(archived_file).unwrap();
        assert!(archived_content.starts_with(b"From: Bob"));

        // Verify: Undo restores ONLY the second conversation
        restore_moved_files(&moves).expect("undo success");
        assert!(file1.exists(), "First conversation still intact");
        assert!(file2.exists(), "Second conversation restored to original path");
        assert!(!archived_file.exists(), "Archive destination cleaned up");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_multi_message_thread_grouping_and_attachments() {
        let temp_dir = std::env::temp_dir().join(format!("iris_multimsg_test_{}", std::process::id()));
        let account_root = temp_dir.join("demo");
        let inbox_new = account_root.join("INBOX").join("new");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&inbox_new).unwrap();

        // Message 1 with attachment
        let email1 = "From: Alex Rivera <alex@hermes.dev>\r\n\
To: Brayan <bry@omarchy.org>\r\n\
Subject: Audio Overhaul\r\n\
Date: Mon, 19 Oct 2026 10:42:00 -0600\r\n\
Message-ID: <msg-audio-01@hermes.dev>\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hey Brayan, here is the spec.\r\n\
--BOUNDARY\r\n\
Content-Type: text/plain; name=\"spec.txt\"\r\n\
Content-Disposition: attachment; filename=\"spec.txt\"\r\n\
\r\n\
Attachment contents inside message 1\r\n\
--BOUNDARY--\r\n";
        fs::write(inbox_new.join("101.mail:2,"), email1).unwrap();

        // Message 2: Reply in same thread
        let email2 = "From: Brayan <bry@omarchy.org>\r\n\
To: Alex Rivera <alex@hermes.dev>\r\n\
Subject: Re: Audio Overhaul\r\n\
Date: Mon, 19 Oct 2026 11:15:00 -0600\r\n\
Message-ID: <msg-audio-02@omarchy.org>\r\n\
In-Reply-To: <msg-audio-01@hermes.dev>\r\n\
References: <msg-audio-01@hermes.dev>\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Got it, testing now!\r\n";
        fs::write(inbox_new.join("102.mail:2,"), email2).unwrap();

        let scanner = MaildirScanner::new(temp_dir.clone());
        let threads = scanner.scan_folder(&account_root.join("INBOX"));

        assert_eq!(threads.len(), 1, "Messages with In-Reply-To must group into 1 thread");
        let thread = &threads[0];
        assert_eq!(thread.messages.len(), 2, "Thread must contain both messages");

        let initial_msg = thread.messages.iter().find(|m| m.id == "msg-audio-01@hermes.dev").unwrap();
        assert_eq!(initial_msg.attachments.len(), 1, "Initial message must have 1 attachment");
        assert_eq!(initial_msg.attachments[0].filename, "spec.txt");

        let reply_msg = thread.messages.iter().find(|m| m.id == "msg-audio-02@omarchy.org").unwrap();
        assert_eq!(reply_msg.attachments.len(), 0, "Reply message has 0 attachments");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sanitize_filename_for_folder_move_strips_mbsync_uid() {
        // Standard mbsync filename with UID
        let orig = "1697812345.12345_1.myhost,U=42:2,S";
        let sanitized = sanitize_filename_for_folder_move(orig);
        assert_eq!(sanitized, "1697812345.12345_1.myhost:2,S");

        // With multiple parameters before flags
        let orig_multi = "1697812345.12345_1.myhost,S=1234,U=999:2,FS";
        let sanitized_multi = sanitize_filename_for_folder_move(orig_multi);
        assert_eq!(sanitized_multi, "1697812345.12345_1.myhost,S=1234:2,FS");

        // Filename without UID remains intact
        let without_uid = "1697812345.12345_1.myhost:2,S";
        let sanitized_no_uid = sanitize_filename_for_folder_move(without_uid);
        assert_eq!(sanitized_no_uid, without_uid);
    }

    #[test]
    fn test_batch_archive_multiple_threads_and_undo() {
        let temp_dir = std::env::temp_dir().join(format!("iris_batch_arch_test_{}", std::process::id()));
        let account_root = temp_dir.join("demo");
        let inbox_new = account_root.join("INBOX").join("new");
        let archive_cur = account_root.join("Archive").join("cur");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&inbox_new).unwrap();
        fs::create_dir_all(&archive_cur).unwrap();

        let email1 = "From: Alice <alice@example.com>\r\nSubject: Thread 1\r\nDate: Mon, 20 Oct 2026 10:00:00 -0600\r\nMessage-ID: <t1@example.com>\r\nContent-Type: text/plain\r\n\r\nMsg 1";
        let email2 = "From: Bob <bob@example.com>\r\nSubject: Thread 2\r\nDate: Mon, 20 Oct 2026 11:00:00 -0600\r\nMessage-ID: <t2@example.com>\r\nContent-Type: text/plain\r\n\r\nMsg 2";
        let email3 = "From: Charlie <charlie@example.com>\r\nSubject: Thread 3\r\nDate: Mon, 20 Oct 2026 12:00:00 -0600\r\nMessage-ID: <t3@example.com>\r\nContent-Type: text/plain\r\n\r\nMsg 3";

        let f1 = inbox_new.join("101.mail:2,");
        let f2 = inbox_new.join("102.mail:2,");
        let f3 = inbox_new.join("103.mail:2,");
        fs::write(&f1, email1).unwrap();
        fs::write(&f2, email2).unwrap();
        fs::write(&f3, email3).unwrap();

        let scanner = MaildirScanner::new(temp_dir.clone());
        let threads = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(threads.len(), 3);

        // Batch archive threads 1 and 2
        let selected = vec![threads[0].clone(), threads[1].clone()];
        let mut all_moves = Vec::new();
        for t in &selected {
            let moves = archive_thread(t, &account_root).expect("archive success");
            all_moves.extend(moves);
        }

        assert_eq!(all_moves.len(), 2);
        let rem_threads = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(rem_threads.len(), 1);
        assert_eq!(rem_threads[0].subject, "Thread 1");

        let arch_threads = scanner.scan_folder(&account_root.join("Archive"));
        assert_eq!(arch_threads.len(), 2);

        // Reversible Undo
        restore_moved_files(&all_moves).expect("undo success");
        let restored_threads = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(restored_threads.len(), 3);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_move_thread_to_custom_folder_and_undo() {
        let temp_dir = std::env::temp_dir().join(format!("iris_custom_move_test_{}", std::process::id()));
        let account_root = temp_dir.join("demo");
        let inbox_new = account_root.join("INBOX").join("new");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&inbox_new).unwrap();

        let email = "From: Boss <boss@work.com>\r\nSubject: Project Alpha\r\nDate: Mon, 20 Oct 2026 10:00:00 -0600\r\nMessage-ID: <alpha@work.com>\r\nContent-Type: text/plain\r\n\r\nReview";
        let f1 = inbox_new.join("201.mail:2,");
        fs::write(&f1, email).unwrap();

        let scanner = MaildirScanner::new(temp_dir.clone());
        let threads = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(threads.len(), 1);

        // Move to custom folder "Projects"
        let moves = move_thread_to_folder(&threads[0], "Projects", &account_root).expect("move success");
        assert_eq!(moves.len(), 1);

        let inbox_after = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(inbox_after.len(), 0);

        let projects_threads = scanner.scan_folder(&account_root.join("Projects"));
        assert_eq!(projects_threads.len(), 1);
        assert_eq!(projects_threads[0].subject, "Project Alpha");

        // Undo move
        restore_moved_files(&moves).expect("undo success");
        let inbox_restored = scanner.scan_folder(&account_root.join("INBOX"));
        assert_eq!(inbox_restored.len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_empty_trash_folder_and_discovery() {
        let temp_dir = std::env::temp_dir().join(format!("iris_empty_trash_test_{}", std::process::id()));
        let account_root = temp_dir.join("demo");
        let trash_cur = account_root.join("Trash").join("cur");
        let trash_new = account_root.join("Trash").join("new");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&trash_cur).unwrap();
        fs::create_dir_all(&trash_new).unwrap();

        fs::write(trash_cur.join("msg1:2,S"), "Trash 1").unwrap();
        fs::write(trash_new.join("msg2:2,"), "Trash 2").unwrap();

        // Check discovery includes standard folders
        let folders = discover_folders_for_account(&account_root);
        assert!(folders.contains(&"Trash".to_string()));
        assert!(folders.contains(&"INBOX".to_string()));

        // Empty trash
        let deleted = empty_trash_folder(&account_root.join("Trash")).expect("empty success");
        assert_eq!(deleted, 2);

        // Verify files are unlinked
        let cur_entries = fs::read_dir(&trash_cur).unwrap().count();
        let new_entries = fs::read_dir(&trash_new).unwrap().count();
        assert_eq!(cur_entries, 0);
        assert_eq!(new_entries, 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
