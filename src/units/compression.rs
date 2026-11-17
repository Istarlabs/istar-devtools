// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Gzip, zlib and raw deflate for text.

use std::io::{Read, Write};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use flate2::Compression as Level;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};

/// Decompressed output larger than this is refused, so a small "bomb" cannot exhaust memory.
pub const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Gzip,
    Zlib,
    Deflate,
}

impl Format {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Format::Zlib,
            2 => Format::Deflate,
            _ => Format::Gzip,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Gzip => "gzip",
            Format::Zlib => "zlib",
            Format::Deflate => "raw deflate",
        }
    }
}

pub fn level_from_index(index: i32) -> Level {
    match index {
        0 => Level::fast(),
        2 => Level::best(),
        _ => Level::default(),
    }
}

pub fn compress(data: &[u8], format: Format, level: Level) -> Result<Vec<u8>, String> {
    let error = |e: std::io::Error| e.to_string();
    match format {
        Format::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), level);
            encoder.write_all(data).map_err(error)?;
            encoder.finish().map_err(error)
        }
        Format::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), level);
            encoder.write_all(data).map_err(error)?;
            encoder.finish().map_err(error)
        }
        Format::Deflate => {
            let mut encoder = DeflateEncoder::new(Vec::new(), level);
            encoder.write_all(data).map_err(error)?;
            encoder.finish().map_err(error)
        }
    }
}

/// Detect the container from its header: gzip magic, a valid zlib header, otherwise raw deflate.
pub fn detect(data: &[u8]) -> Format {
    match data {
        [0x1f, 0x8b, ..] => Format::Gzip,
        [first, second, ..]
            if first & 0x0f == 8 && (u16::from(*first) * 256 + u16::from(*second)) % 31 == 0 =>
        {
            Format::Zlib
        }
        _ => Format::Deflate,
    }
}

fn read_capped(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    reader
        .take(MAX_OUTPUT_BYTES + 1)
        .read_to_end(&mut out)
        .map_err(|e| format!("Not valid compressed data: {e}"))?;
    if out.len() as u64 > MAX_OUTPUT_BYTES {
        return Err(format!(
            "The data expands to more than {} MiB; refusing to decompress it",
            MAX_OUTPUT_BYTES / 1024 / 1024
        ));
    }
    Ok(out)
}

pub fn decompress(data: &[u8]) -> Result<(Vec<u8>, Format), String> {
    let format = detect(data);
    let out = match format {
        Format::Gzip => read_capped(GzDecoder::new(data))?,
        Format::Zlib => read_capped(ZlibDecoder::new(data))?,
        Format::Deflate => read_capped(DeflateDecoder::new(data))?,
    };
    Ok((out, format))
}

/// Everything the text could be, most specific first: hex, then standard and URL-safe Base64.
pub fn decode_candidates(text: &str) -> Vec<Vec<u8>> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut candidates = Vec::new();
    if let Ok(bytes) = hex::decode(&cleaned) {
        candidates.push(bytes);
    }
    for engine in [&STANDARD, &URL_SAFE] {
        if let Ok(bytes) = engine.decode(cleaned.trim_end_matches('=')) {
            candidates.push(bytes);
        }
    }
    // The base64 engines above are strict about padding, so also try with it restored.
    if let Ok(bytes) = STANDARD.decode(&cleaned) {
        candidates.push(bytes);
    }
    candidates
}

pub fn encode_output(data: &[u8], hex_output: bool) -> String {
    if hex_output {
        hex::encode(data)
    } else {
        STANDARD.encode(data)
    }
}

/// Compress `text` and describe the result.
pub fn compress_text(
    text: &str,
    format: Format,
    level: Level,
    hex_output: bool,
) -> Result<(String, String), String> {
    let compressed = compress(text.as_bytes(), format, level)?;
    let ratio = if text.is_empty() {
        100.0
    } else {
        compressed.len() as f64 * 100.0 / text.len() as f64
    };
    let info = format!(
        "{}: {} bytes → {} bytes ({ratio:.1} %)",
        format.name(),
        text.len(),
        compressed.len()
    );
    Ok((encode_output(&compressed, hex_output), info))
}

