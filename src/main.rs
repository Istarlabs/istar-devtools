// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

slint::include_modules!();
mod config;
mod nav_search;
mod pages;
mod units;

fn main() -> Result<(), slint::PlatformError> {
    let saved = config::AppConfig::load();

    let app_window = AppWindow::new()?;
    // Plasma on Wayland finds a window's icon through its app id, the name of its desktop file
    // (net.kurokyuu.devtools.desktop). Slint needs it before the window is shown.
    slint::set_xdg_app_id("net.kurokyuu.devtools")?;

    let lang_code = match saved.language {
        1 => "de",
        2 => "es",
        3 => "ja",
        _ => "en",
    };
    let _ = slint::select_bundled_translation(lang_code);
    app_window.global::<SettingsState>().set_current_language(saved.language);
    app_window.set_current_page(AppPages::from_index(saved.last_page));

    app_window.on_page_selected(|page| {
        let mut cfg = config::AppConfig::load();
        cfg.last_page = page.to_index();
        cfg.save();
    });

    pages::base64::register(&app_window);
    pages::text_case::register(&app_window);
    pages::hash_generator::register(&app_window);
    pages::chmod_notation::register(&app_window);
    pages::html_encode_decode::register(&app_window);
    pages::system_info::register(&app_window);
    pages::number_base::register(&app_window);
    pages::unit_converter::register(&app_window);
    pages::calculator::register(&app_window);
    pages::vector_calculator::register(&app_window);
    pages::lorem_ipsum::register(&app_window);
    pages::uuid_generator::register(&app_window);
    pages::random_generator::register(&app_window);
    pages::cron::register(&app_window);
    pages::qr_code::register(&app_window);
    pages::regex_tester::register(&app_window);
    pages::text_differ::register(&app_window);
    pages::jwt::register(&app_window);
    pages::schema_validator::register(&app_window);
    pages::formatter::register(&app_window);
    pages::systemd::register(&app_window);
    pages::color_image_converter::register(&app_window);
    pages::timestamp::register(&app_window);
    pages::url_tools::register(&app_window);
    pages::format_converter::register(&app_window);
    pages::path_query::register(&app_window);
    pages::cidr::register(&app_window);
    pages::certificate::register(&app_window);
    pages::security::register(&app_window);
    pages::string_escape::register(&app_window);
    pages::markdown_preview::register(&app_window);
    pages::sql_formatter::register(&app_window);
    pages::hex_viewer::register(&app_window);
    pages::id_tools::register(&app_window);
    pages::css_tools::register(&app_window);
    pages::compression::register(&app_window);
    nav_search::register(&app_window);
    pages::info::register(&app_window);
    pages::settings::register(&app_window);
    app_window.run()?;

    Ok(())
}
