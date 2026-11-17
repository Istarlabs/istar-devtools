// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Utc;
use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::id_tools;
use crate::units::timestamp::{self, Zone};
use crate::{AppWindow, IdToolsState, ResultRow};

const MODE_ULID: i32 = 0;
const MODE_NANOID: i32 = 1;
const MAX_COUNT: usize = 100;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<IdToolsState>().on_generate(move || {
        let ui = handle.unwrap();
        generate(&ui.global::<IdToolsState>());
    });

    let handle = app.as_weak();
    app.global::<IdToolsState>().on_decode(move || {
        let ui = handle.unwrap();
        decode(&ui.global::<IdToolsState>());
    });

    let handle = app.as_weak();
    app.global::<IdToolsState>()
        .on_snowflake_from_time(move || {
            let ui = handle.unwrap();
            from_time(&ui.global::<IdToolsState>());
        });

    app.global::<IdToolsState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });
}

fn to_rows(pairs: Vec<(String, String)>) -> ModelRc<ResultRow> {
    let rows: Vec<ResultRow> = pairs
        .into_iter()
        .map(|(label, value)| ResultRow {
            label: label.to_shared_string(),
            value: value.to_shared_string(),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn numbered(values: Vec<String>) -> Vec<(String, String)> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| (format!("#{}", index + 1), value))
        .collect()
}

fn generate(state: &IdToolsState) {
    state.set_error("".to_shared_string());
    state.set_note("".to_shared_string());

    let result = (|| -> Result<(Vec<(String, String)>, String), String> {
        let count = state
            .get_count()
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|count| (1..=MAX_COUNT).contains(count))
            .ok_or_else(|| format!("Count must be between 1 and {MAX_COUNT}"))?;
        let mut rng = rand::rngs::OsRng;

        match state.get_mode_index() {
            MODE_ULID => {
                let now = Utc::now().timestamp_millis() as u64;
                Ok((
                    numbered(id_tools::generate_ulids(count, now, &mut rng)),
                    String::new(),
                ))
            }
            MODE_NANOID => {
                let length = state
                    .get_nanoid_length()
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "Length must be a number".to_string())?;
                let custom = state.get_nanoid_custom();
                let alphabet = match state.get_nanoid_alphabet_index() {
                    0 => id_tools::NANOID_URL_SAFE,
                    1 => id_tools::NANOID_ALPHANUMERIC,
                    2 => id_tools::NANOID_LOWERCASE,
                    3 => id_tools::NANOID_DIGITS,
                    4 => id_tools::NANOID_HEX,
                    _ => custom.as_str(),
                };
                let ids = (0..count)
                    .map(|_| id_tools::generate_nanoid(alphabet, length, &mut rng))
                    .collect::<Result<Vec<_>, _>>()?;
                let bits = id_tools::nanoid_entropy_bits(alphabet, length);
                Ok((numbered(ids), format!("≈ {bits:.0} bits of entropy per ID")))
            }
            _ => Ok((Vec::new(), String::new())),
        }
    })();

    match result {
        Ok((rows, note)) => {
            state.set_rows(to_rows(rows));
            state.set_note(note.to_shared_string());
        }
        Err(e) => {
            state.set_rows(ModelRc::default());
            state.set_error(e.to_shared_string());
        }
    }
}

fn epoch(state: &IdToolsState) -> Result<i64, String> {
    id_tools::snowflake_epoch(
        state.get_snowflake_epoch_index(),
        state.get_snowflake_custom_epoch().as_str(),
    )
}

fn decode(state: &IdToolsState) {
    state.set_error("".to_shared_string());
    state.set_decoded(ModelRc::default());

    let result = if state.get_mode_index() == MODE_ULID {
        let input = state.get_ulid_input();
        if input.trim().is_empty() {
            return;
        }
        id_tools::ulid_rows(input.as_str())
    } else {
        let input = state.get_snowflake_input();
        if input.trim().is_empty() {
            return;
        }
        epoch(state).and_then(|epoch| id_tools::snowflake_rows(input.as_str(), epoch))
    };
    match result {
        Ok(rows) => state.set_decoded(to_rows(rows)),
        Err(e) => state.set_error(e.to_shared_string()),
    }
}

fn from_time(state: &IdToolsState) {
    state.set_error("".to_shared_string());
    let input = state.get_snowflake_time();
    let result = (|| {
        let instant = if input.trim().is_empty() {
            Utc::now()
        } else {
            timestamp::parse_unix(input.as_str())
                .or_else(|| timestamp::parse_date(input.as_str(), Zone::Utc))
                .ok_or_else(|| "Not a Unix timestamp or a recognised date (UTC)".to_string())?
        };
        id_tools::snowflake_for_time(instant.timestamp_millis(), epoch(state)?)
    })();
    match result {
        Ok(id) => {
            state.set_rows(to_rows(vec![(
                "First ID at that time".to_string(),
                id.to_string(),
            )]));
            state.set_snowflake_input(id.to_shared_string());
            decode(state);
        }
        Err(e) => state.set_error(e.to_shared_string()),
    }
}
