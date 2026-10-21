// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboxMessage {
    pub id: String,
    pub account_name: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub queued_at: chrono::DateTime<Utc>,
}

impl OutboxMessage {
    pub fn new(
        account_name: &str,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<String>,
        references: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        let id = format!("{}.{}.iris", now.timestamp_micros(), std::process::id());
        Self {
            id,
            account_name: account_name.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            in_reply_to,
            references,
            queued_at: now,
        }
    }

    /// Renders an RFC 822 compliant email string for saving to Maildir Sent folder.
    pub fn to_rfc822(&self) -> String {
        let date_str = self.queued_at.to_rfc2822();
        let mut headers = format!(
            "From: {}\r\n\
             To: {}\r\n\
             Subject: {}\r\n\
             Date: {}\r\n\
             Message-ID: <{}>\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=utf-8\r\n",
            self.from, self.to, self.subject, date_str, self.id
        );

        if let Some(ref reply_id) = self.in_reply_to {
            headers.push_str(&format!("In-Reply-To: <{reply_id}>\r\n"));
        }
        if !self.references.is_empty() {
            let refs = self
                .references
                .iter()
                .map(|r| format!("<{r}>"))
                .collect::<Vec<_>>()
                .join(" ");
            headers.push_str(&format!("References: {refs}\r\n"));
        }

        headers.push_str("\r\n");
        headers.push_str(&self.body);
        headers.push_str("\r\n");
        headers
    }
}

pub fn get_iris_data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("IRIS_DATA_DIR") {
        PathBuf::from(custom)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share/iris")
    } else {
        std::env::temp_dir().join("iris_data")
    }
}

pub fn get_outbox_dir() -> PathBuf {
    let dir = get_iris_data_dir().join("outbox");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_drafts_dir() -> PathBuf {
    let dir = get_iris_data_dir().join("drafts");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Durably stores an outbox message to disk in a specified directory before dispatch.
pub fn enqueue_outbox_message_in(outbox_dir: &Path, msg: &OutboxMessage) -> Result<PathBuf, String> {
    let _ = fs::create_dir_all(outbox_dir);
    let file_path = outbox_dir.join(format!("{}.json", msg.id));
    let json = serde_json::to_string_pretty(msg)
        .map_err(|e| format!("Failed to serialize outbox message: {e}"))?;
    fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write outbox message to disk: {e}"))?;
    Ok(file_path)
}

/// Durably stores an outbox message to default outbox directory before dispatch.
pub fn enqueue_outbox_message(msg: &OutboxMessage) -> Result<PathBuf, String> {
    enqueue_outbox_message_in(&get_outbox_dir(), msg)
}

/// Removes a message from the specified outbox directory.
pub fn remove_from_outbox_in(outbox_dir: &Path, id: &str) {
    let file_path = outbox_dir.join(format!("{id}.json"));
    let _ = fs::remove_file(file_path);
}

/// Removes a message from the default outbox directory (upon successful send or cancellation).
pub fn remove_from_outbox(id: &str) {
    remove_from_outbox_in(&get_outbox_dir(), id);
}

/// Preserves an outbox message into the specified drafts directory.
pub fn save_failed_to_drafts_in(
    drafts_dir: &Path,
    outbox_dir: &Path,
    msg: &OutboxMessage,
) -> Result<PathBuf, String> {
    let _ = fs::create_dir_all(drafts_dir);
    let file_path = drafts_dir.join(format!("{}.json", msg.id));
    let json = serde_json::to_string_pretty(msg)
        .map_err(|e| format!("Failed to serialize draft: {e}"))?;
    fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write draft to disk: {e}"))?;
    remove_from_outbox_in(outbox_dir, &msg.id);
    Ok(file_path)
}

/// Preserves an outbox message into the default drafts directory upon failure.
pub fn save_failed_to_drafts(msg: &OutboxMessage) -> Result<PathBuf, String> {
    save_failed_to_drafts_in(&get_drafts_dir(), &get_outbox_dir(), msg)
}

/// Recovers any leftover messages from the specified outbox directory into drafts.
pub fn recover_stale_outbox_to_drafts_in(outbox_dir: &Path, drafts_dir: &Path) -> Vec<OutboxMessage> {
    let Ok(entries) = fs::read_dir(outbox_dir) else {
        return Vec::new();
    };

    let mut recovered = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(msg) = serde_json::from_str::<OutboxMessage>(&content) {
                    if save_failed_to_drafts_in(drafts_dir, outbox_dir, &msg).is_ok() {
                        recovered.push(msg);
                    }
                }
            }
        }
    }
    recovered
}

