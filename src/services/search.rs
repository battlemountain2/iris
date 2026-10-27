// SPDX-License-Identifier: GPL-3.0-or-later

use crate::model::Thread;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchTerm {
    From(String),
    To(String),
    Cc(String),
    Subject(String),
    Body(String),
    Attachment(String),
    Tag(String),
    IsUnread(bool),
    IsFlagged(bool),
    HasAttachment(bool),
    General(String),
    Not(Box<SearchTerm>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub terms: Vec<SearchTerm>,
}

impl SearchQuery {
    /// Parses an input search string supporting fielded qualifiers, wildcards, negation, and freeform keywords.
    /// Example: `from:alex is:unread subject:"audio visualizer" attach:*.pdf -tag:spam release`
    pub fn parse(input: &str) -> Self {
        let raw = input.trim();
        if raw.is_empty() {
            return Self { terms: Vec::new() };
        }

        let mut terms = Vec::new();
        let tokens = tokenize_query(raw);

        for token in tokens {
            if let Some(term) = parse_token(&token) {
                terms.push(term);
            }
        }

        Self { terms }
    }

    /// Tests if a Thread satisfies all query terms (AND logic).
    pub fn matches_thread(&self, thread: &Thread) -> bool {
        if self.terms.is_empty() {
            return true;
        }

        self.terms.iter().all(|term| matches_single_term(term, thread))
    }
}

fn parse_token(token: &str) -> Option<SearchTerm> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }

    // Negation prefix: `-from:alex` or `not:is:unread`
    if let Some(inner) = t.strip_prefix('-') {
        if !inner.is_empty() {
            return parse_token(inner).map(|term| SearchTerm::Not(Box::new(term)));
        }
    } else if let Some(inner) = t.strip_prefix("not:") {
        if !inner.is_empty() {
            return parse_token(inner).map(|term| SearchTerm::Not(Box::new(term)));
        }
    }

    if let Some(val) = t.strip_prefix("from:").or_else(|| t.strip_prefix("f:")) {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::From(clean.to_lowercase()));
        }
    } else if let Some(val) = t.strip_prefix("to:") {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::To(clean.to_lowercase()));
        }
    } else if let Some(val) = t.strip_prefix("cc:") {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::Cc(clean.to_lowercase()));
        }
    } else if let Some(val) = t.strip_prefix("subject:").or_else(|| t.strip_prefix("s:")) {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::Subject(clean.to_lowercase()));
        }
    } else if let Some(val) = t.strip_prefix("body:").or_else(|| t.strip_prefix("b:")) {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::Body(clean.to_lowercase()));
        }
    } else if let Some(val) = t
        .strip_prefix("attach:")
        .or_else(|| t.strip_prefix("attachment:"))
        .or_else(|| t.strip_prefix("filename:"))
        .or_else(|| t.strip_prefix("file:"))
    {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::Attachment(clean.to_lowercase()));
        }
    } else if t == "is:unread" || t == "tag:unread" {
        return Some(SearchTerm::IsUnread(true));
    } else if t == "is:read" || t == "tag:read" {
        return Some(SearchTerm::IsUnread(false));
    } else if t == "is:flagged" || t == "is:starred" || t == "tag:flagged" || t == "tag:starred" {
        return Some(SearchTerm::IsFlagged(true));
    } else if t == "is:unflagged" || t == "is:unstarred" {
        return Some(SearchTerm::IsFlagged(false));
    } else if t == "has:attachment" || t == "has:attachments" || t == "tag:attachment" {
        return Some(SearchTerm::HasAttachment(true));
    } else if let Some(val) = t.strip_prefix("tag:").or_else(|| t.strip_prefix("label:")) {
        let clean = clean_val(val);
        if !clean.is_empty() {
            return Some(SearchTerm::Tag(clean.to_lowercase()));
        }
    } else {
        let clean = clean_val(t);
        if !clean.is_empty() {
            return Some(SearchTerm::General(clean.to_lowercase()));
        }
    }

    None
}

fn matches_single_term(term: &SearchTerm, thread: &Thread) -> bool {
    match term {
        SearchTerm::From(val) => {
            thread.senders.iter().any(|s| matches_wildcard(val, s))
                || thread.messages.iter().any(|m| matches_wildcard(val, &m.from))
        }
        SearchTerm::To(val) => {
            thread
                .messages
                .iter()
                .any(|m| m.to.iter().any(|t| matches_wildcard(val, t)))
        }
        SearchTerm::Cc(val) => {
            thread
                .messages
                .iter()
                .any(|m| m.cc.iter().any(|c| matches_wildcard(val, c)))
        }
        SearchTerm::Subject(val) => matches_wildcard(val, &thread.subject),
        SearchTerm::Body(val) => {
            matches_wildcard(val, &thread.snippet)
                || thread.messages.iter().any(|m| {
                    m.body_plain
                        .as_ref()
                        .map(|b| matches_wildcard(val, b))
                        .unwrap_or(false)
                        || m.body_html
                            .as_ref()
                            .map(|b| matches_wildcard(val, b))
                            .unwrap_or(false)
                })
        }
        SearchTerm::Attachment(val) => {
            thread
                .messages
                .iter()
                .any(|m| m.attachments.iter().any(|a| matches_wildcard(val, &a.filename)))
        }
        SearchTerm::Tag(val) => {
            thread.tags.iter().any(|t| matches_wildcard(val, t))
        }
        SearchTerm::IsUnread(expected) => thread.is_unread == *expected,
        SearchTerm::IsFlagged(expected) => thread.is_flagged == *expected,
        SearchTerm::HasAttachment(expected) => {
            let has_att = thread.messages.iter().any(|m| !m.attachments.is_empty());
            has_att == *expected
        }
        SearchTerm::General(val) => {
            matches_wildcard(val, &thread.subject)
                || matches_wildcard(val, &thread.snippet)
                || thread.tags.iter().any(|t| matches_wildcard(val, t))
                || thread.senders.iter().any(|s| matches_wildcard(val, s))
                || thread.messages.iter().any(|m| {
                    matches_wildcard(val, &m.from)
                        || m.to.iter().any(|t| matches_wildcard(val, t))
                        || m.cc.iter().any(|c| matches_wildcard(val, c))
                        || m.body_plain
                            .as_ref()
                            .map(|b| matches_wildcard(val, b))
                            .unwrap_or(false)
                        || m.body_html
                            .as_ref()
                            .map(|b| matches_wildcard(val, b))
                            .unwrap_or(false)
                        || m.attachments
                            .iter()
                            .any(|a| matches_wildcard(val, &a.filename))
                })
        }
        SearchTerm::Not(inner) => !matches_single_term(inner, thread),
    }
}

