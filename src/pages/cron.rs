// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::str::FromStr;

use chrono::Utc;
use croner::Cron;
use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::{AppWindow, CronState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<CronState>().on_parse(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CronState>();
        parse_and_populate(&state);
    });

    let handle = app.as_weak();
    app.global::<CronState>().on_generate_from_builder(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CronState>();

        let expr = format!(
            "{} {} {} {} {}",
            state.get_field_minute(),
            state.get_field_hour(),
            state.get_field_day(),
            state.get_field_month(),
            state.get_field_weekday(),
        );
        state.set_expression(expr.to_shared_string());
        parse_and_populate(&state);
    });

    let handle = app.as_weak();
    app.global::<CronState>().on_copy_expression(move || {
        let ui = handle.unwrap();
        let state = ui.global::<CronState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_expression().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn parse_and_populate(state: &CronState) {
    let expr = state.get_expression();
    let expr = expr.as_str().trim();

    match Cron::from_str(expr) {
        Err(err) => {
            state.set_error(err.to_string().to_shared_string());
            state.set_description("".to_shared_string());
            state.set_next_times(ModelRc::new(VecModel::from(vec![])));
        }
        Ok(cron) => {
            state.set_error("".to_shared_string());
            state.set_description(cron.describe().to_shared_string());

            let now = Utc::now();
            let times: Vec<slint::SharedString> = cron
                .iter_after(now)
                .take(10)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string().to_shared_string())
                .collect();

            state.set_next_times(ModelRc::new(VecModel::from(times)));
        }
    }
}
