// SPDX-License-Identifier: GPL-3.0-or-later

/// Resolves an icon name from a prioritized list of candidates, returning the first
/// candidate available in the current GTK display icon theme, or the last fallback.
pub fn resolve_icon(candidates: &[&'static str]) -> &'static str {
    if candidates.is_empty() {
        return "";
    }
    if let Some(display) = gtk::gdk::Display::default() {
        let theme = gtk::IconTheme::for_display(&display);
        for &name in candidates {
            if theme.has_icon(name) {
                return name;
            }
        }
    }
    // Default fallback to last candidate
    candidates.last().copied().unwrap_or("")
}

/// Returns the best available symbolic icon name for the Inbox / All Inboxes.
pub fn inbox_icon() -> &'static str {
    resolve_icon(&[
        "mail-folder-inbox-symbolic",
        "mail-send-receive-symbolic",
        "mail-read-symbolic",
    ])
}

/// Returns the best available symbolic icon name for a specific email account,
/// detecting provider hints (iCloud, Gmail, Fastmail, Outlook).
pub fn account_icon(name: &str, email: &str) -> &'static str {
    let lower = format!("{} {}", name.to_lowercase(), email.to_lowercase());
    if lower.contains("icloud") || lower.contains("apple") || lower.contains("me.com") || lower.contains("mac.com") {
        resolve_icon(&[
            "weather-few-clouds-symbolic",
            "folder-remote-symbolic",
            "mail-read-symbolic",
        ])
    } else if lower.contains("gmail") || lower.contains("google") {
        resolve_icon(&[
            "google-symbolic",
            "mail-send-receive-symbolic",
            "mail-read-symbolic",
        ])
    } else if lower.contains("fastmail") {
        resolve_icon(&[
            "mail-send-symbolic",
            "mail-read-symbolic",
        ])
    } else if lower.contains("outlook") || lower.contains("office") || lower.contains("hotmail") || lower.contains("microsoft") {
        resolve_icon(&[
            "network-server-symbolic",
            "mail-read-symbolic",
        ])
    } else {
        resolve_icon(&[
            "mail-send-receive-symbolic",
            "mail-read-symbolic",
        ])
    }
}

/// Returns the best available symbolic icon name for the Archive mailbox / action.
pub fn archive_icon() -> &'static str {
    resolve_icon(&[
        "mail-archive-symbolic",
        "package-x-generic-symbolic",
        "folder-documents-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Feed & Newsletters.
pub fn feed_icon() -> &'static str {
    resolve_icon(&[
        "newspaper-symbolic",
        "application-rss+xml-symbolic",
        "user-bookmarks-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Drafts.
pub fn drafts_icon() -> &'static str {
    resolve_icon(&[
        "document-edit-symbolic",
        "mail-message-new-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Sent mail.
pub fn sent_icon() -> &'static str {
    resolve_icon(&[
        "mail-send-symbolic",
        "mail-sent-symbolic",
        "mail-message-new-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Flagged / Starred.
pub fn flagged_icon() -> &'static str {
    resolve_icon(&[
        "starred-symbolic",
        "mail-mark-important-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Trash.
pub fn trash_icon() -> &'static str {
    resolve_icon(&[
        "user-trash-symbolic",
    ])
}

/// Returns the best available symbolic icon name for custom mail folders.
pub fn folder_icon() -> &'static str {
    resolve_icon(&[
        "folder-symbolic",
        "inode-directory-symbolic",
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_icon_fallback() {
        let result = resolve_icon(&["nonexistent-one", "nonexistent-two", "final-fallback"]);
        assert_eq!(result, "final-fallback");
    }

    #[test]
    fn test_inbox_icon_not_empty() {
        let icon = inbox_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_account_icons() {
        assert!(!account_icon("Personal", "bry@gmail.com").is_empty());
        assert!(!account_icon("Apple iCloud", "bry@icloud.com").is_empty());
        assert!(!account_icon("Work", "bry@custom.com").is_empty());
    }

    #[test]
    fn test_archive_icon_not_empty() {
        let icon = archive_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_feed_icon_not_empty() {
        let icon = feed_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_sent_icon_not_empty() {
        let icon = sent_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_drafts_icon_not_empty() {
        let icon = drafts_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_flagged_icon_not_empty() {
        let icon = flagged_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_trash_icon_not_empty() {
        let icon = trash_icon();
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_folder_icon_not_empty() {
        let icon = folder_icon();
        assert!(!icon.is_empty());
    }
}
