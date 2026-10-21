// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;
use gtk::glib;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncResult {
    SyncedRemote(String),
    RefreshedLocal(usize),
    Error(String),
}

fn do_sync(mail_root: &Path) -> SyncResult {
    // 1. Check for custom sync script in ~/.config/iris/sync.sh
    if let Ok(home) = std::env::var("HOME") {
        let script = PathBuf::from(&home).join(".config/iris/sync.sh");
        if script.exists() {
            match Command::new("bash").arg(&script).output() {
                Ok(out) => {
                    let msg = String::from_utf8_lossy(&out.stdout).to_string();
                    if out.status.success() {
                        return SyncResult::SyncedRemote(format!("Synced via script: {}", msg.trim()));
                    } else {
                        let err = String::from_utf8_lossy(&out.stderr).to_string();
                        return SyncResult::Error(format!("Sync script error: {}", err.trim()));
                    }
                }
                Err(e) => {
                    return SyncResult::Error(format!("Failed to execute sync script: {e}"));
                }
            }
        }
    }

    // 2. Check for mbsync
    let has_mbsync = Command::new("which")
        .arg("mbsync")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_mbsync {
        match Command::new("mbsync").arg("-a").output() {
            Ok(out) => {
                if out.status.success() {
                    return SyncResult::SyncedRemote("Remote mailboxes synced via mbsync".to_string());
                } else {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    return SyncResult::Error(format!("mbsync: {}", err.trim()));
                }
            }
            Err(e) => {
                return SyncResult::Error(format!("Failed to run mbsync: {e}"));
            }
        }
    }

    // 3. Fallback: Local Maildir reconciliation
    let scanner = super::maildir::MaildirScanner::new(mail_root.to_path_buf());
    let count = scanner.scan_folder(&mail_root.join("demo/INBOX")).len()
        + scanner.scan_folder(&mail_root.join("work/INBOX")).len()
        + scanner.scan_folder(&mail_root.join("icloud/INBOX")).len();

    SyncResult::RefreshedLocal(count)
}

/// Runs mailbox synchronization in a background worker thread without blocking GTK.
/// The completion callback is executed on the GTK main thread and does not require `Send`.
pub fn run_sync_in_background<F>(mail_root: PathBuf, on_complete: F)
where
    F: FnOnce(SyncResult) + 'static,
{
    let (tx, rx) = mpsc::channel::<SyncResult>();

    std::thread::spawn(move || {
        let result = do_sync(&mail_root);
        let _ = tx.send(result);
    });

    let mut callback = Some(on_complete);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match rx.try_recv() {
            Ok(res) => {
                if let Some(cb) = callback.take() {
                    cb(res);
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(cb) = callback.take() {
                    cb(SyncResult::Error("Sync background worker disconnected".to_string()));
                }
                glib::ControlFlow::Break
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result_enum() {
        let res = SyncResult::RefreshedLocal(5);
        assert_eq!(res, SyncResult::RefreshedLocal(5));
        let remote = SyncResult::SyncedRemote("ok".to_string());
        assert_eq!(remote, SyncResult::SyncedRemote("ok".to_string()));
    }
}
