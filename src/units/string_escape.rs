// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Escaping and unescaping of text for string literals and data formats.

use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    CSharp,
    CSharpVerbatim,
    Json,
    Rust,
    JavaScript,
    Regex,
    Sql,
    Shell,
    Html,
    Csv,
}

impl Target {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Target::CSharpVerbatim,
            2 => Target::Json,
            3 => Target::Rust,
            4 => Target::JavaScript,
            5 => Target::Regex,
            6 => Target::Sql,
            7 => Target::Shell,
            8 => Target::Html,
            9 => Target::Csv,
            _ => Target::CSharp,
        }
    }
}

pub fn escape(text: &str, target: Target) -> String {
    match target {
        Target::CSharp => escape_backslash(text, Style::CSharp),
        Target::Json => escape_backslash(text, Style::Json),
        Target::Rust => escape_backslash(text, Style::Rust),
        Target::JavaScript => escape_backslash(text, Style::JavaScript),
        Target::CSharpVerbatim => text.replace('"', "\"\""),
        Target::Regex => regex::escape(text),
        Target::Sql => text.replace('\'', "''"),
        Target::Shell => format!("'{}'", text.replace('\'', "'\\''")),
        Target::Html => text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;"),
        Target::Csv => {
            if text.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", text.replace('"', "\"\""))
            } else {
                text.to_string()
            }
        }
    }
}

pub fn unescape(text: &str, target: Target) -> Result<String, String> {
    match target {
        Target::CSharp => unescape_backslash(strip_quotes(text), Style::CSharp),
        Target::Json => unescape_backslash(strip_quotes(text), Style::Json),
        Target::Rust => unescape_backslash(strip_quotes(text), Style::Rust),
        Target::JavaScript => unescape_backslash(strip_quotes(text), Style::JavaScript),
        Target::CSharpVerbatim => Ok(strip_quotes(text).replace("\"\"", "\"")),
        Target::Regex => unescape_regex(text),
        Target::Sql => Ok(text.replace("''", "'")),
        Target::Shell => unescape_shell(text),
        Target::Html => Ok(html_escape::decode_html_entities(text).into_owned()),
        Target::Csv => Ok(unescape_csv(text)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    CSharp,
    Json,
    Rust,
    JavaScript,
}

fn escape_backslash(text: &str, style: Style) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' if style != Style::Json => out.push_str("\\0"),
            '\u{8}' if style != Style::Rust => out.push_str("\\b"),
            '\u{c}' if style != Style::Rust => out.push_str("\\f"),
            '\u{7}' if style == Style::CSharp => out.push_str("\\a"),
            '\u{b}' if matches!(style, Style::CSharp | Style::JavaScript) => out.push_str("\\v"),
            '\u{2028}' | '\u{2029}' if style == Style::JavaScript => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if (c as u32) < 0x20 || c == '\u{7f}' => match style {
                Style::Rust => {
                    let _ = write!(out, "\\u{{{:x}}}", c as u32);
                }
                Style::JavaScript => {
                    let _ = write!(out, "\\x{:02x}", c as u32);
                }
                _ => {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
            },
            c => out.push(c),
        }
    }
    out
}

/// Remove one pair of surrounding double quotes, if present.
fn strip_quotes(text: &str) -> &str {
    let trimmed = text.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        text
    }
}

fn hex_digits(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    min: usize,
    max: usize,
) -> Result<u32, String> {
    let mut value = 0u32;
    let mut count = 0;
    while count < max {
        match chars.peek().and_then(|c| c.to_digit(16)) {
            Some(digit) => {
                value = value * 16 + digit;
                chars.next();
                count += 1;
            }
            None => break,
        }
    }
    if count < min {
        Err(format!(
            "Expected {min} hex digit(s) in the escape sequence"
        ))
    } else {
        Ok(value)
    }
}

fn unescape_backslash(text: &str, style: Style) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut pending_high: Option<u32> = None;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            if pending_high.is_some() {
                return Err("A high surrogate must be followed by a low surrogate".to_string());
            }
            out.push(c);
            continue;
        }

        let escape = chars.next().ok_or("The text ends with a lone backslash")?;
        let simple = match escape {
            '\\' => Some('\\'),
            '"' => Some('"'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            '/' if style == Style::Json => Some('/'),
            'b' if style != Style::Rust => Some('\u{8}'),
            'f' if style != Style::Rust => Some('\u{c}'),
            '\'' if style != Style::Json => Some('\''),
            '0' if style != Style::Json => Some('\0'),
            'a' if style == Style::CSharp => Some('\u{7}'),
            'v' if matches!(style, Style::CSharp | Style::JavaScript) => Some('\u{b}'),
            _ => None,
        };
        if let Some(simple) = simple {
            if pending_high.is_some() {
                return Err("A high surrogate must be followed by a low surrogate".to_string());
            }
            out.push(simple);
            continue;
        }

        let code = match escape {
            'u' if chars.peek() == Some(&'{')
                && matches!(style, Style::Rust | Style::JavaScript) =>
            {
                chars.next();
                let value = hex_digits(&mut chars, 1, 6)?;
                if chars.next() != Some('}') {
                    return Err("Expected } to close \\u{...}".to_string());
                }
                value
            }
            'u' => hex_digits(&mut chars, 4, 4)?,
            'U' if style == Style::CSharp => hex_digits(&mut chars, 8, 8)?,
            'x' => {
                let (min, max) = if style == Style::CSharp {
                    (1, 4)
                } else {
                    (2, 2)
                };
                let value = hex_digits(&mut chars, min, max)?;
                if style == Style::Rust && value > 0x7f {
                    return Err("\\x escapes in Rust must be at most \\x7f".to_string());
                }
                value
            }
            '\n' if style == Style::Rust => {
                // Line continuation: the newline and the leading whitespace of the next line vanish.
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
                continue;
            }
            other => return Err(format!("Unknown escape sequence \\{other}")),
        };

        match (pending_high, code) {
            (Some(high), 0xDC00..=0xDFFF) => {
                let combined = 0x10000 + ((high - 0xD800) << 10) + (code - 0xDC00);
                out.push(char::from_u32(combined).ok_or("Invalid surrogate pair")?);
                pending_high = None;
            }
            (Some(_), _) => {
                return Err("A high surrogate must be followed by a low surrogate".to_string());
            }
            (None, 0xD800..=0xDBFF) => pending_high = Some(code),
            (None, 0xDC00..=0xDFFF) => return Err("Unexpected low surrogate".to_string()),
            (None, code) => {
                out.push(char::from_u32(code).ok_or("Not a valid Unicode scalar value")?)
            }
        }
    }
    if pending_high.is_some() {
        return Err("The text ends after a high surrogate".to_string());
    }
    Ok(out)
}

