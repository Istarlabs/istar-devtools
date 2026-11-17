// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, SharedString, ToSharedString, VecModel};

use crate::units::byte_unit_converter::ByteUnit;
use crate::units::unit_converter::Category;
use crate::{AppWindow, ByteConverterState, UnitConverterState};

pub fn register(app: &AppWindow) {
    set_unit_names(app, Category::Length);
    register_byte_converter(app);

    let ui_handle = app.as_weak();
    app.global::<UnitConverterState>()
        .on_category_changed(move |idx| {
            let ui = ui_handle.unwrap();
            let category = category_at(idx);
            set_unit_names(&ui, category);

            let state = ui.global::<UnitConverterState>();
            state.set_from_unit_index(0);
            state.set_to_unit_index(1);
            state.set_input("".to_shared_string());
            state.set_output("".to_shared_string());
        });

    let ui_handle = app.as_weak();
    app.global::<UnitConverterState>().on_convert(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<UnitConverterState>();
        let input_str = state.get_input();

        let result = if input_str.is_empty() {
            String::new()
        } else {
            match input_str.parse::<f64>() {
                Ok(value) => {
                    let category = category_at(state.get_selected_category());
                    let from = state.get_from_unit_index() as usize;
                    let to = state.get_to_unit_index() as usize;

                    category
                        .convert(value, from, to)
                        .map(format_result)
                        .unwrap_or_default()
                }
                Err(_) => String::new(),
            }
        };

        state.set_output(result.to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<UnitConverterState>().on_swap_units(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<UnitConverterState>();

        let from = state.get_from_unit_index();
        let to = state.get_to_unit_index();
        let previous_output = state.get_output().to_string();

        state.set_from_unit_index(to);
        state.set_to_unit_index(from);
        state.set_input(previous_output.to_shared_string());
        state.invoke_convert();
    });

    let ui_handle = app.as_weak();
    app.global::<UnitConverterState>().on_copy_output(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<UnitConverterState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn register_byte_converter(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<ByteConverterState>().on_convert(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<ByteConverterState>();
        let input_str = state.get_input();

        let result = if input_str.is_empty() {
            String::new()
        } else {
            match input_str.parse::<f64>() {
                Ok(value) => {
                    let from = byte_unit_at(state.get_from_unit_index());
                    let to = byte_unit_at(state.get_to_unit_index());
                    match (from, to) {
                        (Some(f), Some(t)) => {
                            let bytes = value * f.factor() as f64;
                            let result = bytes / t.factor() as f64;
                            format_result(result)
                        }
                        _ => String::new(),
                    }
                }
                Err(_) => String::new(),
            }
        };

        state.set_output(result.to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<ByteConverterState>().on_swap_units(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<ByteConverterState>();

        let from = state.get_from_unit_index();
        let to = state.get_to_unit_index();
        let previous_output = state.get_output().to_string();

        state.set_from_unit_index(to);
        state.set_to_unit_index(from);
        state.set_input(previous_output.to_shared_string());
        state.invoke_convert();
    });

    let ui_handle = app.as_weak();
    app.global::<ByteConverterState>().on_copy_output(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<ByteConverterState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn byte_unit_at(idx: i32) -> Option<ByteUnit> {
    match idx {
        0 => Some(ByteUnit::Byte),
        1 => Some(ByteUnit::Kilobyte),
        2 => Some(ByteUnit::Megabyte),
        3 => Some(ByteUnit::Gigabyte),
        4 => Some(ByteUnit::Terabyte),
        5 => Some(ByteUnit::Petabyte),
        6 => Some(ByteUnit::Exabyte),
        7 => Some(ByteUnit::Kibibyte),
        8 => Some(ByteUnit::Mebibyte),
        9 => Some(ByteUnit::Gibibyte),
        10 => Some(ByteUnit::Tebibyte),
        11 => Some(ByteUnit::Pebibyte),
        12 => Some(ByteUnit::Exbibyte),
        _ => None,
    }
}

fn category_at(idx: i32) -> Category {
    Category::ALL
        .get(idx as usize)
        .copied()
        .unwrap_or(Category::Length)
}

fn set_unit_names(app: &AppWindow, category: Category) {
    let names: Vec<SharedString> = category
        .units()
        .iter()
        .map(|u| format!("{} ({})", u.name, u.symbol).into())
        .collect();

    app.global::<UnitConverterState>()
        .set_unit_names(ModelRc::new(VecModel::from(names)));
}

fn format_result(value: f64) -> String {
    if !value.is_finite() {
        return String::new();
    }
    let s = format!("{:.10}", value);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
