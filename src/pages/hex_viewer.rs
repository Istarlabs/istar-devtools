// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::{Cell, RefCell};
use std::io::Read;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::hex_viewer::{self, BYTES_PER_ROW};
use crate::{AppWindow, HexViewerState, ResultRow};

/// Rows shown at a time.
const WINDOW: usize = 256 * BYTES_PER_ROW;
/// Largest part of a file that is loaded.
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MATCHES: usize = 100;

#[derive(Default)]
struct Loaded {
    bytes: RefCell<Vec<u8>>,
    start: Cell<usize>,
}

pub fn register(app: &AppWindow) {
    let loaded = Rc::new(Loaded::default());

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_open_file(move || {
        let ui = handle.unwrap();
        let state = ui.global::<HexViewerState>();
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };
        let result = std::fs::File::open(&path).and_then(|file| {
            let total = file.metadata()?.len();
            let mut bytes = Vec::new();
            file.take(MAX_FILE_BYTES).read_to_end(&mut bytes)?;
            Ok((bytes, total > MAX_FILE_BYTES))
        });
        match result {
            Ok((bytes, truncated)) => {
                load(&state, &data, bytes, &path.display().to_string(), truncated)
            }
            Err(e) => state.set_error(format!("{}: {e}", path.display()).to_shared_string()),
        }
    });

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_paste_text(move || {
        let ui = handle.unwrap();
        let state = ui.global::<HexViewerState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            load(&state, &data, text.into_bytes(), "clipboard (text)", false);
        }
    });

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_paste_hex(move || {
        let ui = handle.unwrap();
        let state = ui.global::<HexViewerState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            match hex_viewer::parse_hex(&text) {
                Ok(bytes) => load(&state, &data, bytes, "clipboard (hex)", false),
                Err(e) => state.set_error(e.to_shared_string()),
            }
        }
    });

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_go_to(move || {
        let ui = handle.unwrap();
        let state = ui.global::<HexViewerState>();
        match hex_viewer::parse_offset(state.get_offset_text().as_str()) {
            Ok(offset) => {
                state.set_error("".to_shared_string());
                show(&state, &data, offset);
            }
            Err(e) => state.set_error(e.to_shared_string()),
        }
    });

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_find(move || {
        let ui = handle.unwrap();
        let state = ui.global::<HexViewerState>();
        state.set_matches(ModelRc::default());
        state.set_error("".to_shared_string());

        let query = state.get_search_text();
        if query.is_empty() {
            return;
        }
        let pattern = if state.get_search_is_hex() {
            match hex_viewer::parse_hex(query.as_str()) {
                Ok(pattern) => pattern,
                Err(e) => return state.set_error(e.to_shared_string()),
            }
        } else {
            query.as_bytes().to_vec()
        };

        let found = hex_viewer::find_all(&data.bytes.borrow(), &pattern, MAX_MATCHES);
        if found.is_empty() {
            return state.set_error("No match".to_shared_string());
        }
        let rows: Vec<ResultRow> = found
            .iter()
            .enumerate()
            .map(|(index, offset)| ResultRow {
                label: format!("#{}", index + 1).to_shared_string(),
                value: format!("0x{offset:x} ({offset})").to_shared_string(),
            })
            .collect();
        state.set_matches(ModelRc::new(VecModel::from(rows)));
        show(&state, &data, found[0]);
    });

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_prev_page(move || {
        let ui = handle.unwrap();
        let start = data.start.get().saturating_sub(WINDOW);
        show(&ui.global::<HexViewerState>(), &data, start);
    });

    let handle = app.as_weak();
    let data = loaded.clone();
    app.global::<HexViewerState>().on_next_page(move || {
        let ui = handle.unwrap();
        let start = data.start.get() + WINDOW;
        show(&ui.global::<HexViewerState>(), &data, start);
    });

    let data = loaded.clone();
    app.global::<HexViewerState>().on_copy_dump(move || {
        let text = hex_viewer::dump(&data.bytes.borrow(), data.start.get(), WINDOW);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    });

    app.global::<HexViewerState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });
}

fn load(state: &HexViewerState, data: &Loaded, bytes: Vec<u8>, source: &str, truncated: bool) {
    let facts: Vec<ResultRow> = hex_viewer::facts(&bytes, source, truncated)
        .into_iter()
        .map(|(label, value)| ResultRow {
            label: label.to_shared_string(),
            value: value.to_shared_string(),
        })
        .collect();
    state.set_facts(ModelRc::new(VecModel::from(facts)));
    state.set_matches(ModelRc::default());
    state.set_error("".to_shared_string());
    state.set_has_data(true);
    *data.bytes.borrow_mut() = bytes;
    show(state, data, 0);
}

/// Show the window containing `offset`.
fn show(state: &HexViewerState, data: &Loaded, offset: usize) {
    let bytes = data.bytes.borrow();
    let start = (offset / WINDOW * WINDOW).min(bytes.len().saturating_sub(1) / WINDOW * WINDOW);
    data.start.set(start);

    let end = (start + WINDOW).min(bytes.len());
    state.set_dump(hex_viewer::dump(&bytes, start, WINDOW).to_shared_string());
    state.set_window_info(
        format!(
            "0x{start:x} – 0x{:x} of 0x{:x}",
            end.saturating_sub(1),
            bytes.len()
        )
        .to_shared_string(),
    );
    state.set_can_prev(start > 0);
    state.set_can_next(end < bytes.len());
}
