// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Once;

static INIT_RESOURCES: Once = Once::new();

/// Ensures that the bundled Iris GResource icon assets are registered with GIO
/// and attached to the default display's GTK IconTheme.
pub fn register_bundled_icons() {
    INIT_RESOURCES.call_once(|| {
        let _ = gio::resources_register_include!("iris.gresource");
        if let Some(display) = gtk::gdk::Display::default() {
            let theme = gtk::IconTheme::for_display(&display);
            theme.add_resource_path("/io/github/battlemountain2/Iris/icons");
        }
    });
}

/// Resolves an icon name from a prioritized list of candidates, returning the first
/// candidate available in the current GTK display icon theme, or the last fallback.
pub fn resolve_icon(candidates: &[&'static str]) -> &'static str {
    if candidates.is_empty() {
        return "";
    }
    register_bundled_icons();
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
        "iris-inbox-symbolic",
        "mail-folder-inbox-symbolic",
        "mail-send-receive-symbolic",
        "mail-read-symbolic",
    ])
}

/// Returns the best available symbolic icon name for the unified All Inboxes mailbox.
pub fn all_inboxes_icon() -> &'static str {
    resolve_icon(&[
        "iris-all-inboxes-symbolic",
        "iris-inbox-symbolic",
        "mail-folder-inbox-symbolic",
        "mail-send-receive-symbolic",
    ])
}

/// Returns the best available symbolic icon name for a specific email account,
/// detecting provider hints (iCloud, Gmail, Fastmail, Outlook).
pub fn account_icon(name: &str, email: &str) -> &'static str {
    let lower = format!("{} {}", name.to_lowercase(), email.to_lowercase());
    if lower.contains("icloud") || lower.contains("apple") || lower.contains("me.com") || lower.contains("mac.com") {
        resolve_icon(&[
            "iris-provider-icloud-symbolic",
            "folder-remote-symbolic",
            "weather-few-clouds-symbolic",
            "mail-read-symbolic",
        ])
    } else if lower.contains("gmail") || lower.contains("google") {
        resolve_icon(&[
            "iris-provider-gmail-symbolic",
            "google-symbolic",
            "mail-send-receive-symbolic",
            "mail-read-symbolic",
        ])
    } else if lower.contains("fastmail") {
        resolve_icon(&[
            "iris-provider-fastmail-symbolic",
            "mail-send-symbolic",
            "mail-read-symbolic",
        ])
    } else if lower.contains("outlook") || lower.contains("office") || lower.contains("hotmail") || lower.contains("microsoft") {
        resolve_icon(&[
            "iris-provider-outlook-symbolic",
            "network-server-symbolic",
            "mail-read-symbolic",
        ])
    } else {
        resolve_icon(&[
            "iris-provider-custom-symbolic",
            "mail-send-receive-symbolic",
            "mail-read-symbolic",
        ])
    }
}

