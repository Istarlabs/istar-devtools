// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{Color, ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::css_tools::{self, Context};
use crate::{AppWindow, CssToolsState, ResultRow};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<CssToolsState>().on_convert(move || {
        let ui = handle.unwrap();
        convert(&ui.global::<CssToolsState>());
    });

    let handle = app.as_weak();
    app.global::<CssToolsState>().on_swap_colors(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CssToolsState>();
        let foreground = state.get_foreground();
        state.set_foreground(state.get_background());
        state.set_background(foreground);
        convert(&state);
    });

    app.global::<CssToolsState>().on_copy(|text| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.as_str());
        }
    });

    convert(&app.global::<CssToolsState>());
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

fn number(text: &str, what: &str) -> Result<f64, String> {
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("{what} must be a number"))
}

fn convert(state: &CssToolsState) {
    state.set_error("".to_shared_string());

    let result = if state.get_mode_index() == 1 {
        css_tools::contrast(
            state.get_foreground().as_str(),
            state.get_background().as_str(),
        )
        .map(|contrast| {
            let (fr, fg, fb) = contrast.foreground;
            let (br, bg, bb) = contrast.background;
            state.set_foreground_color(Color::from_rgb_u8(fr, fg, fb));
            state.set_background_color(Color::from_rgb_u8(br, bg, bb));
            state.set_contrast_ok(contrast.ratio >= 4.5);
            contrast.rows
        })
    } else {
        (|| {
            let context = Context {
                root_px: number(state.get_root_size().as_str(), "Root size")?,
                parent_px: number(state.get_parent_size().as_str(), "Parent size")?,
                viewport_width_px: number(state.get_viewport_width().as_str(), "Viewport width")?,
                viewport_height_px: number(
                    state.get_viewport_height().as_str(),
                    "Viewport height",
                )?,
            };
            let value = number(state.get_value().as_str(), "Value")?;
            let unit = css_tools::UNITS
                .get(state.get_unit_index().max(0) as usize)
                .copied()
                .unwrap_or("px");
            css_tools::convert_all(value, unit, context)
        })()
    };

    match result {
        Ok(rows) => state.set_rows(to_rows(rows)),
        Err(e) => {
            state.set_rows(ModelRc::default());
            state.set_error(e.to_shared_string());
        }
    }
}
