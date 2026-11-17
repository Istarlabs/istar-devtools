// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, Base64State};
use base64::{
    Engine, alphabet,
    engine::{self, general_purpose},
};
use slint::{ComponentHandle, ToSharedString};
const CUSTOM_ENGINE_SAFE_URL: engine::GeneralPurpose =
    engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

fn encode_decode(engine: &impl Engine, input: &str, encode: bool) -> String {
    if encode {
        engine.encode(input)
    } else {
        engine
            .decode(input)
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .unwrap_or_else(|_| "Invalid base64".to_string())
    }
}

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<Base64State>().on_convert(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<Base64State>();
        let input = state.get_input();

        let result = if state.get_url_safe() {
            encode_decode(
                &CUSTOM_ENGINE_SAFE_URL,
                input.as_str(),
                state.get_encode_mode(),
            )
        } else {
            encode_decode(
                &general_purpose::STANDARD,
                input.as_str(),
                state.get_encode_mode(),
            )
        };
        state.set_output(result.into());
    });

    let ui_handle = app.as_weak();
    app.global::<Base64State>().on_paste_input(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<Base64State>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        state.set_input(clipboard.get_text().unwrap_or_default().to_shared_string());
        state.invoke_convert();
    });

    let ui_handle = app.as_weak();
    app.global::<Base64State>().on_copy_output(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<Base64State>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });

    let ui_handle = app.as_weak();
    app.global::<Base64State>().on_open_from_file(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<Base64State>();

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select a file to import from")
            .pick_file()
        {
            let content = std::fs::read_to_string(path).unwrap_or_else(|e| e.to_string());
            state.set_input(content.into());
            state.invoke_convert();
        }
    });
}
