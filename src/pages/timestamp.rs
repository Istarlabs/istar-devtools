// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Utc;
use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::timestamp::{self, Zone};
use crate::{AppWindow, ResultRow, TimestampState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<TimestampState>().on_convert(move || {
        let ui = handle.unwrap();
        convert(&ui.global::<TimestampState>());
    });

    let handle = app.as_weak();
    app.global::<TimestampState>().on_now(move || {
        let ui = handle.unwrap();
        let state = ui.global::<TimestampState>();
        state.set_input(Utc::now().timestamp().to_shared_string());
        convert(&state);
    });

    app.global::<TimestampState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });
}

fn convert(state: &TimestampState) {
    let input = state.get_input();
    if input.trim().is_empty() {
        state.set_rows(ModelRc::default());
        state.set_error("".to_shared_string());
        return;
    }

    let index = state.get_zone_index();
    let label = timestamp::ZONES
        .get(index.max(0) as usize)
        .copied()
        .unwrap_or("UTC");

    match timestamp::convert(input.as_str(), Zone::from_index(index), label, Utc::now()) {
        Ok(rows) => {
            let rows: Vec<ResultRow> = rows
                .into_iter()
                .map(|(label, value)| ResultRow {
                    label: label.to_shared_string(),
                    value: value.to_shared_string(),
                })
                .collect();
            state.set_rows(ModelRc::new(VecModel::from(rows)));
            state.set_error("".to_shared_string());
        }
        Err(e) => {
            state.set_rows(ModelRc::default());
            state.set_error(e.to_shared_string());
        }
    }
}
