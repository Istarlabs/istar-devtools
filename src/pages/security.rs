// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::security::{self, HmacAlgorithm, PassphraseOptions, PasswordOptions};
use crate::{AppWindow, ResultRow, SecurityState};

const MODE_HMAC: i32 = 0;
const MODE_PASSWORD: i32 = 1;
const MAX_COUNT: usize = 100;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<SecurityState>().on_generate(move || {
        let ui = handle.unwrap();
        generate(&ui.global::<SecurityState>());
    });

    app.global::<SecurityState>().on_copy(|text| {
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

fn parse_number(text: &str, what: &str) -> Result<usize, String> {
    text.trim()
        .parse::<usize>()
        .map_err(|_| format!("{what} must be a number"))
}

fn generate(state: &SecurityState) {
    state.set_rows(ModelRc::default());
    state.set_note("".to_shared_string());
    state.set_error("".to_shared_string());

    let result = match state.get_mode_index() {
        MODE_HMAC => hmac(state),
        MODE_PASSWORD => passwords(state),
        _ => passphrases(state),
    };
    match result {
        Ok((rows, note)) => {
            state.set_rows(to_rows(rows));
            state.set_note(note.to_shared_string());
        }
        Err(e) => state.set_error(e.to_shared_string()),
    }
}

type Outcome = Result<(Vec<(String, String)>, String), String>;

fn hmac(state: &SecurityState) -> Outcome {
    let message = state.get_hmac_message();
    let key = state.get_hmac_key();
    if message.is_empty() && key.is_empty() {
        return Ok((Vec::new(), String::new()));
    }
    let key_bytes = if state.get_hmac_key_hex() {
        security::from_hex(key.as_str())?
    } else {
        key.as_bytes().to_vec()
    };
    let algorithm = HmacAlgorithm::from_index(state.get_hmac_algorithm_index());
    Ok((
        security::hmac_rows(algorithm, &key_bytes, message.as_bytes()),
        String::new(),
    ))
}

fn count(state: &SecurityState) -> Result<usize, String> {
    let count = parse_number(state.get_count().as_str(), "Count")?;
    if (1..=MAX_COUNT).contains(&count) {
        Ok(count)
    } else {
        Err(format!("Count must be between 1 and {MAX_COUNT}"))
    }
}

fn passwords(state: &SecurityState) -> Outcome {
    let options = PasswordOptions {
        length: parse_number(state.get_password_length().as_str(), "Length")?,
        lower: state.get_password_lower(),
        upper: state.get_password_upper(),
        digits: state.get_password_digits(),
        symbols: state.get_password_symbols(),
        exclude_look_alikes: state.get_password_no_look_alikes(),
    };
    let mut rng = rand::rngs::OsRng;
    let rows = (1..=count(state)?)
        .map(|index| {
            security::generate_password(&options, &mut rng).map(|p| (format!("#{index}"), p))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        rows,
        format!("≈ {:.0} bits of entropy", options.entropy_bits()),
    ))
}

fn passphrases(state: &SecurityState) -> Outcome {
    let options = PassphraseOptions {
        words: parse_number(state.get_passphrase_words().as_str(), "Words")?,
        separator: state.get_passphrase_separator().to_string(),
        capitalize: state.get_passphrase_capitalize(),
        add_digit: state.get_passphrase_digit(),
    };
    let mut rng = rand::rngs::OsRng;
    let rows = (1..=count(state)?)
        .map(|index| {
            security::generate_passphrase(&options, &mut rng).map(|p| (format!("#{index}"), p))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        rows,
        format!(
            "≈ {:.0} bits of entropy ({} word list)",
            security::passphrase_entropy_bits(&options),
            security::wordlist().len()
        ),
    ))
}
