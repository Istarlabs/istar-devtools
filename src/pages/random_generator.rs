// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use rand::Rng;
use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, RandomGeneratorState};

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<RandomGeneratorState>().on_generate(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<RandomGeneratorState>();

        let (output, error) = if state.get_mode_index() == 0 {
            generate_text(
                state.get_text_length().to_string(),
                state.get_text_count().to_string(),
                state.get_use_uppercase(),
                state.get_use_lowercase(),
                state.get_use_digits(),
                state.get_use_special(),
            )
        } else {
            generate_numbers(
                state.get_number_count().to_string(),
                state.get_num_min().to_string(),
                state.get_num_max().to_string(),
                state.get_number_type_index() == 1,
                state.get_decimal_places().to_string(),
            )
        };

        state.set_output(output.to_shared_string());
        state.set_error(error.to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<RandomGeneratorState>().on_copy_output(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<RandomGeneratorState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn generate_text(
    length_str: String,
    count_str: String,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    special: bool,
) -> (String, String) {
    let length = match length_str.trim().parse::<usize>() {
        Ok(n) if n > 0 => n.min(10_000),
        _ => return (String::new(), "Invalid length.".into()),
    };
    let count = match count_str.trim().parse::<usize>() {
        Ok(n) if n > 0 => n.min(1_000),
        _ => return (String::new(), "Invalid count.".into()),
    };

    let mut pool: Vec<u8> = Vec::new();
    if uppercase { pool.extend(b'A'..=b'Z'); }
    if lowercase { pool.extend(b'a'..=b'z'); }
    if digits    { pool.extend(b'0'..=b'9'); }
    if special   { pool.extend((0x21u8..=0x7E).filter(|b| !b.is_ascii_alphanumeric())); }

    if pool.is_empty() {
        return (String::new(), "Select at least one character set.".into());
    }

    let mut rng = rand::thread_rng();
    let lines: Vec<String> = (0..count)
        .map(|_| (0..length).map(|_| pool[rng.gen_range(0..pool.len())] as char).collect())
        .collect();

    (lines.join("\n"), String::new())
}

fn generate_numbers(
    count_str: String,
    min_str: String,
    max_str: String,
    is_float: bool,
    decimals_str: String,
) -> (String, String) {
    let count = match count_str.trim().parse::<usize>() {
        Ok(n) if n > 0 => n.min(10_000),
        _ => return (String::new(), "Invalid count.".into()),
    };

    let mut rng = rand::thread_rng();

    if is_float {
        let min = match min_str.trim().parse::<f64>() {
            Ok(n) => n,
            Err(_) => return (String::new(), "Invalid minimum.".into()),
        };
        let max = match max_str.trim().parse::<f64>() {
            Ok(n) => n,
            Err(_) => return (String::new(), "Invalid maximum.".into()),
        };
        if min >= max {
            return (String::new(), "Minimum must be < maximum.".into());
        }
        let decimals = decimals_str.trim().parse::<usize>().unwrap_or(2).min(10);
        let lines: Vec<String> = (0..count)
            .map(|_| format!("{:.prec$}", rng.gen_range(min..max), prec = decimals))
            .collect();
        (lines.join("\n"), String::new())
    } else {
        let min = match min_str.trim().parse::<i64>() {
            Ok(n) => n,
            Err(_) => return (String::new(), "Invalid minimum.".into()),
        };
        let max = match max_str.trim().parse::<i64>() {
            Ok(n) => n,
            Err(_) => return (String::new(), "Invalid maximum.".into()),
        };
        if min > max {
            return (String::new(), "Minimum must be <= maximum.".into());
        }
        let lines: Vec<String> = (0..count)
            .map(|_| rng.gen_range(min..=max).to_string())
            .collect();
        (lines.join("\n"), String::new())
    }
}
