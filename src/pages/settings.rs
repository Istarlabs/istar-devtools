// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{config, AppWindow, SettingsState};
use slint::ComponentHandle;

pub fn register(app: &AppWindow) {
    let settings = app.global::<SettingsState>();
    settings.on_set_language(|index| {
        let code = match index {
            1 => "de",
            2 => "es",
            3 => "ja",
            _ => "en",
        };
        let _ = slint::select_bundled_translation(code);

        let mut cfg = config::AppConfig::load();
        cfg.language = index;
        cfg.save();
    });
}
