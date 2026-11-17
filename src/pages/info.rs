// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{AppWindow, LicenseDep, LicenseState};

#[derive(serde::Deserialize)]
struct LicenseEntry {
    name: String,
    version: String,
    license: Option<String>,
    description: Option<String>,
    repository: Option<String>,
}

static LICENSES_JSON: &str = include_str!("../../app_licenses.json");

const EXCLUDED: &[&str] = &["istar-devtools", "calc_engine", "math"];

pub fn register(app: &AppWindow) {
    let entries: Vec<LicenseEntry> = serde_json::from_str(LICENSES_JSON).unwrap_or_default();

    let mut deps: Vec<LicenseDep> = entries
        .into_iter()
        .filter(|e| !EXCLUDED.contains(&e.name.as_str()))
        .map(|e| LicenseDep {
            name: e.name.into(),
            version: e.version.into(),
            license: e.license.unwrap_or_default().into(),
            description: e.description.unwrap_or_default().into(),
            repository: e.repository.unwrap_or_default().into(),
        })
        .collect();

    deps.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));

    app.global::<LicenseState>().set_version_text(
        format!("v{} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_COMMIT_ID")).into(),
    );

    app.global::<LicenseState>()
        .set_deps(ModelRc::new(VecModel::from(deps)));

    app.global::<LicenseState>().on_open_url(|url| {
        let _ = std::process::Command::new("xdg-open")
            .arg(url.as_str())
            .spawn();
    });
}
