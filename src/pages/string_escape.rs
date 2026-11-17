// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ToSharedString};

use crate::units::string_escape::{self, Target};
use crate::{AppWindow, EscapeState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<EscapeState>().on_convert(move || {
        let ui = handle.unwrap();
        convert(&ui.global::<EscapeState>());
    });

    let handle = app.as_weak();
    app.global::<EscapeState>().on_swap(move || {
        let ui = handle.unwrap();
        let state = ui.global::<EscapeState>();
        if !state.get_output().is_empty() {
            state.set_input(state.get_output());
        }
        state.set_unescape_mode(!state.get_unescape_mode());
        convert(&state);
    });

    let handle = app.as_weak();
    app.global::<EscapeState>().on_copy_output(move || {
        let ui = handle.unwrap();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(ui.global::<EscapeState>().get_output().as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<EscapeState>().on_paste_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<EscapeState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            state.set_input(text.to_shared_string());
            convert(&state);
        }
    });
}

fn convert(state: &EscapeState) {
    let input = state.get_input();
    let target = Target::from_index(state.get_target_index());
    let result = if state.get_unescape_mode() {
        string_escape::unescape(input.as_str(), target)
    } else {
        Ok(string_escape::escape(input.as_str(), target))
    };
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