/// Case-insensitive text and wildcard matcher.
/// If `pattern` contains '*' or '?', performs wildcard matching;
/// otherwise performs standard case-insensitive substring matching.
pub fn matches_wildcard(pattern: &str, text: &str) -> bool {
    let p = pattern.trim().to_lowercase();
    let t = text.trim().to_lowercase();
    if p.is_empty() {
        return true;
    }
    if !p.contains('*') && !p.contains('?') {
        return t.contains(&p);
    }
    wildcard_match_chars(&p.chars().collect::<Vec<_>>(), &t.chars().collect::<Vec<_>>())
}

fn wildcard_match_chars(pattern: &[char], text: &[char]) -> bool {
    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut p_star = None;
    let mut t_star = 0;

    while t_idx < text.len() {
        if p_idx < pattern.len() && (pattern[p_idx] == '?' || pattern[p_idx] == text[t_idx]) {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < pattern.len() && pattern[p_idx] == '*' {
            p_star = Some(p_idx);
            p_idx += 1;
            t_star = t_idx;
        } else if let Some(star) = p_star {
            p_idx = star + 1;
            t_star += 1;
            t_idx = t_star;
        } else {
            return false;
        }
    }

    while p_idx < pattern.len() && pattern[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == pattern.len()
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

    #[test]
    fn test_search_attachment_and_wildcard_matching() {
        let thread = make_test_thread(
            "Hermes Audio Pipeline",
            &["Alex Rivera <alex@hermes.dev>"],
            "bry@omarchy.org",
            "Goertzel filter bank",
            "Full implementation details inside",
            true,
            false,
            true, // has file.pdf
        );

        // Attachment search
        assert!(SearchQuery::parse("attach:file.pdf").matches_thread(&thread));
        assert!(SearchQuery::parse("attach:*.pdf").matches_thread(&thread));
        assert!(SearchQuery::parse("filename:*pdf*").matches_thread(&thread));
        assert!(!SearchQuery::parse("attach:*.zip").matches_thread(&thread));

        // Wildcard search across senders and subject
        assert!(SearchQuery::parse("from:*@hermes.dev*").matches_thread(&thread));
        assert!(SearchQuery::parse("subject:*audio*").matches_thread(&thread));
        assert!(SearchQuery::parse("subject:hermes*").matches_thread(&thread));

        // Negation
        assert!(SearchQuery::parse("-from:spam").matches_thread(&thread));
        assert!(SearchQuery::parse("-is:read").matches_thread(&thread));
        assert!(!SearchQuery::parse("-from:alex").matches_thread(&thread));
        assert!(!SearchQuery::parse("-has:attachment").matches_thread(&thread));

        // General full text search matches body
        assert!(SearchQuery::parse("implementation").matches_thread(&thread));
        assert!(SearchQuery::parse("filter").matches_thread(&thread));
    }

    #[test]
    fn test_matches_wildcard_cases() {
        assert!(matches_wildcard("*.flac", "audio_spec.flac"));
        assert!(matches_wildcard("spec*", "spec_v2.pdf"));
        assert!(matches_wildcard("*spec*", "hermes_spec_v2.pdf"));
        assert!(matches_wildcard("alex", "Alex Rivera"));
        assert!(!matches_wildcard("*.zip", "audio_spec.flac"));
    }

    #[test]
    fn test_search_tags_and_labels() {
        let mut thread = make_test_thread(
            "Project Review",
            &["Alex Rivera <alex@hermes.dev>"],
            "Brayan <bry@omarchy.org>",
            "Reviewing architecture",
            "Looks great overall",
            false,
            false,
            false,
        );
        thread.tags = vec!["Urgent".to_string(), "Project-Alpha".to_string()];

        assert!(SearchQuery::parse("tag:urgent").matches_thread(&thread));
        assert!(SearchQuery::parse("label:urgent").matches_thread(&thread));
        assert!(SearchQuery::parse("tag:project*").matches_thread(&thread));
        assert!(SearchQuery::parse("tag:*alpha").matches_thread(&thread));
        assert!(!SearchQuery::parse("tag:finance").matches_thread(&thread));

        // Universal general search matches tag
        assert!(SearchQuery::parse("Urgent").matches_thread(&thread));
        assert!(SearchQuery::parse("Project-Alpha").matches_thread(&thread));
    }
}
