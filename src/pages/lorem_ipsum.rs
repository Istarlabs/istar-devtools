// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, LoremIpsumState};

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<LoremIpsumState>().on_generate(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<LoremIpsumState>();

        let count = state
            .get_count()
            .as_str()
            .trim()
            .parse::<usize>()
            .unwrap_or(3)
            .max(1)
            .min(500);
        let unit_index = state.get_unit_index();
        let classic_start = state.get_classic_start();

        let output = match unit_index {
            0 => generate_words(count),
            1 => generate_sentences(count, classic_start),
            _ => generate_paragraphs(count, classic_start),
        };

        state.set_output(output.to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<LoremIpsumState>().on_copy_output(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<LoremIpsumState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn generate_words(count: usize) -> String {
    let raw = lipsum::lipsum(count + 5);
    raw.split_whitespace()
        .take(count)
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_sentences(count: usize, classic_start: bool) -> String {
    let raw = lipsum::lipsum(count * 12 + 30);
    let all: Vec<String> = raw
        .split(". ")
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let s = s.trim();
            if s.ends_with('.') {
                s.to_string()
            } else {
                format!("{s}.")
            }
        })
        .collect();

    let skip = if classic_start { 0 } else { 1.min(all.len().saturating_sub(count)) };
    all.into_iter().skip(skip).take(count).collect::<Vec<_>>().join(" ")
}

fn generate_paragraphs(count: usize, classic_start: bool) -> String {
    const WORDS_PER_PARAGRAPH: usize = 75;
    let extra = if classic_start { 0 } else { WORDS_PER_PARAGRAPH };
    let raw = lipsum::lipsum(count * WORDS_PER_PARAGRAPH + extra + 10);
    let words: Vec<&str> = raw.split_whitespace().collect();

    let skip = if classic_start { 0 } else { WORDS_PER_PARAGRAPH.min(words.len()) };
    let usable: Vec<&str> = words.into_iter().skip(skip).collect();

    (0..count)
        .filter_map(|i| {
            let start = i * WORDS_PER_PARAGRAPH;
            if start >= usable.len() {
                return None;
            }
            let end = (start + WORDS_PER_PARAGRAPH).min(usable.len());
            Some(usable[start..end].join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
