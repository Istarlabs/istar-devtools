// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ToSharedString};

use crate::units::compression::{self, Format};
use crate::{AppWindow, CompressionState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<CompressionState>().on_run(move || {
        let ui = handle.unwrap();
        run(&ui.global::<CompressionState>());
    });

    let handle = app.as_weak();
    app.global::<CompressionState>().on_swap(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CompressionState>();
        if !state.get_output().is_empty() {
            state.set_input(state.get_output());
        }
        state.set_decompress_mode(!state.get_decompress_mode());
        run(&state);
    });

    let handle = app.as_weak();
    app.global::<CompressionState>().on_copy_output(move || {
        let ui = handle.unwrap();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(ui.global::<CompressionState>().get_output().as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<CompressionState>().on_paste_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CompressionState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            state.set_input(text.to_shared_string());
            run(&state);
        }
    });
}

fn run(state: &CompressionState) {
    let input = state.get_input();
    if input.is_empty() {
        state.set_output("".to_shared_string());
        state.set_info("".to_shared_string());
        state.set_error("".to_shared_string());
        return;
    }

    let result = if state.get_decompress_mode() {
        compression::decompress_text(input.as_str())
    } else {
        compression::compress_text(
            input.as_str(),
            Format::from_index(state.get_format_index()),
            compression::level_from_index(state.get_level_index()),
            state.get_encoding_index() == 1,
        )
    };
    match result {
        Ok((output, info)) => {
            state.set_output(output.to_shared_string());
            state.set_info(info.to_shared_string());
            state.set_error("".to_shared_string());
        }
        Err(e) => {
            state.set_output("".to_shared_string());
            state.set_info("".to_shared_string());
            state.set_error(e.to_shared_string());
        }
    }
}
