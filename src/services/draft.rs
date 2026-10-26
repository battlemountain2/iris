// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;

use super::maildir::ensure_maildir_folder;

/// Durably saves an in-progress draft email to the target Maildir `Drafts` folder.
/// If an existing `draft_path` is provided, it updates that file in place.
/// Otherwise, it creates a new unique Maildir file with standard `:2,DS` flags (Draft + Seen).
/// If `to`, `subject`, and `body` are all empty and an existing file exists, it removes it.
pub fn save_draft(
    drafts_dir: &Path,
    draft_path: Option<&Path>,
    _account_id: &str,
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
    references: &[String],
) -> Result<Option<PathBuf>, String> {
    // If entire draft content is empty: clean up existing file if present and do not create new file
    if to.trim().is_empty() && subject.trim().is_empty() && body.trim().is_empty() {
        if let Some(path) = draft_path {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
        return Ok(None);
    }

    ensure_maildir_folder(drafts_dir)
        .map_err(|e| format!("Failed to create Drafts directory: {e}"))?;

    let cur_dir = drafts_dir.join("cur");

    // Construct standard RFC 822 Draft text
    let mut rfc822 = String::new();
    rfc822.push_str(&format!("From: {from}\r\n"));
    if !to.trim().is_empty() {
        rfc822.push_str(&format!("To: {to}\r\n"));
    }
    rfc822.push_str(&format!("Subject: {subject}\r\n"));
    rfc822.push_str(&format!("Date: {}\r\n", Utc::now().to_rfc2822()));

    let message_id = if let Some(path) = draft_path {
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("draft");
        format!("<{file_stem}@iris.local>")
    } else {
        format!("<draft-{}-{}@iris.local>", Utc::now().timestamp(), std::process::id())
    };
    rfc822.push_str(&format!("Message-ID: {message_id}\r\n"));

    if let Some(reply_id) = in_reply_to {
        let clean = reply_id.trim_matches(|c| c == '<' || c == '>');
        rfc822.push_str(&format!("In-Reply-To: <{clean}>\r\n"));
    }

    if !references.is_empty() {
        let refs_formatted: Vec<String> = references
            .iter()
            .map(|r| {
                let clean = r.trim_matches(|c| c == '<' || c == '>');
                format!("<{clean}>")
            })
            .collect();
        rfc822.push_str(&format!("References: {}\r\n", refs_formatted.join(" ")));
    }

    rfc822.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    rfc822.push_str("\r\n");
    rfc822.push_str(body);

    let target_path = if let Some(path) = draft_path {
        path.to_path_buf()
    } else {
        let now = Utc::now();
        let filename = format!(
            "{}.{}_{}.iris:2,DS",
            now.timestamp(),
            now.timestamp_subsec_micros(),
            std::process::id()
        );
        cur_dir.join(filename)
    };

    fs::write(&target_path, rfc822)
        .map_err(|e| format!("Failed to write draft to '{}': {e}", target_path.display()))?;

    Ok(Some(target_path))
}

/// Discards/deletes a saved draft file from disk.
pub fn delete_draft(draft_path: &Path) -> Result<(), String> {
    if draft_path.exists() {
        fs::remove_file(draft_path)
            .map_err(|e| format!("Failed to delete draft '{}': {e}", draft_path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_update_draft() {
        let temp_dir = std::env::temp_dir().join(format!("iris_draft_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let drafts_dir = temp_dir.join("Drafts");

        // 1. Save new draft
        let saved_path = save_draft(
            &drafts_dir,
            None,
            "demo",
            "bry@gmail.com",
            "alex@hermes.dev",
            "Initial Draft Subject",
            "Initial Draft Body",
            Some("<parent-msg@dev>"),
            &["<root-msg@dev>".to_string()],
        )
        .unwrap()
        .expect("Draft should be saved");

        assert!(saved_path.exists());
        assert!(saved_path.to_string_lossy().contains(":2,DS"));

        let content = fs::read_to_string(&saved_path).unwrap();
        assert!(content.contains("To: alex@hermes.dev"));
        assert!(content.contains("Subject: Initial Draft Subject"));
        assert!(content.contains("In-Reply-To: <parent-msg@dev>"));
        assert!(content.contains("Initial Draft Body"));

        // 2. Update existing draft
        let updated_path = save_draft(
            &drafts_dir,
            Some(&saved_path),
            "demo",
            "bry@gmail.com",
            "alex@hermes.dev",
            "Updated Draft Subject",
            "Updated Draft Body with more text",
            Some("<parent-msg@dev>"),
            &["<root-msg@dev>".to_string()],
        )
        .unwrap()
        .expect("Draft should be updated");

        assert_eq!(saved_path, updated_path);
        let updated_content = fs::read_to_string(&updated_path).unwrap();
        assert!(updated_content.contains("Subject: Updated Draft Subject"));
        assert!(updated_content.contains("Updated Draft Body with more text"));

        // 3. Delete draft
        delete_draft(&updated_path).unwrap();
        assert!(!updated_path.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_empty_draft_cleans_up_file() {
        let temp_dir = std::env::temp_dir().join(format!("iris_draft_empty_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let drafts_dir = temp_dir.join("Drafts");

        // Save initial
        let path = save_draft(
            &drafts_dir,
            None,
            "demo",
            "bry@gmail.com",
            "test@dev",
            "Some Subject",
            "Some Body",
            None,
            &[],
        )
        .unwrap()
        .unwrap();
        assert!(path.exists());

        // Saving empty should remove the file
        let empty_result = save_draft(
            &drafts_dir,
            Some(&path),
            "demo",
            "bry@gmail.com",
            "",
            "",
            "",
            None,
            &[],
        )
        .unwrap();

        assert!(empty_result.is_none());
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
