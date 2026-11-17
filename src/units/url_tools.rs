// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Percent-encoding and URL parsing.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use url::Url;

/// Everything but the RFC 3986 unreserved characters, like JavaScript's `encodeURIComponent`
/// (which additionally leaves `!*'()` alone; those are escaped here, which is also valid).
const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Like JavaScript's `encodeURI`: reserved characters that structure a URL stay as they are.
const FULL_URL: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'!')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b';')
    .remove(b',')
    .remove(b'/')
    .remove(b'?')
    .remove(b':')
    .remove(b'@')
    .remove(b'&')
    .remove(b'=')
    .remove(b'+')
    .remove(b'$')
    .remove(b'#');

pub fn encode_component(text: &str) -> String {
    utf8_percent_encode(text, COMPONENT).to_string()
}

pub fn encode_url(text: &str) -> String {
    utf8_percent_encode(text, FULL_URL).to_string()
}

/// Decode percent escapes. With `form` set, `+` stands for a space (`application/x-www-form-urlencoded`).
pub fn decode(text: &str, form: bool) -> Result<String, String> {
    let source = if form {
        text.replace('+', " ")
    } else {
        text.to_string()
    };
    percent_decode_str(&source)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .map_err(|_| "The decoded bytes are not valid UTF-8".to_string())
}

/// Break a URL into labelled rows, query parameters last.
pub fn parse(text: &str) -> Result<Vec<(String, String)>, String> {
    let url = Url::parse(text.trim()).map_err(|e| e.to_string())?;

    let mut rows = vec![("Scheme".to_string(), url.scheme().to_string())];
    if !url.username().is_empty() {
        rows.push(("User".to_string(), decode(url.username(), false)?));
    }
    if let Some(password) = url.password() {
        rows.push(("Password".to_string(), decode(password, false)?));
    }
    if let Some(host) = url.host_str() {
        rows.push(("Host".to_string(), host.to_string()));
    }
    if let Some(port) = url.port_or_known_default() {
        let note = if url.port().is_none() {
            " (default)"
        } else {
            ""
        };
        rows.push(("Port".to_string(), format!("{port}{note}")));
    }
    rows.push(("Path".to_string(), decode(url.path(), false)?));
    if let Some(query) = url.query() {
        rows.push(("Query".to_string(), query.to_string()));
    }
    if let Some(fragment) = url.fragment() {
        rows.push(("Fragment".to_string(), decode(fragment, false)?));
    }
    for (key, value) in url.query_pairs() {
        rows.push((format!("Param: {key}"), value.into_owned()));
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_components_and_full_urls() {
        assert_eq!(encode_component("a b&c=d/é~"), "a%20b%26c%3Dd%2F%C3%A9~");
        assert_eq!(
            encode_url("https://x.org/a b?q=ü&r=1#top"),
            "https://x.org/a%20b?q=%C3%BC&r=1#top"
        );
    }

    #[test]
    fn decodes_plain_and_form_data() {
        assert_eq!(decode("a%20b+c", false).unwrap(), "a b+c");
        assert_eq!(decode("a%20b+c", true).unwrap(), "a b c");
        assert_eq!(decode("%C3%A9", false).unwrap(), "é");
        assert!(decode("%FF", false).is_err());
    }

    #[test]
    fn parses_all_parts() {
        let rows = parse("https://me:pw@example.com:8443/a%20b/c?x=1&y=two%20words#frag").unwrap();
        let get = |label: &str| {
            rows.iter()
                .find(|(l, _)| l == label)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("Scheme"), Some("https"));
        assert_eq!(get("User"), Some("me"));
        assert_eq!(get("Password"), Some("pw"));
        assert_eq!(get("Host"), Some("example.com"));
        assert_eq!(get("Port"), Some("8443"));
        assert_eq!(get("Path"), Some("/a b/c"));
        assert_eq!(get("Fragment"), Some("frag"));
        assert_eq!(get("Param: y"), Some("two words"));
    }

    #[test]
    fn default_port_is_marked_and_garbage_is_rejected() {
        let rows = parse("http://example.com").unwrap();
        assert!(rows.contains(&("Port".to_string(), "80 (default)".to_string())));
        assert!(parse("not a url").is_err());
    }
}
