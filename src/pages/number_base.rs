// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::num::ParseIntError;

use slint::ComponentHandle;

use crate::{AppWindow, NumberBaseState, NumberBaseType};

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<NumberBaseState>().on_copy_field(move |field_type| {
        let ui = ui_handle.unwrap();
        let data = ui.global::<NumberBaseState>().get_data();
        let text = match field_type {
            NumberBaseType::Decimal => data.decimal.as_str().to_owned(),
            NumberBaseType::Binary => data.binary.as_str().to_owned(),
            NumberBaseType::HexaDecimal => data.hex.as_str().to_owned(),
            NumberBaseType::Octal => data.octal.as_str().to_owned(),
            NumberBaseType::Ascii => data.ascii.as_str().to_owned(),
            NumberBaseType::Utf8 => data.utf8.as_str().to_owned(),
        };
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(&text) {
            eprintln!("Clipboard error: {e}");
        }
    });

    let ui_handle = app.as_weak();
    app.global::<NumberBaseState>().on_on_input(move |num| {
        let ui = ui_handle.unwrap();
        let state = ui.global::<NumberBaseState>();
        let mut data = state.get_data();

        let result: Result<i64, ParseIntError> = match num {
            crate::NumberBaseType::Decimal => i64::from_str_radix(&data.decimal.as_str(), 10),
            crate::NumberBaseType::Binary => i64::from_str_radix(&data.binary.as_str(), 2),
            crate::NumberBaseType::HexaDecimal => i64::from_str_radix(&data.hex, 16),
            crate::NumberBaseType::Octal => i64::from_str_radix(&data.octal, 8),
            crate::NumberBaseType::Ascii => todo!(),
            crate::NumberBaseType::Utf8 => todo!(),
        };

        match result {
            Ok(val) => {
                data.decimal = format!("{:}", val).into();
                data.hex = format!("{:x}", val).into();
                data.binary = format!("{:b}", val).into();
                data.octal = format!("{:o}", val).into();
            }
            Err(_) => todo!(),
        }

        state.set_data(data);
    });
}