/// Recovers any leftover messages in the default outbox directory into drafts.
/// Useful on startup if the application was unexpectedly terminated during an undo window.
pub fn recover_stale_outbox_to_drafts() -> Vec<OutboxMessage> {
    recover_stale_outbox_to_drafts_in(&get_outbox_dir(), &get_drafts_dir())
}

/// Saves the sent email directly into the account's Maildir `Sent/cur/` folder.
pub fn record_to_sent_maildir(
    mail_root: &Path,
    account: &str,
    msg: &OutboxMessage,
) -> Result<PathBuf, String> {
    let sent_cur = mail_root.join(account).join("Sent").join("cur");
    fs::create_dir_all(&sent_cur)
        .map_err(|e| format!("Failed to create Sent Maildir folder: {e}"))?;

    let filename = format!("{}:2,S", msg.id);
    let target_path = sent_cur.join(filename);
    let rfc822 = msg.to_rfc822();

    fs::write(&target_path, rfc822)
        .map_err(|e| format!("Failed to record sent message in Maildir: {e}"))?;

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outbox_message_creation_and_rfc822() {
        let msg = OutboxMessage::new(
            "demo",
            "bry@omarchy.org",
            "alex@hermes.dev",
            "Test Subject",
            "Hello Alex, this is a test.",
            Some("hermes-01@hermes.dev".to_string()),
            vec!["hermes-01@hermes.dev".to_string()],
        );

        let rfc822 = msg.to_rfc822();
        assert!(rfc822.contains("From: bry@omarchy.org"));
        assert!(rfc822.contains("To: alex@hermes.dev"));
        assert!(rfc822.contains("Subject: Test Subject"));
        assert!(rfc822.contains("In-Reply-To: <hermes-01@hermes.dev>"));
        assert!(rfc822.contains("Hello Alex, this is a test."));
    }

    #[test]
    fn test_record_to_sent_maildir() {
        let temp_dir = std::env::temp_dir().join(format!("iris_outbox_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let msg = OutboxMessage::new(
            "demo",
            "bry@omarchy.org",
            "alex@hermes.dev",
            "Sent Test",
            "Body of sent mail.",
            None,
            vec![],
        );

        let saved = record_to_sent_maildir(&temp_dir, "demo", &msg).unwrap();
        assert!(saved.exists());
        assert!(saved.to_string_lossy().contains("demo/Sent/cur/"));
        assert!(saved.to_string_lossy().ends_with(":2,S"));

        let content = fs::read_to_string(&saved).unwrap();
        assert!(content.contains("Subject: Sent Test"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_enqueue_remove_and_recover_outbox() {
        let temp_dir = std::env::temp_dir().join(format!("iris_outbox_queue_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let outbox_dir = temp_dir.join("outbox");
        let drafts_dir = temp_dir.join("drafts");

        let msg = OutboxMessage::new(
            "demo",
            "bry@omarchy.org",
            "alex@hermes.dev",
            "Queue Test",
            "Body for queue",
            None,
            vec![],
        );

        let queued_path = enqueue_outbox_message_in(&outbox_dir, &msg).unwrap();
        assert!(queued_path.exists());

        // Test recovery of interrupted/stale message into drafts
        let recovered = recover_stale_outbox_to_drafts_in(&outbox_dir, &drafts_dir);
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].id, msg.id);
        assert!(!queued_path.exists()); // was moved out of outbox

        let draft_path = drafts_dir.join(format!("{}.json", msg.id));
        assert!(draft_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
