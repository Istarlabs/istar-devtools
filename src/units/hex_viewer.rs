// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Classic offset / hex / ASCII dumps and byte searches.

use std::fmt::Write;

pub const BYTES_PER_ROW: usize = 16;

/// Dump `length` bytes from `start` (rounded down to a row boundary).
pub fn dump(bytes: &[u8], start: usize, length: usize) -> String {
    let start = (start / BYTES_PER_ROW * BYTES_PER_ROW).min(bytes.len());
    let end = (start + length).min(bytes.len());
    let mut out = String::new();

    for row_start in (start..end).step_by(BYTES_PER_ROW) {
        let row = &bytes[row_start..(row_start + BYTES_PER_ROW).min(end)];
        let _ = write!(out, "{row_start:08x}  ");
        for index in 0..BYTES_PER_ROW {
            match row.get(index) {
                Some(byte) => {
                    let _ = write!(out, "{byte:02x} ");
                }
                None => out.push_str("   "),
            }
            if index == 7 {
                out.push(' ');
            }
        }
        out.push_str(" |");
        out.extend(row.iter().map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        }));
        out.push_str("|\n");
    }
    out
}

/// Parse `de ad be ef`, `0xDE,0xAD` or `deadbeef` into bytes.
pub fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == ':')
        .map(|part| part.trim_start_matches("0x").trim_start_matches("0X"))
        .collect();
    if cleaned.is_empty() {
        return Err("No hex digits given".to_string());
    }
    hex::decode(&cleaned).map_err(|e| format!("Not valid hex: {e}"))
}

/// A decimal number, or hexadecimal with a `0x` prefix.
pub fn parse_offset(text: &str) -> Result<usize, String> {
    let text = text.trim();
    let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => usize::from_str_radix(hex, 16),
        None => text.parse::<usize>(),
    };
    parsed.map_err(|_| format!("\"{text}\" is not an offset (decimal or 0x…)"))
}

/// Start offsets of every (overlapping) occurrence of `pattern`, at most `limit`.
pub fn find_all(bytes: &[u8], pattern: &[u8], limit: usize) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > bytes.len() {
        return Vec::new();
    }
    bytes
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| *window == pattern)
        .map(|(offset, _)| offset)
        .take(limit)
        .collect()
}

pub fn detect_type(bytes: &[u8]) -> Option<&'static str> {
    const SIGNATURES: &[(&[u8], &str)] = &[
        (b"\x89PNG\r\n\x1a\n", "PNG image"),
        (b"\xff\xd8\xff", "JPEG image"),
        (b"GIF87a", "GIF image"),
        (b"GIF89a", "GIF image"),
        (b"%PDF-", "PDF document"),
        (b"PK\x03\x04", "ZIP archive (also docx, xlsx, jar, apk)"),
        (b"\x1f\x8b", "gzip data"),
        (b"\x7fELF", "ELF executable"),
        (b"MZ", "DOS/Windows executable"),
        (b"BM", "BMP image"),
        (b"RIFF", "RIFF container (WAV, WebP, AVI)"),
        (b"OggS", "Ogg media"),
        (b"ID3", "MP3 audio (ID3 tag)"),
        (b"fLaC", "FLAC audio"),
        (b"\xfd7zXZ\0", "xz data"),
        (b"BZh", "bzip2 data"),
        (b"7z\xbc\xaf\x27\x1c", "7-Zip archive"),
        (b"\x28\xb5\x2f\xfd", "Zstandard data"),
        (b"SQLite format 3\0", "SQLite database"),
        (b"-----BEGIN", "PEM text"),
        (b"{\\rtf", "RTF document"),
        (b"\xca\xfe\xba\xbe", "Java class / Mach-O universal binary"),
    ];
    SIGNATURES
        .iter()
        .find(|(magic, _)| bytes.starts_with(magic))
        .map(|(_, name)| *name)
}

