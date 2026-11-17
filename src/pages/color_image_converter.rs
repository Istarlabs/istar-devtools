// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::rc::Rc;

use css_color_parser2::Color as CssColor;
use image::{DynamicImage, ImageFormat, imageops::FilterType};
use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, ColorImageConverterState};

/// CSS3 named colors (name, r, g, b) for nearest-name lookup.
const NAMED_COLORS: &[(&str, u8, u8, u8)] = &[
    ("aliceblue", 240, 248, 255),
    ("antiquewhite", 250, 235, 215),
    ("aqua", 0, 255, 255),
    ("aquamarine", 127, 255, 212),
    ("azure", 240, 255, 255),
    ("beige", 245, 245, 220),
    ("bisque", 255, 228, 196),
    ("black", 0, 0, 0),
    ("blanchedalmond", 255, 235, 205),
    ("blue", 0, 0, 255),
    ("blueviolet", 138, 43, 226),
    ("brown", 165, 42, 42),
    ("burlywood", 222, 184, 135),
    ("cadetblue", 95, 158, 160),
    ("chartreuse", 127, 255, 0),
    ("chocolate", 210, 105, 30),
    ("coral", 255, 127, 80),
    ("cornflowerblue", 100, 149, 237),
    ("cornsilk", 255, 248, 220),
    ("crimson", 220, 20, 60),
    ("cyan", 0, 255, 255),
    ("darkblue", 0, 0, 139),
    ("darkcyan", 0, 139, 139),
    ("darkgoldenrod", 184, 134, 11),
    ("darkgray", 169, 169, 169),
    ("darkgreen", 0, 100, 0),
    ("darkgrey", 169, 169, 169),
    ("darkkhaki", 189, 183, 107),
    ("darkmagenta", 139, 0, 139),
    ("darkolivegreen", 85, 107, 47),
    ("darkorange", 255, 140, 0),
    ("darkorchid", 153, 50, 204),
    ("darkred", 139, 0, 0),
    ("darksalmon", 233, 150, 122),
    ("darkseagreen", 143, 188, 143),
    ("darkslateblue", 72, 61, 139),
    ("darkslategray", 47, 79, 79),
    ("darkslategrey", 47, 79, 79),
    ("darkturquoise", 0, 206, 209),
    ("darkviolet", 148, 0, 211),
    ("deeppink", 255, 20, 147),
    ("deepskyblue", 0, 191, 255),
    ("dimgray", 105, 105, 105),
    ("dimgrey", 105, 105, 105),
    ("dodgerblue", 30, 144, 255),
    ("firebrick", 178, 34, 34),
    ("floralwhite", 255, 250, 240),
    ("forestgreen", 34, 139, 34),
    ("fuchsia", 255, 0, 255),
    ("gainsboro", 220, 220, 220),
    ("ghostwhite", 248, 248, 255),
    ("gold", 255, 215, 0),
    ("goldenrod", 218, 165, 32),
    ("gray", 128, 128, 128),
    ("green", 0, 128, 0),
    ("greenyellow", 173, 255, 47),
    ("grey", 128, 128, 128),
    ("honeydew", 240, 255, 240),
    ("hotpink", 255, 105, 180),
    ("indianred", 205, 92, 92),
    ("indigo", 75, 0, 130),
    ("ivory", 255, 255, 240),
    ("khaki", 240, 230, 140),
    ("lavender", 230, 230, 250),
    ("lavenderblush", 255, 240, 245),
    ("lawngreen", 124, 252, 0),
    ("lemonchiffon", 255, 250, 205),
    ("lightblue", 173, 216, 230),
    ("lightcoral", 240, 128, 128),
    ("lightcyan", 224, 255, 255),
    ("lightgoldenrodyellow", 250, 250, 210),
    ("lightgray", 211, 211, 211),
    ("lightgreen", 144, 238, 144),
    ("lightgrey", 211, 211, 211),
    ("lightpink", 255, 182, 193),
    ("lightsalmon", 255, 160, 122),
    ("lightseagreen", 32, 178, 170),
    ("lightskyblue", 135, 206, 250),
    ("lightslategray", 119, 136, 153),
    ("lightslategrey", 119, 136, 153),
    ("lightsteelblue", 176, 196, 222),
    ("lightyellow", 255, 255, 224),
    ("lime", 0, 255, 0),
    ("limegreen", 50, 205, 50),
    ("linen", 250, 240, 230),
    ("magenta", 255, 0, 255),
    ("maroon", 128, 0, 0),
    ("mediumaquamarine", 102, 205, 170),
    ("mediumblue", 0, 0, 205),
    ("mediumorchid", 186, 85, 211),
    ("mediumpurple", 147, 112, 219),
    ("mediumseagreen", 60, 179, 113),
    ("mediumslateblue", 123, 104, 238),
    ("mediumspringgreen", 0, 250, 154),
    ("mediumturquoise", 72, 209, 204),
    ("mediumvioletred", 199, 21, 133),
    ("midnightblue", 25, 25, 112),
    ("mintcream", 245, 255, 250),
    ("mistyrose", 255, 228, 225),
    ("moccasin", 255, 228, 181),
    ("navajowhite", 255, 222, 173),
    ("navy", 0, 0, 128),
    ("oldlace", 253, 245, 230),
    ("olive", 128, 128, 0),
    ("olivedrab", 107, 142, 35),
    ("orange", 255, 165, 0),
    ("orangered", 255, 69, 0),
    ("orchid", 218, 112, 214),
    ("palegoldenrod", 238, 232, 170),
    ("palegreen", 152, 251, 152),
    ("paleturquoise", 175, 238, 238),
    ("palevioletred", 219, 112, 147),
    ("papayawhip", 255, 239, 213),
    ("peachpuff", 255, 218, 185),
    ("peru", 205, 133, 63),
    ("pink", 255, 192, 203),
    ("plum", 221, 160, 221),
    ("powderblue", 176, 224, 230),
    ("purple", 128, 0, 128),
    ("red", 255, 0, 0),
    ("rosybrown", 188, 143, 143),
    ("royalblue", 65, 105, 225),
    ("saddlebrown", 139, 69, 19),
    ("salmon", 250, 128, 114),
    ("sandybrown", 244, 164, 96),
    ("seagreen", 46, 139, 87),
    ("seashell", 255, 245, 238),
    ("sienna", 160, 82, 45),
    ("silver", 192, 192, 192),
    ("skyblue", 135, 206, 235),
    ("slateblue", 106, 90, 205),
    ("slategray", 112, 128, 144),
    ("slategrey", 112, 128, 144),
    ("snow", 255, 250, 250),
    ("springgreen", 0, 255, 127),
    ("steelblue", 70, 130, 180),
    ("tan", 210, 180, 140),
    ("teal", 0, 128, 128),
    ("thistle", 216, 191, 216),
    ("tomato", 255, 99, 71),
    ("turquoise", 64, 224, 208),
    ("violet", 238, 130, 238),
    ("wheat", 245, 222, 179),
    ("white", 255, 255, 255),
    ("whitesmoke", 245, 245, 245),
    ("yellow", 255, 255, 0),
    ("yellowgreen", 154, 205, 50),
];

