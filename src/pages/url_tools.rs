// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::url_tools;
use crate::{AppWindow, ResultRow, UrlState};

const MODE_PARSE: i32 = 4;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<UrlState>().on_convert(move || {
        let ui = handle.unwrap();
        convert(&ui.global::<UrlState>());
    });

    let handle = app.as_weak();
    app.global::<UrlState>().on_copy_output(move || {
        let ui = handle.unwrap();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(ui.global::<UrlState>().get_output().as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<UrlState>().on_paste_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<UrlState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            state.set_input(text.to_shared_string());
            convert(&state);
        }
    });

    app.global::<UrlState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });
}

fn convert(state: &UrlState) {
    let input = state.get_input();
    let text = input.as_str();
    state.set_output("".to_shared_string());
    state.set_rows(ModelRc::default());
    state.set_error("".to_shared_string());
    if text.trim().is_empty() {
        return;
    }

    let mode = state.get_mode_index();
    if mode == MODE_PARSE {
        match url_tools::parse(text) {
            Ok(rows) => {
                let rows: Vec<ResultRow> = rows
                    .into_iter()
                    .map(|(label, value)| ResultRow {
                        label: label.to_shared_string(),
                        value: value.to_shared_string(),
                    })
                    .collect();
                state.set_rows(ModelRc::new(VecModel::from(rows)));
            }
            Err(e) => state.set_error(e.to_shared_string()),
        }
        return;
    }

    let result = match mode {
        0 => Ok(url_tools::encode_component(text)),
        1 => Ok(url_tools::encode_url(text)),
        2 => url_tools::decode(text, false),
        _ => url_tools::decode(text, true),
    };
    match result {
        Ok(output) => state.set_output(output.to_shared_string()),
        Err(e) => state.set_error(e.to_shared_string()),
    }
}
