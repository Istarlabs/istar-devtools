// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use image::RgbImage;
use qrcode::{Color, EcLevel, QrCode};
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer, ToSharedString};

use crate::{AppWindow, QrCodeState};

const MODULE_PX: usize = 10;
const QUIET_MODULES: usize = 4;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<QrCodeState>().on_generate(move || {
        let ui = handle.unwrap();
        let state = ui.global::<QrCodeState>();

        let input = state.get_input();
        let ec = map_ec_level(state.get_ec_level_index());

        match render_qr(input.as_str(), ec) {
            Ok((slint_image, _, _)) => {
                state.set_qr_image(slint_image);
                state.set_has_image(true);
                state.set_error("".to_shared_string());
            }
            Err(e) => {
                state.set_error(e.to_shared_string());
                state.set_has_image(false);
            }
        }
    });

    let handle = app.as_weak();
    app.global::<QrCodeState>().on_save_png(move || {
        let ui = handle.unwrap();
        let state = ui.global::<QrCodeState>();

        let input = state.get_input();
        let ec = map_ec_level(state.get_ec_level_index());

        let Ok((_, rgb_bytes, size)) = render_qr(input.as_str(), ec) else {
            return;
        };

        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG Image", &["png"])
            .set_file_name("qrcode.png")
            .save_file()
        else {
            return;
        };

        if let Some(img) = RgbImage::from_raw(size, size, rgb_bytes) {
            if let Err(e) = img.save(&path) {
                eprintln!("Save error: {e}");
            }
        }
    });

    let handle = app.as_weak();
    app.global::<QrCodeState>().on_copy_image(move || {
        let ui = handle.unwrap();
        let state = ui.global::<QrCodeState>();

        let input = state.get_input();
        let ec = map_ec_level(state.get_ec_level_index());

        let Ok((_, rgb_bytes, size)) = render_qr(input.as_str(), ec) else {
            return;
        };

        let rgba: Vec<u8> = rgb_bytes
            .chunks(3)
            .flat_map(|c| [c[0], c[1], c[2], 255u8])
            .collect();

        let image_data = arboard::ImageData {
            width: size as usize,
            height: size as usize,
            bytes: rgba.into(),
        };

        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_image(image_data) {
            eprintln!("Clipboard error: {e}");
        }
    });
}

fn map_ec_level(index: i32) -> EcLevel {
    match index {
        0 => EcLevel::L,
        2 => EcLevel::Q,
        3 => EcLevel::H,
        _ => EcLevel::M,
    }
}

fn render_qr(data: &str, ec_level: EcLevel) -> Result<(Image, Vec<u8>, u32), String> {
    if data.is_empty() {
        return Err("Input is empty".to_string());
    }

    let code = QrCode::with_error_correction_level(data.as_bytes(), ec_level)
        .map_err(|e| e.to_string())?;

    let qr_width = code.width();
    let img_size = (qr_width + QUIET_MODULES * 2) * MODULE_PX;
    let colors = code.to_colors();

    let mut buffer = SharedPixelBuffer::<Rgb8Pixel>::new(img_size as u32, img_size as u32);
    let bytes = buffer.make_mut_bytes();
    bytes.iter_mut().for_each(|b| *b = 255);

    for y in 0..qr_width {
        for x in 0..qr_width {
            if colors[y * qr_width + x] == Color::Dark {
                for py in 0..MODULE_PX {
                    for px in 0..MODULE_PX {
                        let img_x = (QUIET_MODULES + x) * MODULE_PX + px;
                        let img_y = (QUIET_MODULES + y) * MODULE_PX + py;
                        let idx = (img_y * img_size + img_x) * 3;
                        bytes[idx] = 0;
                        bytes[idx + 1] = 0;
                        bytes[idx + 2] = 0;
                    }
                }
            }
        }
    }

    let raw = bytes.to_vec();
    let slint_image = Image::from_rgb8(buffer);

    Ok((slint_image, raw, img_size as u32))
}
