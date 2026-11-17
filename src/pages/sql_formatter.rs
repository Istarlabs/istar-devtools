// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ToSharedString};

use crate::units::sql_formatter::{self, KeywordCase};
use crate::{AppWindow, SqlFormatterState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<SqlFormatterState>().on_format(move || {
        let ui = handle.unwrap();
        let state = ui.global::<SqlFormatterState>();
        let formatted = sql_formatter::format(
            state.get_input().as_str(),
            sql_formatter::indent_from_index(state.get_indent_index()),
            KeywordCase::from_index(state.get_case_index()),
            sql_formatter::dialect_from_index(state.get_dialect_index()),
        );
        state.set_output(formatted.to_shared_string());
    });

    let handle = app.as_weak();
    app.global::<SqlFormatterState>().on_minify(move || {
        let ui = handle.unwrap();
        let state = ui.global::<SqlFormatterState>();
        let minified = sql_formatter::minify(state.get_input().as_str());
        state.set_output(minified.to_shared_string());
    });

    let handle = app.as_weak();
    app.global::<SqlFormatterState>().on_copy_output(move || {
        let ui = handle.unwrap();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(ui.global::<SqlFormatterState>().get_output().as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<SqlFormatterState>().on_paste_input(move || {
        let ui = handle.unwrap();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            ui.global::<SqlFormatterState>()
                .set_input(text.to_shared_string());
        }
    });

    let handle = app.as_weak();
    app.global::<SqlFormatterState>()
        .on_use_output_as_input(move || {
            let ui = handle.unwrap();
            let state = ui.global::<SqlFormatterState>();
            state.set_input(state.get_output());
            state.set_output("".to_shared_string());
        });
}
