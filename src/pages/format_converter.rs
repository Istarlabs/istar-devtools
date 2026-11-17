// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ToSharedString};

use crate::units::format_converter::{self, Format};
use crate::{AppWindow, FormatConverterState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<FormatConverterState>().on_convert(move || {
        let ui = handle.unwrap();
        convert(&ui.global::<FormatConverterState>());
    });

    let handle = app.as_weak();
    app.global::<FormatConverterState>().on_swap(move || {
        let ui = handle.unwrap();
        let state = ui.global::<FormatConverterState>();
        let source = state.get_source_index();
        state.set_source_index(state.get_target_index());
        state.set_target_index(source);
        if !state.get_output().is_empty() {
            state.set_input(state.get_output());
        }
        convert(&state);
    });

    let handle = app.as_weak();
    app.global::<FormatConverterState>()
        .on_copy_output(move || {
            let ui = handle.unwrap();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ =
                    clipboard.set_text(ui.global::<FormatConverterState>().get_output().as_str());
            }
        });

    let handle = app.as_weak();
    app.global::<FormatConverterState>()
        .on_paste_input(move || {
            let ui = handle.unwrap();
            let state = ui.global::<FormatConverterState>();
            if let Ok(mut clipboard) = arboard::Clipboard::new()
                && let Ok(text) = clipboard.get_text()
            {
                state.set_input(text.to_shared_string());
                convert(&state);
            }
        });
}

fn convert(state: &FormatConverterState) {
    let input = state.get_input();
    if input.trim().is_empty() {
        state.set_output("".to_shared_string());
        state.set_error("".to_shared_string());
        return;
    }

    let result = format_converter::convert(
        input.as_str(),
        Format::from_index(state.get_source_index()),
        Format::from_index(state.get_target_index()),
        format_converter::delimiter_from_index(state.get_delimiter_index()),
    );
    match result {
        Ok(text) => {
            state.set_output(text.to_shared_string());
            state.set_error("".to_shared_string());
        }
        Err(e) => {
            state.set_output("".to_shared_string());
            state.set_error(e.to_shared_string());
        }
    }
}
