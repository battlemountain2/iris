// SPDX-License-Identifier: GPL-3.0-or-later

//! Lightweight Markdown to HTML & Pango Markup Transformer for Iris
//!
//! Generates RFC 2046 compliant email HTML bodies with clean, accessible typography
//! and safe Pango markup for live preview within GTK4.

/// Converts a Markdown string into an email-ready HTML document.
pub fn markdown_to_html(md: &str) -> String {
    let mut html = String::with_capacity(md.len() * 2);
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<style>\n");
    html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; font-size: 14px; line-height: 1.6; color: #22252a; }\n");
    html.push_str("h1 { font-size: 1.6em; border-bottom: 1px solid #e5e9f0; padding-bottom: 4px; margin-top: 16px; margin-bottom: 10px; }\n");
    html.push_str("h2 { font-size: 1.3em; margin-top: 14px; margin-bottom: 8px; }\n");
    html.push_str("h3 { font-size: 1.1em; margin-top: 12px; margin-bottom: 6px; }\n");
    html.push_str("p { margin: 8px 0; }\n");
    html.push_str("a { color: #5e81ac; text-decoration: none; }\n");
    html.push_str("a:hover { text-decoration: underline; }\n");
    html.push_str("blockquote { border-left: 3px solid #88c0d0; padding-left: 12px; margin: 10px 0; color: #4c566a; font-style: italic; }\n");
    html.push_str("code { font-family: 'JetBrains Mono', Consolas, Monaco, monospace; font-size: 0.9em; background-color: #f0f2f5; padding: 2px 5px; border-radius: 4px; }\n");
    html.push_str("pre { background-color: #f4f6f9; border: 1px solid #e5e9f0; padding: 12px; border-radius: 6px; overflow-x: auto; }\n");
    html.push_str("pre code { background: none; padding: 0; font-size: 0.92em; }\n");
    html.push_str("ul, ol { padding-left: 20px; margin: 8px 0; }\n");
    html.push_str("li { margin: 4px 0; }\n");
    html.push_str("hr { border: none; border-top: 1px solid #d8dee9; margin: 20px 0; }\n");
    html.push_str("</style>\n</head>\n<body>\n");

    let body_html = convert_blocks(md, false);
    html.push_str(&body_html);
    html.push_str("\n</body>\n</html>");
    html
}

/// Converts a Markdown string into valid Pango markup for GTK4 live preview.
pub fn markdown_to_pango(md: &str) -> String {
    convert_blocks(md, true)
}

/// Checks whether a raw text body contains any Markdown formatting syntax.
pub fn contains_markdown(text: &str) -> bool {
    let patterns = [
        "**", "*", "__", "_", "`", "# ", "## ", "### ", "> ", "- ", "* ", "1. ", "2. ", "[",
    ];
    for p in &patterns {
        if text.contains(p) {
            return true;
        }
    }
    false
}