/// Size, type and encoding facts as label / value pairs.
pub fn facts(bytes: &[u8], source: &str, truncated: bool) -> Vec<(String, String)> {
    let mut rows = vec![("Source".to_string(), source.to_string())];
    let size = if truncated {
        format!(
            "{} bytes loaded (file is larger, only the beginning is shown)",
            bytes.len()
        )
    } else {
        format!("{} bytes (0x{:x})", bytes.len(), bytes.len())
    };
    rows.push(("Size".to_string(), size));
    if let Some(kind) = detect_type(bytes) {
        rows.push(("Type".to_string(), kind.to_string()));
    }
    let utf8 = match std::str::from_utf8(bytes) {
        Ok(_) => "valid UTF-8".to_string(),
        // A cut-off multi-byte character at the end of a truncated file is not an error.
        Err(e) if truncated && e.error_len().is_none() => "valid UTF-8".to_string(),
        Err(e) => format!("invalid at offset {}", e.valid_up_to()),
    };
    rows.push(("Encoding".to_string(), utf8));
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        rows.push(("BOM".to_string(), "UTF-8 byte order mark".to_string()));
    } else if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        rows.push(("BOM".to_string(), "UTF-16 byte order mark".to_string()));
    }
    let crlf = bytes.windows(2).filter(|w| *w == b"\r\n").count();
    let lf = bytes.iter().filter(|&&b| b == b'\n').count();
    if lf > 0 {
        rows.push((
            "Line endings".to_string(),
            if crlf == lf {
                "CRLF (Windows)".to_string()
            } else if crlf == 0 {
                "LF (Unix)".to_string()
            } else {
                "mixed".to_string()
            },
        ));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_layout() {
        let bytes = b"Hello, hex viewer!\x00\xff";
        let text = dump(bytes, 0, 4096);
        let expected = "00000000  48 65 6c 6c 6f 2c 20 68  65 78 20 76 69 65 77 65  |Hello, hex viewe|\n\
00000010  72 21 00 ff                                       |r!..|\n";
        assert_eq!(text, expected);
    }

    #[test]
    fn dump_windows_are_row_aligned_and_bounded() {
        let bytes: Vec<u8> = (0..100).collect();
        let text = dump(&bytes, 37, 32);
        let rows: Vec<&str> = text.lines().collect();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("00000020  20 21"));
        assert!(rows[1].starts_with("00000030  30 31"));
        assert_eq!(dump(&bytes, 1000, 16), "");
        assert_eq!(dump(&[], 0, 16), "");
    }

    #[test]
    fn parsing_hex_and_offsets() {
        assert_eq!(parse_hex("de ad BE:ef").unwrap(), [0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(parse_hex("0xDE,0xAD").unwrap(), [0xde, 0xad]);
        assert_eq!(parse_hex("cafebabe").unwrap(), [0xca, 0xfe, 0xba, 0xbe]);
        assert!(parse_hex("abc").is_err());
        assert!(parse_hex("  ").is_err());
        assert_eq!(parse_offset("255").unwrap(), 255);
        assert_eq!(parse_offset("0xff").unwrap(), 255);
        assert!(parse_offset("ff").is_err());
    }

    #[test]
    fn searching_finds_overlapping_matches() {
        assert_eq!(find_all(b"aaaa", b"aa", 10), [0, 1, 2]);
        assert_eq!(find_all(b"abcabc", b"bc", 1), [1]);
        assert!(find_all(b"abc", b"", 10).is_empty());
        assert!(find_all(b"abc", b"abcd", 10).is_empty());
    }

    #[test]
    fn detects_types_and_facts() {
        assert_eq!(detect_type(b"\x89PNG\r\n\x1a\nrest"), Some("PNG image"));
        assert_eq!(detect_type(b"plain text"), None);

        let rows = facts(b"\xef\xbb\xbfa\r\nb\r\n", "test", false);
        let get = |label: &str| {
            rows.iter()
                .find(|(l, _)| l == label)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(get("Encoding").unwrap(), "valid UTF-8");
        assert_eq!(get("BOM").unwrap(), "UTF-8 byte order mark");
        assert_eq!(get("Line endings").unwrap(), "CRLF (Windows)");
        assert_eq!(get("Size").unwrap(), "9 bytes (0x9)");

        let broken = facts(b"ab\xffcd", "test", false);
        assert!(
            broken
                .iter()
                .any(|(l, v)| l == "Encoding" && v == "invalid at offset 2")
        );
        let cut = facts(b"ab\xc3", "test", true);
        assert!(
            cut.iter()
                .any(|(l, v)| l == "Encoding" && v == "valid UTF-8")
        );
    }
}