fn unescape_regex(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let next = chars.next().ok_or("The text ends with a lone backslash")?;
            if next.is_alphanumeric() {
                return Err(format!(
                    "\\{next} is a regex class or escape, not an escaped character"
                ));
            }
            out.push(next);
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Undo POSIX shell quoting of one or more words: `'...'`, `"..."` and backslashes.
fn unescape_shell(text: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = text.trim().chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => loop {
                match chars.next() {
                    Some('\'') => break,
                    Some(c) => out.push(c),
                    None => return Err("Unterminated single quote".to_string()),
                }
            },
            '"' => loop {
                match chars.next() {
                    Some('"') => break,
                    Some('\\') => match chars.next() {
                        Some(e @ ('"' | '\\' | '$' | '`')) => out.push(e),
                        Some(other) => {
                            out.push('\\');
                            out.push(other);
                        }
                        None => return Err("Unterminated double quote".to_string()),
                    },
                    Some(c) => out.push(c),
                    None => return Err("Unterminated double quote".to_string()),
                }
            },
            '\\' => out.push(chars.next().ok_or("The text ends with a lone backslash")?),
            c => out.push(c),
        }
    }
    Ok(out)
}

fn unescape_csv(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].replace("\"\"", "\"")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "say \"hi\"\\path\n\ttab\u{1}é😀";

    #[test]
    fn escapes_per_language() {
        assert_eq!(
            escape(SAMPLE, Target::CSharp),
            "say \\\"hi\\\"\\\\path\\n\\ttab\\u0001é😀"
        );
        assert_eq!(escape("a\u{7}\u{b}\0", Target::CSharp), "a\\a\\v\\0");
        assert_eq!(escape("a\u{8}\0", Target::Json), "a\\b\\u0000");
        assert_eq!(escape("a\u{1}", Target::Rust), "a\\u{1}");
        assert_eq!(
            escape("a\u{1}\u{2028}", Target::JavaScript),
            "a\\x01\\u2028"
        );
        assert_eq!(escape("say \"x\"", Target::CSharpVerbatim), "say \"\"x\"\"");
        assert_eq!(escape("a.b*c", Target::Regex), "a\\.b\\*c");
        assert_eq!(escape("it's", Target::Sql), "it''s");
        assert_eq!(escape("it's", Target::Shell), "'it'\\''s'");
        assert_eq!(
            escape("<a href=\"x\">&</a>", Target::Html),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
        assert_eq!(escape("plain", Target::Csv), "plain");
        assert_eq!(escape("a,\"b\"", Target::Csv), "\"a,\"\"b\"\"\"");
    }

    #[test]
    fn round_trips() {
        for target in [
            Target::CSharp,
            Target::CSharpVerbatim,
            Target::Json,
            Target::Rust,
            Target::JavaScript,
            Target::Regex,
            Target::Sql,
            Target::Shell,
            Target::Html,
            Target::Csv,
        ] {
            let text = "line1\nit's \"quoted\" a.b*c <&> back\\slash, é😀";
            // Targets without newline support in their escaped form still round-trip the characters they escape.
            let escaped = escape(text, target);
            assert_eq!(unescape(&escaped, target).unwrap(), text, "{target:?}");
        }
    }

    #[test]
    fn unescapes_unicode_forms() {
        assert_eq!(
            unescape("\\u00e9\\uD83D\\uDE00", Target::Json).unwrap(),
            "é😀"
        );
        assert_eq!(unescape("\\u{1F600}\\x41", Target::Rust).unwrap(), "😀A");
        assert_eq!(
            unescape("\\U0001F600\\x41\\a", Target::CSharp).unwrap(),
            "😀A\u{7}"
        );
        assert_eq!(
            unescape("\"quoted \\\"text\\\"\"", Target::Json).unwrap(),
            "quoted \"text\""
        );
        assert_eq!(unescape("a\\\n   b", Target::Rust).unwrap(), "ab");
    }

    #[test]
    fn rejects_invalid_sequences() {
        assert!(unescape("\\q", Target::CSharp).is_err());
        assert!(unescape("\\'", Target::Json).is_err());
        assert!(unescape("\\u12", Target::Json).is_err());
        assert!(unescape("\\uD83D", Target::Json).is_err());
        assert!(unescape("\\uDE00", Target::Json).is_err());
        assert!(unescape("\\xFF", Target::Rust).is_err());
        assert!(unescape("end\\", Target::Json).is_err());
        assert!(unescape("\\d+", Target::Regex).is_err());
        assert!(unescape("'open", Target::Shell).is_err());
    }

    #[test]
    fn shell_quoting_forms() {
        assert_eq!(
            unescape("'a b' \"c \\\"d\\\"\" e\\ f", Target::Shell).unwrap(),
            "a b c \"d\" e f"
        );
    }
}
