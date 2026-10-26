// SPDX-License-Identifier: GPL-3.0-or-later

use crate::model::Thread;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchTerm {
    From(String),
    To(String),
    Subject(String),
    Body(String),
    IsUnread(bool),
    IsFlagged(bool),
    HasAttachment(bool),
    General(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub terms: Vec<SearchTerm>,
}

impl SearchQuery {
    /// Parses an input search string supporting fielded qualifiers and freeform keywords.
    /// Example: `from:alex is:unread subject:"audio visualizer" release`
    pub fn parse(input: &str) -> Self {
        let raw = input.trim();
        if raw.is_empty() {
            return Self { terms: Vec::new() };
        }

        let mut terms = Vec::new();
        let tokens = tokenize_query(raw);

        for token in tokens {
            if let Some(val) = token.strip_prefix("from:") {
                let clean = clean_val(val);
                if !clean.is_empty() {
                    terms.push(SearchTerm::From(clean.to_lowercase()));
                }
            } else if let Some(val) = token.strip_prefix("f:") {
                let clean = clean_val(val);
                if !clean.is_empty() {
                    terms.push(SearchTerm::From(clean.to_lowercase()));
                }
            } else if let Some(val) = token.strip_prefix("to:") {
                let clean = clean_val(val);
                if !clean.is_empty() {
                    terms.push(SearchTerm::To(clean.to_lowercase()));
                }
            } else if let Some(val) = token.strip_prefix("subject:") {
                let clean = clean_val(val);
                if !clean.is_empty() {
                    terms.push(SearchTerm::Subject(clean.to_lowercase()));
                }
            } else if let Some(val) = token.strip_prefix("s:") {
                let clean = clean_val(val);
                if !clean.is_empty() {
                    terms.push(SearchTerm::Subject(clean.to_lowercase()));
                }
            } else if let Some(val) = token.strip_prefix("body:") {
                let clean = clean_val(val);
                if !clean.is_empty() {
                    terms.push(SearchTerm::Body(clean.to_lowercase()));
                }
            } else if token == "is:unread" || token == "tag:unread" {
                terms.push(SearchTerm::IsUnread(true));
            } else if token == "is:read" || token == "tag:read" {
                terms.push(SearchTerm::IsUnread(false));
            } else if token == "is:flagged" || token == "is:starred" || token == "tag:flagged" || token == "tag:starred" {
                terms.push(SearchTerm::IsFlagged(true));
            } else if token == "is:unflagged" || token == "is:unstarred" {
                terms.push(SearchTerm::IsFlagged(false));
            } else if token == "has:attachment" || token == "has:attachments" || token == "tag:attachment" {
                terms.push(SearchTerm::HasAttachment(true));
            } else {
                let clean = clean_val(&token);
                if !clean.is_empty() {
                    terms.push(SearchTerm::General(clean.to_lowercase()));
                }
            }
        }

        Self { terms }
    }

    /// Tests if a Thread satisfies all query terms (AND logic).
    pub fn matches_thread(&self, thread: &Thread) -> bool {
        if self.terms.is_empty() {
            return true;
        }

        let subject_lower = thread.subject.to_lowercase();
        let snippet_lower = thread.snippet.to_lowercase();

        for term in &self.terms {
            match term {
                SearchTerm::From(val) => {
                    let matches_sender = thread
                        .senders
                        .iter()
                        .any(|s| s.to_lowercase().contains(val))
                        || thread
                            .messages
                            .iter()
                            .any(|m| m.from.to_lowercase().contains(val));
                    if !matches_sender {
                        return false;
                    }
                }
                SearchTerm::To(val) => {
                    let matches_to = thread
                        .messages
                        .iter()
                        .any(|m| m.to.iter().any(|t| t.to_lowercase().contains(val)));
                    if !matches_to {
                        return false;
                    }
                }
                SearchTerm::Subject(val) => {
                    if !subject_lower.contains(val) {
                        return false;
                    }
                }
                SearchTerm::Body(val) => {
                    let matches_body = snippet_lower.contains(val)
                        || thread.messages.iter().any(|m| {
                            m.body_plain
                                .as_ref()
                                .map(|b| b.to_lowercase().contains(val))
                                .unwrap_or(false)
                                || m.body_html
                                    .as_ref()
                                    .map(|b| b.to_lowercase().contains(val))
                                    .unwrap_or(false)
                        });
                    if !matches_body {
                        return false;
                    }
                }
                SearchTerm::IsUnread(expected) => {
                    if thread.is_unread != *expected {
                        return false;
                    }
                }
                SearchTerm::IsFlagged(expected) => {
                    if thread.is_flagged != *expected {
                        return false;
                    }
                }
                SearchTerm::HasAttachment(expected) => {
                    let has_att = thread
                        .messages
                        .iter()
                        .any(|m| !m.attachments.is_empty());
                    if has_att != *expected {
                        return false;
                    }
                }
                SearchTerm::General(val) => {
                    let matches_any = subject_lower.contains(val)
                        || snippet_lower.contains(val)
                        || thread.senders.iter().any(|s| s.to_lowercase().contains(val))
                        || thread.messages.iter().any(|m| {
                            m.from.to_lowercase().contains(val)
                                || m.body_plain
                                    .as_ref()
                                    .map(|b| b.to_lowercase().contains(val))
                                    .unwrap_or(false)
                        });
                    if !matches_any {
                        return false;
                    }
                }
            }
        }

        true
    }
}

fn clean_val(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'').trim().to_string()
}