fn convert_blocks(md: &str, is_pango: bool) -> String {
    let mut out = String::with_capacity(md.len() + 128);
    let lines: Vec<&str> = md.lines().collect();
    let mut idx = 0;
    let total = lines.len();

    let mut in_list = false;
    let mut in_ol = false;

    while idx < total {
        let line = lines[idx];
        let trimmed = line.trim();

        // 1. Code blocks (```)
        if trimmed.starts_with("```") {
            if in_list {
                out.push_str(if is_pango { "\n" } else { "</ul>\n" });
                in_list = false;
            }
            if in_ol {
                out.push_str(if is_pango { "\n" } else { "</ol>\n" });
                in_ol = false;
            }

            idx += 1;
            let mut code_content = String::new();
            while idx < total && !lines[idx].trim().starts_with("```") {
                code_content.push_str(lines[idx]);
                code_content.push('\n');
                idx += 1;
            }
            if idx < total {
                idx += 1; // skip closing ```
            }

            let escaped = escape_xml(&code_content);
            if is_pango {
                out.push_str("<tt>");
                out.push_str(&escaped);
                out.push_str("</tt>\n\n");
            } else {
                out.push_str("<pre><code>");
                out.push_str(&escaped);
                out.push_str("</code></pre>\n");
            }
            continue;
        }

        // 2. Blank line -> close active lists and add spacing
        if trimmed.is_empty() {
            if in_list {
                out.push_str(if is_pango { "" } else { "</ul>\n" });
                in_list = false;
            }
            if in_ol {
                out.push_str(if is_pango { "" } else { "</ol>\n" });
                in_ol = false;
            }
            if is_pango {
                out.push('\n');
            }
            idx += 1;
            continue;
        }

        // 3. Horizontal Rule (--- or ***)
        if trimmed == "---" || trimmed == "***" {
            if in_list {
                out.push_str(if is_pango { "" } else { "</ul>\n" });
                in_list = false;
            }
            if in_ol {
                out.push_str(if is_pango { "" } else { "</ol>\n" });
                in_ol = false;
            }
            if is_pango {
                out.push_str("――――――――――――――――――――――――――――――\n\n");
            } else {
                out.push_str("<hr>\n");
            }
            idx += 1;
            continue;
        }

        // 4. Headers (#, ##, ###)
        if let Some(h1) = trimmed.strip_prefix("# ") {
            let parsed = parse_inline(h1, is_pango);
            if is_pango {
                out.push_str(&format!("<span size=\"x-large\" weight=\"bold\">{parsed}</span>\n\n"));
            } else {
                out.push_str(&format!("<h1>{parsed}</h1>\n"));
            }
            idx += 1;
            continue;
        }
        if let Some(h2) = trimmed.strip_prefix("## ") {
            let parsed = parse_inline(h2, is_pango);
            if is_pango {
                out.push_str(&format!("<span size=\"large\" weight=\"bold\">{parsed}</span>\n\n"));
            } else {
                out.push_str(&format!("<h2>{parsed}</h2>\n"));
            }
            idx += 1;
            continue;
        }
        if let Some(h3) = trimmed.strip_prefix("### ") {
            let parsed = parse_inline(h3, is_pango);
            if is_pango {
                out.push_str(&format!("<span weight=\"bold\">{parsed}</span>\n\n"));
            } else {
                out.push_str(&format!("<h3>{parsed}</h3>\n"));
            }
            idx += 1;
            continue;
        }

        // 5. Blockquotes (> )
        if let Some(quote) = trimmed.strip_prefix("> ") {
            let parsed = parse_inline(quote, is_pango);
            if is_pango {
                out.push_str(&format!("  <i>│ {parsed}</i>\n"));
            } else {
                out.push_str(&format!("<blockquote><p>{parsed}</p></blockquote>\n"));
            }
            idx += 1;
            continue;
        }

        // 6. Unordered Lists (- or *)
        if let Some(item) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            if in_ol {
                out.push_str(if is_pango { "" } else { "</ol>\n" });
                in_ol = false;
            }
            if !in_list {
                if !is_pango {
                    out.push_str("<ul>\n");
                }
                in_list = true;
            }
            let parsed = parse_inline(item, is_pango);
            if is_pango {
                out.push_str(&format!(" • {parsed}\n"));
            } else {
                out.push_str(&format!("  <li>{parsed}</li>\n"));
            }
            idx += 1;
            continue;
        }

        // 7. Ordered Lists (1. , 2. )
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
                let item = &trimmed[dot_pos + 2..];
                if in_list {
                    out.push_str(if is_pango { "" } else { "</ul>\n" });
                    in_list = false;
                }
                if !in_ol {
                    if !is_pango {
                        out.push_str("<ol>\n");
                    }
                    in_ol = true;
                }
                let parsed = parse_inline(item, is_pango);
                if is_pango {
                    out.push_str(&format!(" {prefix}. {parsed}\n"));
                } else {
                    out.push_str(&format!("  <li>{parsed}</li>\n"));
                }
                idx += 1;
                continue;
            }
        }

        // 8. Regular paragraph lines
        if in_list {
            out.push_str(if is_pango { "" } else { "</ul>\n" });
            in_list = false;
        }
        if in_ol {
            out.push_str(if is_pango { "" } else { "</ol>\n" });
            in_ol = false;
        }

        let parsed = parse_inline(line, is_pango);
        if is_pango {
            out.push_str(&parsed);
            out.push('\n');
        } else {
            out.push_str(&format!("<p>{parsed}</p>\n"));
        }

        idx += 1;
    }

    if in_list && !is_pango {
        out.push_str("</ul>\n");
    }
    if in_ol && !is_pango {
        out.push_str("</ol>\n");
    }

    out
}