/// Decode `text` (hex or Base64) and decompress it into text.
pub fn decompress_text(text: &str) -> Result<(String, String), String> {
    let mut last_error = "The input is neither hex nor Base64".to_string();
    for bytes in decode_candidates(text) {
        match decompress(&bytes) {
            Ok((out, format)) => {
                let info = format!(
                    "{}: {} bytes → {} bytes",
                    format.name(),
                    bytes.len(),
                    out.len()
                );
                return Ok(match String::from_utf8(out) {
                    Ok(text) => (text, info),
                    Err(e) => (
                        String::from_utf8_lossy(e.as_bytes()).into_owned(),
                        format!("{info} (not valid UTF-8, shown lossy)"),
                    ),
                });
            }
            Err(e) => last_error = e,
        }
    }
    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        "The quick brown fox jumps over the lazy dog. ".repeat(20)
    }

    #[test]
    fn round_trips_every_format_and_encoding() {
        let sample = sample();
        for format in [Format::Gzip, Format::Zlib, Format::Deflate] {
            for hex_output in [false, true] {
                let (encoded, info) =
                    compress_text(&sample, format, Level::default(), hex_output).unwrap();
                assert!(info.starts_with(format.name()), "{info}");
                let (decoded, _) = decompress_text(&encoded).unwrap();
                assert_eq!(decoded, sample, "{format:?} hex={hex_output}");
            }
        }
    }

    #[test]
    fn detects_the_container() {
        assert_eq!(
            detect(&compress(b"x", Format::Gzip, Level::fast()).unwrap()),
            Format::Gzip
        );
        assert_eq!(
            detect(&compress(b"x", Format::Zlib, Level::best()).unwrap()),
            Format::Zlib
        );
        assert_eq!(
            detect(&compress(b"x", Format::Deflate, Level::default()).unwrap()),
            Format::Deflate
        );
        // Well-known header bytes of a zlib stream.
        assert_eq!(detect(&[0x78, 0x9c, 0]), Format::Zlib);
    }

    #[test]
    fn known_gzip_data_decodes() {
        // `printf hello | gzip -n | base64`
        let (text, info) = decompress_text("H4sIAAAAAAAAA8tIzcnJBwCGphA2BQAAAA==").unwrap();
        assert_eq!(text, "hello");
        assert!(info.starts_with("gzip"));
    }

    #[test]
    fn repetitive_text_shrinks_and_levels_differ() {
        let sample = sample();
        let fast = compress(sample.as_bytes(), Format::Zlib, Level::fast()).unwrap();
        assert!(fast.len() < sample.len() / 4);
        assert!(level_from_index(2).level() > level_from_index(0).level());
    }

    #[test]
    fn rejects_garbage_and_bombs() {
        assert!(decompress_text("not base64 or hex!!").is_err());
        assert!(decompress_text("AAECAwQF").is_err());
        let bomb = compress(
            &vec![0u8; (MAX_OUTPUT_BYTES + 1024) as usize],
            Format::Gzip,
            Level::fast(),
        )
        .unwrap();
        assert!(bomb.len() < 1_000_000);
        let error = decompress(&bomb).unwrap_err();
        assert!(error.contains("refusing"), "{error}");
    }

    #[test]
    fn non_utf8_output_is_flagged() {
        let encoded = encode_output(
            &compress(&[0xff, 0xfe, b'a'], Format::Gzip, Level::default()).unwrap(),
            false,
        );
        let (text, info) = decompress_text(&encoded).unwrap();
        assert!(text.ends_with('a'));
        assert!(info.contains("not valid UTF-8"));
    }
}