/// Returns the best available symbolic icon name for the Archive mailbox / action.
pub fn archive_icon() -> &'static str {
    resolve_icon(&[
        "iris-archive-symbolic",
        "mail-archive-symbolic",
        "package-x-generic-symbolic",
        "folder-documents-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Feed & Newsletters.
pub fn feed_icon() -> &'static str {
    resolve_icon(&[
        "iris-feed-symbolic",
        "newspaper-symbolic",
        "user-bookmarks-symbolic",
        "chat-message-new-symbolic",
        "application-rss+xml-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Drafts.
pub fn drafts_icon() -> &'static str {
    resolve_icon(&[
        "iris-drafts-symbolic",
        "document-edit-symbolic",
        "mail-message-new-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Sent mail.
pub fn sent_icon() -> &'static str {
    resolve_icon(&[
        "iris-sent-symbolic",
        "mail-send-symbolic",
        "mail-sent-symbolic",
        "mail-message-new-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Flagged / Starred.
pub fn flagged_icon() -> &'static str {
    resolve_icon(&[
        "iris-flag-symbolic",
        "starred-symbolic",
        "mail-mark-important-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Star / Favorite (outline).
pub fn star_icon() -> &'static str {
    resolve_icon(&[
        "iris-star-symbolic",
        "non-starred-symbolic",
        "starred-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Star / Favorite (filled).
pub fn star_filled_icon() -> &'static str {
    resolve_icon(&[
        "iris-star-filled-symbolic",
        "starred-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Trash.
pub fn trash_icon() -> &'static str {
    resolve_icon(&[
        "iris-trash-symbolic",
        "user-trash-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Junk / Spam.
#[allow(dead_code)]
pub fn junk_icon() -> &'static str {
    resolve_icon(&[
        "iris-junk-symbolic",
        "mail-mark-junk-symbolic",
    ])
}

/// Returns the best available symbolic icon name for custom mail folders.
pub fn folder_icon() -> &'static str {
    resolve_icon(&[
        "iris-folder-symbolic",
        "folder-symbolic",
        "inode-directory-symbolic",
    ])
}

/// Returns the best available symbolic icon name for open folders.
#[allow(dead_code)]
pub fn folder_open_icon() -> &'static str {
    resolve_icon(&[
        "iris-folder-open-symbolic",
        "folder-open-symbolic",
        "folder-symbolic",
    ])
}

/// Returns the best available symbolic icon name for creating a new folder.
pub fn folder_new_icon() -> &'static str {
    resolve_icon(&[
        "iris-folder-new-symbolic",
        "folder-new-symbolic",
        "folder-symbolic",
    ])
}

/// Returns the best available symbolic icon name for labels and tags.
pub fn tag_icon() -> &'static str {
    resolve_icon(&[
        "iris-tag-symbolic",
        "tag-symbolic",
        "user-bookmarks-symbolic",
        "bookmark-new-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Reply.
pub fn reply_icon() -> &'static str {
    resolve_icon(&[
        "iris-reply-symbolic",
        "mail-reply-sender-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Reply All.
pub fn reply_all_icon() -> &'static str {
    resolve_icon(&[
        "iris-reply-all-symbolic",
        "mail-reply-all-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Forward.
pub fn forward_icon() -> &'static str {
    resolve_icon(&[
        "iris-forward-symbolic",
        "mail-forward-symbolic",
    ])
}

/// Returns the best available symbolic icon name for Compose / New Message.
pub fn compose_icon() -> &'static str {
    resolve_icon(&[
        "iris-compose-symbolic",
        "mail-message-new-symbolic",
    ])
}

/// Returns the best available symbolic icon name for filter actions.
pub fn filter_icon() -> &'static str {
    resolve_icon(&[
        "iris-filter-symbolic",
        "view-list-symbolic",
    ])
}

/// Returns the best available symbolic icon name for refresh / sync.
pub fn refresh_icon() -> &'static str {
    resolve_icon(&[
        "iris-refresh-symbolic",
        "view-refresh-symbolic",
    ])
}

/// Returns the best available symbolic icon name for settings.
pub fn settings_icon() -> &'static str {
    resolve_icon(&[
        "iris-settings-symbolic",
        "emblem-system-symbolic",
        "preferences-system-symbolic",
    ])
}

/// Returns the best available symbolic icon name for attachments.
pub fn attachment_icon() -> &'static str {
    resolve_icon(&[
        "iris-attachment-symbolic",
        "mail-attachment-symbolic",
    ])
}

/// Returns the best available symbolic icon name for privacy protection.
pub fn shield_icon() -> &'static str {
    resolve_icon(&[
        "iris-shield-symbolic",
        "security-high-symbolic",
    ])
}

/// Returns the best available symbolic icon name for password entries.
pub fn password_icon() -> &'static str {
    resolve_icon(&[
        "iris-lock-symbolic",
        "dialog-password-symbolic",
        "changes-prevent-symbolic",
    ])
}

/// Returns the best available symbolic icon name for servers and network hosts.
pub fn server_icon() -> &'static str {
    resolve_icon(&[
        "iris-server-symbolic",
        "network-server-symbolic",
        "preferences-system-network-symbolic",
    ])
}

/// Returns the best available symbolic icon name for users and contacts.
pub fn user_icon() -> &'static str {
    resolve_icon(&[
        "iris-user-symbolic",
        "avatar-default-symbolic",
        "user-info-symbolic",
    ])
}

/// Returns the best available symbolic icon name for search bars and actions.
pub fn search_icon() -> &'static str {
    resolve_icon(&[
        "iris-search-symbolic",
        "system-search-symbolic",
        "edit-find-symbolic",
    ])
}

/// Returns the best available symbolic icon name for sidebar toggles.
pub fn sidebar_toggle_icon() -> &'static str {
    resolve_icon(&[
        "iris-sidebar-symbolic",
        "sidebar-show-symbolic",
    ])
}

/// Returns the best available symbolic icon name for keyboard shortcuts.
pub fn keyboard_icon() -> &'static str {
    resolve_icon(&[
        "iris-keyboard-symbolic",
        "preferences-desktop-keyboard-shortcuts-symbolic",
    ])
}

/// Returns the best available symbolic icon name for themes and palettes.
pub fn palette_icon() -> &'static str {
    resolve_icon(&[
        "iris-palette-symbolic",
        "applications-graphics-symbolic",
    ])
}

/// Returns the best available symbolic icon name for plus / add actions.
pub fn plus_icon() -> &'static str {
    resolve_icon(&[
        "iris-plus-symbolic",
        "list-add-symbolic",
    ])
}

/// Returns the best available symbolic icon name for chevron right navigation.
pub fn chevron_right_icon() -> &'static str {
    resolve_icon(&[
        "iris-chevron-right-symbolic",
        "go-next-symbolic",
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

    #[test]
    fn test_new_icons_not_empty() {
        assert!(!folder_new_icon().is_empty());
        assert!(!tag_icon().is_empty());
        assert!(!password_icon().is_empty());
        assert!(!server_icon().is_empty());
        assert!(!user_icon().is_empty());
        assert!(!search_icon().is_empty());
    }
}