fn parse_inline(text: &str, is_pango: bool) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        // Escaped characters (\*, \_, \`)
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == '*' || next == '_' || next == '`' || next == '[' || next == ']' {
                    chars.next();
                    out.push_str(&escape_char(next));
                    continue;
                }
            }
            out.push('\\');
            continue;
        }

        // Inline Code (`code`)
        if c == '`' {
            let mut code = String::new();
            let mut closed = false;
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '`' {
                    closed = true;
                    break;
                }
                code.push(nc);
            }
            if closed {
                let esc = escape_xml(&code);
                if is_pango {
                    out.push_str(&format!("<tt>{esc}</tt>"));
                } else {
                    out.push_str(&format!("<code>{esc}</code>"));
                }
            } else {
                out.push('`');
                out.push_str(&escape_xml(&code));
            }
            continue;
        }

        // Links: [title](url)
        if c == '[' {
            let mut title = String::new();
            let mut closed_bracket = false;
            while let Some(&tc) = chars.peek() {
                chars.next();
                if tc == ']' {
                    closed_bracket = true;
                    break;
                }
                title.push(tc);
            }

            if closed_bracket && chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                let mut url = String::new();
                let mut closed_paren = false;
                while let Some(&uc) = chars.peek() {
                    chars.next();
                    if uc == ')' {
                        closed_paren = true;
                        break;
                    }
                    url.push(uc);
                }

                if closed_paren {
                    let esc_title = escape_xml(&title);
                    let esc_url = escape_xml(url.trim());
                    out.push_str(&format!("<a href=\"{esc_url}\">{esc_title}</a>"));
                    continue;
                } else {
                    out.push('[');
                    out.push_str(&escape_xml(&title));
                    out.push_str("](");
                    out.push_str(&escape_xml(&url));
                    continue;
                }
            } else {
                out.push('[');
                out.push_str(&escape_xml(&title));
                if closed_bracket {
                    out.push(']');
                }
                continue;
            }
        }

        // Bold (**text** or __text__)
        if (c == '*' && chars.peek() == Some(&'*')) || (c == '_' && chars.peek() == Some(&'_')) {
            let marker = c;
            chars.next(); // consume second marker
            let mut content = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == marker && chars.peek() == Some(&marker) {
                    chars.next();
                    closed = true;
                    break;
                }
                content.push(nc);
            }
            if closed {
                let inner = parse_inline(&content, is_pango);
                out.push_str(&format!("<b>{inner}</b>"));
            } else {
                out.push(marker);
                out.push(marker);
                out.push_str(&escape_xml(&content));
            }
            continue;
        }

        // Italic (*text* or _text_)
        if c == '*' || c == '_' {
            let marker = c;
            let mut content = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == marker {
                    closed = true;
                    break;
                }
                content.push(nc);
            }
            if closed {
                let inner = parse_inline(&content, is_pango);
                out.push_str(&format!("<i>{inner}</i>"));
            } else {
                out.push(marker);
                out.push_str(&escape_xml(&content));
            }
            continue;
        }

        // Strikethrough (~~text~~)
        if c == '~' && chars.peek() == Some(&'~') {
            chars.next();
            let mut content = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == '~' && chars.peek() == Some(&'~') {
                    chars.next();
                    closed = true;
                    break;
                }
                content.push(nc);
            }
            if closed {
                let inner = parse_inline(&content, is_pango);
                out.push_str(&format!("<s>{inner}</s>"));
            } else {
                out.push_str("~~");
                out.push_str(&escape_xml(&content));
            }
            continue;
        }

        // Standard character
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }

    out
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_char(c: char) -> String {
    match c {
        '&' => "&amp;".to_string(),
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '"' => "&quot;".to_string(),
        '\'' => "&apos;".to_string(),
        _ => c.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_headers_and_emphasis() {
        let md = "# Title\n\n## Subtitle\n\nThis is **bold** and *italic* and `code`.";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<h2>Subtitle</h2>"));
        assert!(html.contains("<b>bold</b>"));
        assert!(html.contains("<i>italic</i>"));
        assert!(html.contains("<code>code</code>"));

        let pango = markdown_to_pango(md);
        assert!(pango.contains("weight=\"bold\">Title</span>"));
        assert!(pango.contains("<b>bold</b>"));
        assert!(pango.contains("<i>italic</i>"));
        assert!(pango.contains("<tt>code</tt>"));
    }

    #[test]
    fn test_markdown_links_and_quotes() {
        let md = "[Iris Repo](https://github.com/battlemountain2/iris)\n\n> Liquid-glass email client";
        let html = markdown_to_html(md);
        assert!(html.contains("<a href=\"https://github.com/battlemountain2/iris\">Iris Repo</a>"));
        assert!(html.contains("<blockquote><p>Liquid-glass email client</p></blockquote>"));

        let pango = markdown_to_pango(md);
        assert!(pango.contains("<a href=\"https://github.com/battlemountain2/iris\">Iris Repo</a>"));
        assert!(pango.contains("│ Liquid-glass email client"));
    }

    #[test]
    fn test_markdown_lists_and_codeblock() {
        let md = "- Item 1\n- Item 2\n\n```rust\nfn main() {}\n```";
        let html = markdown_to_html(md);
        assert!(html.contains("<ul>\n  <li>Item 1</li>\n  <li>Item 2</li>\n</ul>"));
        assert!(html.contains("<pre><code>fn main() {}\n</code></pre>"));

        let pango = markdown_to_pango(md);
        assert!(pango.contains(" • Item 1\n • Item 2\n"));
        assert!(pango.contains("<tt>fn main() {}\n</tt>"));
    }

    #[test]
    fn test_contains_markdown() {
        assert!(contains_markdown("Check **this** out"));
        assert!(contains_markdown("# Heading"));
        assert!(contains_markdown("Visit [Link](https://iris.dev)"));
        assert!(!contains_markdown("Hello world, this is a plain message without formatting."));
    }
}
