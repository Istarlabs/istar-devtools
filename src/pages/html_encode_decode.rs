// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, HtmlEncodeDecodeState};
use slint::{ComponentHandle, ToSharedString};

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<HtmlEncodeDecodeState>().on_convert(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<HtmlEncodeDecodeState>();

        if state.get_encode_mode() {
            let input = state.get_input();
            let encoded = html_escape::encode_text(input.as_str());
            state.set_output(encoded.to_shared_string());
        } else {
            let mut output = String::new();
            let decoded = html_escape::decode_html_entities_to_string(
                state.get_input().as_str(),
                &mut output,
            );

            state.set_output(decoded.to_shared_string());
        }
    });

    let ui_handle = app.as_weak();
    app.global::<HtmlEncodeDecodeState>()
        .on_paste_input(move || {
            let ui = ui_handle.unwrap();
            let state = ui.global::<HtmlEncodeDecodeState>();
            let mut clipboard = arboard::Clipboard::new().unwrap();
            state.set_input(clipboard.get_text().unwrap_or_default().to_shared_string());
            state.invoke_convert();
        });

    let ui_handle = app.as_weak();
    app.global::<HtmlEncodeDecodeState>()
        .on_copy_output(move || {
            let ui = ui_handle.unwrap();
            let state = ui.global::<HtmlEncodeDecodeState>();
            let mut clipboard = arboard::Clipboard::new().unwrap();

            if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
                eprintln!("Clipboard error: {}", e);
            }
        });

    let ui_handle = app.as_weak();
    app.global::<HtmlEncodeDecodeState>()
        .on_open_from_file(move || {
            let ui = ui_handle.unwrap();
            let state = ui.global::<HtmlEncodeDecodeState>();

            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select a file to import from.")
                .pick_file()
            {
                let content = std::fs::read_to_string(path).unwrap_or_else(|e| e.to_string());
                state.set_input(content.into());
                state.invoke_convert();
            }
        });
}
