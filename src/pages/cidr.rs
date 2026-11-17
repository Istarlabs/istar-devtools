// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::cidr;
use crate::{AppWindow, CidrState, ResultRow};

const SUBNET_LIMIT: usize = 64;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<CidrState>().on_calculate(move || {
        let ui = handle.unwrap();
        calculate(&ui.global::<CidrState>());
    });

    app.global::<CidrState>().on_copy(|text| {
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

fn calculate(state: &CidrState) {
    state.set_rows(ModelRc::default());
    state.set_subnets(ModelRc::default());
    state.set_subnets_note("".to_shared_string());
    state.set_contains_result("".to_shared_string());
    state.set_error("".to_shared_string());

    let input = state.get_network();
    if input.trim().is_empty() {
        return;
    }
    let network = match cidr::parse_network(input.as_str()) {
        Ok(network) => network,
        Err(e) => return state.set_error(e.to_shared_string()),
    };
    state.set_rows(to_rows(cidr::describe(network)));

    let check = state.get_check_address();
    if !check.trim().is_empty() {
        match cidr::parse_address(check.as_str()) {
            Ok(address) => {
                let inside = network.contains(address);
                state.set_contains_ok(inside);
                let verdict = if inside {
                    "✓ inside this network"
                } else {
                    "✗ outside this network"
                };
                state.set_contains_result(verdict.to_shared_string());
            }
            Err(e) => return state.set_error(e.to_shared_string()),
        }
    }

    let split = state.get_split_prefix();
    if !split.trim().is_empty() {
        let result = split
            .trim()
            .trim_start_matches('/')
            .parse::<u32>()
            .map_err(|_| "The split prefix must be a number".to_string())
            .and_then(|prefix| cidr::split(network, prefix, SUBNET_LIMIT));
        match result {
            Ok((subnets, total)) => {
                let shown = subnets.len();
                let pairs = subnets
                    .into_iter()
                    .enumerate()
                    .map(|(index, subnet)| (format!("#{}", index + 1), subnet))
                    .collect();
                state.set_subnets(to_rows(pairs));
                state.set_subnets_note(
                    format!("{total} subnet(s) in total, showing {shown}").to_shared_string(),
                );
            }
            Err(e) => state.set_error(e.to_shared_string()),
        }
    }
}
