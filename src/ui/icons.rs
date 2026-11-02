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

/// Returns the best available symbolic icon name for the Archive mailbox / action.
pub fn archive_icon() -> &'static str {
    resolve_icon(&["mail-archive-symbolic", "package-x-generic-symbolic"])
}

/// Returns the best available symbolic icon name for Feed & Newsletters.
pub fn feed_icon() -> &'static str {
    resolve_icon(&["newspaper-symbolic", "application-rss+xml-symbolic"])
}

/// Returns the best available symbolic icon name for Sent mail.
pub fn sent_icon() -> &'static str {
    resolve_icon(&["mail-send-symbolic", "mail-sent-symbolic"])
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
}