pub fn register(app: &AppWindow) {
    // Cache the most recently loaded image so converting does not re-read the file.
    let loaded_image: Rc<RefCell<Option<DynamicImage>>> = Rc::new(RefCell::new(None));

    let handle = app.as_weak();
    app.global::<ColorImageConverterState>()
        .on_parse_color(move || {
            let ui = handle.unwrap();
            let state = ui.global::<ColorImageConverterState>();
            let input = state.get_color_input();

            match input.as_str().trim().parse::<CssColor>() {
                Ok(color) => {
                    let (r, g, b) = (color.r, color.g, color.b);
                    state.set_color_hex(format!("#{r:02x}{g:02x}{b:02x}").to_shared_string());
                    state.set_color_rgb(format!("rgb({r}, {g}, {b})").to_shared_string());
                    state.set_color_hsl(format_hsl(r, g, b).to_shared_string());
                    state.set_color_hsv(format_hsv(r, g, b).to_shared_string());
                    state.set_color_cmyk(format_cmyk(r, g, b).to_shared_string());
                    state.set_color_css_name(nearest_css_name(r, g, b).to_shared_string());
                    state.set_color_swatch(slint::Color::from_rgb_u8(r, g, b));
                    state.set_color_valid(true);
                    state.set_color_error("".to_shared_string());
                }
                Err(_) => {
                    state.set_color_valid(false);
                    state.set_color_error(
                        "Could not parse color. Use #hex, rgb(), hsl() or a CSS name."
                            .to_shared_string(),
                    );
                }
            }
        });

    let handle = app.as_weak();
    app.global::<ColorImageConverterState>()
        .on_copy_color(move |index| {
            let ui = handle.unwrap();
            let state = ui.global::<ColorImageConverterState>();
            let text = match index {
                0 => state.get_color_hex(),
                1 => state.get_color_rgb(),
                2 => state.get_color_hsl(),
                3 => state.get_color_hsv(),
                4 => state.get_color_cmyk(),
                _ => state.get_color_css_name(),
            };
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text.as_str());
            }
        });

    let handle = app.as_weak();
    let cache = loaded_image.clone();
    app.global::<ColorImageConverterState>()
        .on_open_image(move || {
            let ui = handle.unwrap();
            let state = ui.global::<ColorImageConverterState>();

            let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                .pick_file()
            else {
                return;
            };

            match image::open(&path) {
                Ok(img) => {
                    let info = format!(
                        "{}  ({} × {})",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                        img.width(),
                        img.height()
                    );
                    state.set_resize_width(img.width().to_string().into());
                    state.set_resize_height(img.height().to_string().into());
                    *cache.borrow_mut() = Some(img);
                    state.set_image_path(path.to_string_lossy().to_shared_string());
                    state.set_image_info(info.to_shared_string());
                    state.set_has_image(true);
                    state.set_image_status("".to_shared_string());
                }
                Err(e) => {
                    state.set_has_image(false);
                    state.set_image_status(format!("Failed to load: {e}").to_shared_string());
                }
            }
        });

    let handle = app.as_weak();
    let cache = loaded_image.clone();
    app.global::<ColorImageConverterState>()
        .on_convert_image(move || {
            let ui = handle.unwrap();
            let state = ui.global::<ColorImageConverterState>();

            let guard = cache.borrow();
            let Some(source) = guard.as_ref() else {
                state.set_image_status("No image loaded".to_shared_string());
                return;
            };

            let mut img = source.clone();

            if state.get_resize_enabled() {
                let w = state.get_resize_width().as_str().trim().parse::<u32>().ok();
                let h = state.get_resize_height().as_str().trim().parse::<u32>().ok();
                match (w, h) {
                    (Some(w), Some(h)) if w > 0 && h > 0 => {
                        img = if state.get_keep_aspect_ratio() {
                            img.resize(w, h, FilterType::Lanczos3)
                        } else {
                            img.resize_exact(w, h, FilterType::Lanczos3)
                        };
                    }
                    _ => {
                        state.set_image_status(
                            "Resize width/height must be positive integers".to_shared_string(),
                        );
                        return;
                    }
                }
            }

            let (format, ext) = match state.get_output_format_index() {
                1 => (ImageFormat::Jpeg, "jpg"),
                2 => (ImageFormat::WebP, "webp"),
                3 => (ImageFormat::Bmp, "bmp"),
                _ => (ImageFormat::Png, "png"),
            };

            let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &[ext])
                .set_file_name(format!("converted.{ext}"))
                .save_file()
            else {
                return;
            };

            let result = if format == ImageFormat::Jpeg {
                save_jpeg(&img, &path, state.get_jpeg_quality() as u8)
            } else {
                img.save_with_format(&path, format).map_err(|e| e.to_string())
            };

            match result {
                Ok(()) => state.set_image_status(
                    format!("Saved to {}", path.to_string_lossy()).to_shared_string(),
                ),
                Err(e) => state.set_image_status(format!("Save failed: {e}").to_shared_string()),
            }
        });
}

