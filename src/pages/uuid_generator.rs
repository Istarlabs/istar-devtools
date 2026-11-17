// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::OnceLock;

use rand::RngCore;
use slint::{ComponentHandle, ToSharedString};
use uuid::{Timestamp, Uuid};
use uuid::timestamp::context::NoContext;

use crate::{AppWindow, UuidGeneratorState};

static NODE_ID: OnceLock<[u8; 6]> = OnceLock::new();

fn node_id() -> &'static [u8; 6] {
    NODE_ID.get_or_init(|| {
        let mut id = [0u8; 6];
        rand::thread_rng().fill_bytes(&mut id);
        id
    })
}

pub fn register(app: &AppWindow) {
    let state = app.global::<UuidGeneratorState>();
    state.set_version_description(version_description(2).to_shared_string());

    let ui_handle = app.as_weak();
    app.global::<UuidGeneratorState>().on_version_changed(move |idx| {
        let ui = ui_handle.unwrap();
        ui.global::<UuidGeneratorState>()
            .set_version_description(version_description(idx).to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<UuidGeneratorState>().on_generate(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<UuidGeneratorState>();

        let count = state
            .get_count()
            .as_str()
            .trim()
            .parse::<usize>()
            .unwrap_or(1)
            .max(1)
            .min(100);

        let version_index = state.get_version_index();
        let namespace = namespace_at(state.get_namespace_index());
        let name = state.get_namespace_name().to_string();

        let mut lines: Vec<String> = Vec::with_capacity(count);
        let mut error = String::new();

        for _ in 0..count {
            match generate_uuid(version_index, namespace, &name) {
                Ok(uuid) => lines.push(uuid.hyphenated().to_string()),
                Err(e) => {
                    error = e;
                    break;
                }
            }
        }

        state.set_error(error.to_shared_string());
        state.set_output(lines.join("\n").to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<UuidGeneratorState>().on_copy_all(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<UuidGeneratorState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });

    let ui_handle = app.as_weak();
    app.global::<UuidGeneratorState>().on_copy_first(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<UuidGeneratorState>();
        let first = state
            .get_output()
            .to_string()
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(&first) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn generate_uuid(version_index: i32, namespace: Uuid, name: &str) -> Result<Uuid, String> {
    match version_index {
        0 => Ok(Uuid::new_v1(Timestamp::now(NoContext), node_id())),
        1 => {
            if name.is_empty() {
                return Err("Name required for v3.".into());
            }
            Ok(Uuid::new_v3(&namespace, name.as_bytes()))
        }
        2 => Ok(Uuid::new_v4()),
        3 => {
            if name.is_empty() {
                return Err("Name required for v5.".into());
            }
            Ok(Uuid::new_v5(&namespace, name.as_bytes()))
        }
        4 => Ok(Uuid::new_v6(Timestamp::now(NoContext), node_id())),
        5 => Ok(Uuid::now_v7()),
        _ => Err("Unknown version.".into()),
    }
}

fn namespace_at(idx: i32) -> Uuid {
    match idx {
        0 => Uuid::NAMESPACE_DNS,
        1 => Uuid::NAMESPACE_URL,
        2 => Uuid::NAMESPACE_OID,
        3 => Uuid::NAMESPACE_X500,
        _ => Uuid::NAMESPACE_DNS,
    }
}

fn version_description(idx: i32) -> &'static str {
    match idx {
        0 => "v1: Based on the current timestamp and a node identifier.",
        1 => "v3: Deterministic. MD5 hash of a namespace UUID and a name string.",
        2 => "v4: Randomly generated. 122 bits of cryptographic randomness.",
        3 => "v5: Deterministic. SHA-1 hash of a namespace UUID and a name string.",
        4 => "v6: Like v1 but with reordered timestamp bytes for better database locality.",
        5 => "v7: Based on the current Unix timestamp with random bits. Monotonically sortable.",
        _ => "",
    }
}
