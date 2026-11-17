// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::path_query;
use crate::{AppWindow, PathQueryState, ResultRow};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<PathQueryState>().on_run(move || {
        let ui = handle.unwrap();
        run(&ui.global::<PathQueryState>());
    });

    let handle = app.as_weak();
    app.global::<PathQueryState>().on_paste_document(move || {
        let ui = handle.unwrap();
        let state = ui.global::<PathQueryState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            state.set_document(text.to_shared_string());
            run(&state);
        }
    });

    app.global::<PathQueryState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });
}

fn run(state: &PathQueryState) {
    state.set_rows(ModelRc::default());
    state.set_summary("".to_shared_string());
    state.set_error("".to_shared_string());

    let document = state.get_document();
    let expression = state.get_expression();
    if document.trim().is_empty() || expression.trim().is_empty() {
        return;
    }

    let result = if state.get_mode_index() == 1 {
        path_query::xpath(document.as_str(), expression.trim())
    } else {
        path_query::jsonpath(document.as_str(), expression.trim())
    };

    match result {
        Ok(matches) => {
            state.set_summary(format!("{} match(es)", matches.len()).to_shared_string());
            let rows: Vec<ResultRow> = matches
                .into_iter()
                .map(|(label, value)| ResultRow {
                    label: label.to_shared_string(),
                    value: value.replace('\n', " ").to_shared_string(),
                })
                .collect();
            state.set_rows(ModelRc::new(VecModel::from(rows)));
        }
        Err(e) => state.set_error(e.to_shared_string()),
    }
}
