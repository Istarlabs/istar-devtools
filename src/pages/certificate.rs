// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Utc;
use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::certificate;
use crate::{AppWindow, CertificateState, ResultRow};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<CertificateState>().on_decode(move || {
        let ui = handle.unwrap();
        decode(&ui.global::<CertificateState>());
    });

    let handle = app.as_weak();
    app.global::<CertificateState>().on_paste_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CertificateState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            state.set_input(text.to_shared_string());
            decode(&state);
        }
    });

    let handle = app.as_weak();
    app.global::<CertificateState>().on_open_file(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CertificateState>();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Certificates", &["pem", "crt", "cer", "der", "csr", "key"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                // Text files are PEM already; anything else is taken to be a DER certificate.
                let text = match String::from_utf8(bytes) {
                    Ok(text) if text.contains("-----BEGIN") => text,
                    Ok(text) => certificate::der_to_pem(text.as_bytes()),
                    Err(e) => certificate::der_to_pem(e.as_bytes()),
                };
                state.set_input(text.to_shared_string());
                decode(&state);
            }
            Err(e) => state.set_error(format!("{}: {e}", path.display()).to_shared_string()),
        }
    });

    app.global::<CertificateState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });
}

fn decode(state: &CertificateState) {
    state.set_rows(ModelRc::default());
    state.set_error("".to_shared_string());

    let input = state.get_input();
    if input.trim().is_empty() {
        return;
    }
    match certificate::decode(input.as_str(), Utc::now().timestamp()) {
        Ok(pairs) => {
            let rows: Vec<ResultRow> = pairs
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
}