fn save_jpeg(img: &DynamicImage, path: &PathBuf, quality: u8) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality.clamp(1, 100));
    // JPEG has no alpha channel; flatten to RGB8.
    encoder
        .encode_image(&img.to_rgb8())
        .map_err(|e| e.to_string())
}

fn rgb_to_unit(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

fn hue(r: f64, g: f64, b: f64, max: f64, delta: f64) -> f64 {
    if delta == 0.0 {
        return 0.0;
    }
    let h = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (h * 60.0).rem_euclid(360.0)
}

fn format_hsl(r: u8, g: u8, b: u8) -> String {
    let (r, g, b) = rgb_to_unit(r, g, b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = hue(r, g, b, max, delta);
    format!(
        "hsl({}, {}%, {}%)",
        h.round() as i32,
        (s * 100.0).round() as i32,
        (l * 100.0).round() as i32
    )
}

fn format_hsv(r: u8, g: u8, b: u8) -> String {
    let (r, g, b) = rgb_to_unit(r, g, b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = hue(r, g, b, max, delta);
    format!(
        "hsv({}, {}%, {}%)",
        h.round() as i32,
        (s * 100.0).round() as i32,
        (max * 100.0).round() as i32
    )
}

fn format_cmyk(r: u8, g: u8, b: u8) -> String {
    let (r, g, b) = rgb_to_unit(r, g, b);
    let k = 1.0 - r.max(g).max(b);
    let (c, m, y) = if (1.0 - k).abs() < f64::EPSILON {
        (0.0, 0.0, 0.0)
    } else {
        (
            (1.0 - r - k) / (1.0 - k),
            (1.0 - g - k) / (1.0 - k),
            (1.0 - b - k) / (1.0 - k),
        )
    };
    format!(
        "cmyk({}%, {}%, {}%, {}%)",
        (c * 100.0).round() as i32,
        (m * 100.0).round() as i32,
        (y * 100.0).round() as i32,
        (k * 100.0).round() as i32
    )
}

fn nearest_css_name(r: u8, g: u8, b: u8) -> String {
    let mut best = NAMED_COLORS[0];
    let mut best_distance = u32::MAX;
    for &(name, nr, ng, nb) in NAMED_COLORS {
        let dr = r as i32 - nr as i32;
        let dg = g as i32 - ng as i32;
        let db = b as i32 - nb as i32;
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best_distance = distance;
            best = (name, nr, ng, nb);
        }
    }
    if best_distance == 0 {
        best.0.to_string()
    } else {
        format!("{} (nearest)", best.0)
    }
}
