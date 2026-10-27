// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use crate::model::Thread;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Contact {
    pub display_name: String,
    pub email: String,
}

impl Contact {
    pub fn new(display_name: &str, email: &str) -> Self {
        Self {
            display_name: display_name.trim().to_string(),
            email: email.trim().to_lowercase(),
        }
    }

    pub fn to_address_string(&self) -> String {
        if self.display_name.is_empty() || self.display_name.eq_ignore_ascii_case(&self.email) {
            self.email.clone()
        } else {
            format!("{} <{}>", self.display_name, self.email)
        }
    }

    #[allow(dead_code)]
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        self.display_name.to_lowercase().contains(&q) || self.email.contains(&q)
    }
}

/// Parses an RFC 822 address string like `"Alex <alex@hermes.dev>"` or `"alex@hermes.dev"`.
pub fn parse_address(raw: &str) -> Option<Contact> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let (Some(start), Some(end)) = (trimmed.find('<'), trimmed.rfind('>')) {
        if start < end {
            let name = trimmed[..start].trim().trim_matches('"').trim();
            let email = trimmed[start + 1..end].trim();
            if email.contains('@') {
                return Some(Contact::new(name, email));
            }
        }
    }

    if trimmed.contains('@') {
        let clean = trimmed.trim_matches(|c| c == '<' || c == '>' || c == '"');
        return Some(Contact::new("", clean));
    }

    None
}

/// Extracts all unique contacts from a list of scanned threads (senders, from, to, cc).
pub fn extract_contacts_from_threads(threads: &[Thread]) -> Vec<Contact> {
    let mut set = HashSet::new();

    for t in threads {
        for s in &t.senders {
            if let Some(c) = parse_address(s) {
                set.insert(c);
            }
        }
        for m in &t.messages {
            if let Some(c) = parse_address(&m.from) {
                set.insert(c);
            }
            for to in &m.to {
                if let Some(c) = parse_address(to) {
                    set.insert(c);
                }
            }
            for cc in &m.cc {
                if let Some(c) = parse_address(cc) {
                    set.insert(c);
                }
            }
        }
    }

    let mut list: Vec<Contact> = set.into_iter().collect();
    list.sort_by(|a, b| {
        let name_a = if a.display_name.is_empty() { &a.email } else { &a.display_name };
        let name_b = if b.display_name.is_empty() { &b.email } else { &b.display_name };
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_with_name() {
        let c = parse_address("Alex Karas <alex@hermes.dev>").unwrap();
        assert_eq!(c.display_name, "Alex Karas");
        assert_eq!(c.email, "alex@hermes.dev");
        assert_eq!(c.to_address_string(), "Alex Karas <alex@hermes.dev>");
    }

    #[test]
    fn test_parse_address_plain_email() {
        let c = parse_address("support@github.com").unwrap();
        assert_eq!(c.display_name, "");
        assert_eq!(c.email, "support@github.com");
        assert_eq!(c.to_address_string(), "support@github.com");
    }

    #[test]
    fn test_parse_address_quoted_name() {
        let c = parse_address("\"Noctalia Core\" <core@noctalia.org>").unwrap();
        assert_eq!(c.display_name, "Noctalia Core");
        assert_eq!(c.email, "core@noctalia.org");
    }

    #[test]
    fn test_contact_matches_query() {
        let c = Contact::new("Alex Karas", "alex@hermes.dev");
        assert!(c.matches_query("alex"));
        assert!(c.matches_query("KARAS"));
        assert!(c.matches_query("hermes"));
        assert!(!c.matches_query("gmail"));
    }

    #[test]
    fn test_extract_contacts_from_threads() {
        use chrono::Utc;
        use crate::model::Message;

        let thread = Thread {
            id: "t1".into(),
            subject: "Team Sync".into(),
            snippet: "See you at 10".into(),
            senders: vec!["Elena Rox <elena@hermes.dev>".into()],
            latest_timestamp: Utc::now(),
            message_count: 1,
            is_unread: false,
            is_flagged: false,
            tags: vec![],
            messages: vec![Message {
                id: "m1".into(),
                thread_id: "t1".into(),
                subject: "Team Sync".into(),
                from: "Elena Rox <elena@hermes.dev>".into(),
                reply_to: None,
                to: vec!["Brayan <bry@omarchy.org>".into()],
                cc: vec!["Nikola <nikola@hermes.dev>".into()],
                date: Utc::now(),
                body_plain: Some("Hello".into()),
                body_html: None,
                in_reply_to: None,
                references: vec![],
                is_unread: false,
                is_flagged: false,
                attachments: vec![],
                has_remote_images: false,
                file_path: None,
            }],
        };

        let contacts = extract_contacts_from_threads(&[thread]);
        assert_eq!(contacts.len(), 3);
        let emails: Vec<&str> = contacts.iter().map(|c| c.email.as_str()).collect();
        assert!(emails.contains(&"elena@hermes.dev"));
        assert!(emails.contains(&"bry@omarchy.org"));
        assert!(emails.contains(&"nikola@hermes.dev"));
    }
}
