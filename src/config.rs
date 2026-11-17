// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::AppPages;

impl AppPages {
    pub fn to_index(self) -> i32 {
        match self {
            AppPages::Home => 0,
            AppPages::Info => 1,
            AppPages::Settings => 2,
            AppPages::Base64 => 3,
            AppPages::TextInspector => 4,
            AppPages::HashGenerator => 5,
            AppPages::Chmod => 6,
            AppPages::Html => 7,
            AppPages::SystemInfo => 8,
            AppPages::NumberBase => 9,
            AppPages::UnitConverter => 10,
            AppPages::Calculator => 11,
            AppPages::VectorCalculator => 12,
            AppPages::LoremIpsum => 13,
            AppPages::UuidGenerator => 14,
            AppPages::RandomGenerator => 15,
            AppPages::RegexTester => 16,
            AppPages::TextDiffer => 17,
            AppPages::Cron => 18,
            AppPages::QrCode => 19,
            AppPages::Jwt => 20,
            AppPages::SchemaValidator => 21,
            AppPages::ColorImage => 22,
            AppPages::Formatter => 23,
            AppPages::Systemd => 24,
            AppPages::Timestamp => 25,
            AppPages::Url => 26,
            AppPages::FormatConverter => 27,
            AppPages::PathQuery => 28,
            AppPages::Cidr => 29,
            AppPages::Certificate => 30,
            AppPages::Security => 31,
            AppPages::Escape => 32,
            AppPages::Markdown => 33,
            AppPages::SqlFormatter => 34,
            AppPages::HexViewer => 35,
            AppPages::IdTools => 36,
            AppPages::CssTools => 37,
            AppPages::Compression => 38,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index {
            1 => AppPages::Info,
            2 => AppPages::Settings,
            3 => AppPages::Base64,
            4 => AppPages::TextInspector,
            5 => AppPages::HashGenerator,
            6 => AppPages::Chmod,
            7 => AppPages::Html,
            8 => AppPages::SystemInfo,
            9 => AppPages::NumberBase,
            10 => AppPages::UnitConverter,
            11 => AppPages::Calculator,
            12 => AppPages::VectorCalculator,
            13 => AppPages::LoremIpsum,
            14 => AppPages::UuidGenerator,
            15 => AppPages::RandomGenerator,
            16 => AppPages::RegexTester,
            17 => AppPages::TextDiffer,
            18 => AppPages::Cron,
            19 => AppPages::QrCode,
            20 => AppPages::Jwt,
            21 => AppPages::SchemaValidator,
            22 => AppPages::ColorImage,
            23 => AppPages::Formatter,
            24 => AppPages::Systemd,
            25 => AppPages::Timestamp,
            26 => AppPages::Url,
            27 => AppPages::FormatConverter,
            28 => AppPages::PathQuery,
            29 => AppPages::Cidr,
            30 => AppPages::Certificate,
            31 => AppPages::Security,
            32 => AppPages::Escape,
            33 => AppPages::Markdown,
            34 => AppPages::SqlFormatter,
            35 => AppPages::HexViewer,
            36 => AppPages::IdTools,
            37 => AppPages::CssTools,
            38 => AppPages::Compression,
            _ => AppPages::Home,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub language: i32,
    pub last_page: i32,
    /// Names of the collapsed navigation groups.
    #[serde(default)]
    pub collapsed_groups: Vec<String>,
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("istar-devtools").join("config.json"))
    }

    /// Where the settings lived while the program was called dev_tools; read once when the new file is not there yet.
    fn legacy_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("dev_tools").join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let content = std::fs::read_to_string(&path).or_else(|_| {
            Self::legacy_config_path().map_or(
                Err(std::io::ErrorKind::NotFound.into()),
                std::fs::read_to_string,
            )
        });
        let Ok(content) = content else {
            return Self::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }
}