/// Tokenizes a query string respecting quotes e.g. `subject:"audio visualizer"`
fn tokenize_query(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push('"');
            }
            ' ' | '\t' if !in_quotes => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::model::Message;

    fn make_test_thread(
        subject: &str,
        senders: &[&str],
        recipient: &str,
        snippet: &str,
        body: &str,
        is_unread: bool,
        is_flagged: bool,
        has_attachment: bool,
    ) -> Thread {
        let attachments = if has_attachment {
            vec![crate::model::AttachmentMeta {
                filename: "file.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                size_bytes: 1024,
                local_path: None,
            }]
        } else {
            Vec::new()
        };

        let msg = Message {
            id: "<test-msg-1@iris>".to_string(),
            thread_id: "<test-thread-1@iris>".to_string(),
            in_reply_to: None,
            references: Vec::new(),
            from: senders.first().unwrap_or(&"sender@test").to_string(),
            reply_to: None,
            to: vec![recipient.to_string()],
            cc: Vec::new(),
            date: Utc::now(),
            subject: subject.to_string(),
            body_plain: Some(body.to_string()),
            body_html: None,
            has_remote_images: false,
            attachments,
            is_unread,
            is_flagged,
            file_path: None,
        };

        Thread {
            id: "<test-thread-1@iris>".to_string(),
            subject: subject.to_string(),
            snippet: snippet.to_string(),
            senders: senders.iter().map(|s| s.to_string()).collect(),
            latest_timestamp: Utc::now(),
            message_count: 1,
            is_unread,
            is_flagged,
            tags: Vec::new(),
            messages: vec![msg],
        }
    }

    #[test]
    fn test_search_query_parsing() {
        let q = SearchQuery::parse(r#"from:alex to:bry subject:"audio visualizer" is:unread has:attachment release"#);
        assert_eq!(q.terms.len(), 6);
        assert_eq!(q.terms[0], SearchTerm::From("alex".to_string()));
        assert_eq!(q.terms[1], SearchTerm::To("bry".to_string()));
        assert_eq!(q.terms[2], SearchTerm::Subject("audio visualizer".to_string()));
        assert_eq!(q.terms[3], SearchTerm::IsUnread(true));
        assert_eq!(q.terms[4], SearchTerm::HasAttachment(true));
        assert_eq!(q.terms[5], SearchTerm::General("release".to_string()));
    }

    #[test]
    fn test_search_matching() {
        let thread = make_test_thread(
            "Hermes Audio Pipeline",
            &["Alex Rivera <alex@hermes.dev>"],
            "Brayan <bry@omarchy.org>",
            "32-band Goertzel filter bank",
            "Full implementation of audio visualizer",
            true,
            true,
            true,
        );

        // Matching queries
        assert!(SearchQuery::parse("from:alex").matches_thread(&thread));
        assert!(SearchQuery::parse("to:bry").matches_thread(&thread));
        assert!(SearchQuery::parse("subject:hermes").matches_thread(&thread));
        assert!(SearchQuery::parse("body:visualizer").matches_thread(&thread));
        assert!(SearchQuery::parse("is:unread").matches_thread(&thread));
        assert!(SearchQuery::parse("is:flagged").matches_thread(&thread));
        assert!(SearchQuery::parse("has:attachment").matches_thread(&thread));
        assert!(SearchQuery::parse("from:alex is:unread hermes").matches_thread(&thread));

        // Non-matching queries
        assert!(!SearchQuery::parse("from:carol").matches_thread(&thread));
        assert!(!SearchQuery::parse("to:charlie").matches_thread(&thread));
        assert!(!SearchQuery::parse("is:read").matches_thread(&thread));
        assert!(!SearchQuery::parse("is:unflagged").matches_thread(&thread));
        assert!(!SearchQuery::parse("subject:marketing").matches_thread(&thread));
    }

    #[test]
    fn test_search_shorthand_prefixes_and_empty() {
        let thread = make_test_thread(
            "Quick Fix",
            &["dev@iris.local"],
            "bry@omarchy.org",
            "Fixed search query parser",
            "All unit tests green",
            false,
            false,
            false,
        );

        // Empty query matches all
        assert!(SearchQuery::parse("").matches_thread(&thread));
        assert!(SearchQuery::parse("   ").matches_thread(&thread));

        // Shorthand prefixes
        assert!(SearchQuery::parse("f:dev").matches_thread(&thread));
        assert!(SearchQuery::parse("s:quick").matches_thread(&thread));
        assert!(SearchQuery::parse("is:read").matches_thread(&thread));
        assert!(SearchQuery::parse("is:unflagged").matches_thread(&thread));
    }
}
