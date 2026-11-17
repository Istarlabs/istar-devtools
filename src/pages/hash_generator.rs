// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, HashGeneratorState};
use adler::Adler32;
use crc::{CRC_32_ISO_HDLC, Crc};
use md5;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};
use sha3::{Sha3_256, Sha3_512};
use slint::{ComponentHandle, ToSharedString};
#[repr(i32)]
#[derive(Copy, Clone)]
enum HashGeneratorOption {
    Md5 = 0,
    Sha1 = 1,
    Sha256 = 2,
    Sha512 = 3,
    Sha3256 = 4,
    Sha3512 = 5,
    Adler = 6,
    Crc = 7,
}

impl HashGeneratorOption {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Md5),
            1 => Some(Self::Sha1),
            2 => Some(Self::Sha256),
            3 => Some(Self::Sha512),
            4 => Some(Self::Sha3256),
            5 => Some(Self::Sha3512),
            6 => Some(Self::Adler),
            7 => Some(Self::Crc),
            _ => None,
        }
    }
}

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<HashGeneratorState>().on_convert(move |idx| {
        let ui = ui_handle.unwrap();
        let state = ui.global::<HashGeneratorState>();
        let input = state.get_input();

        let converted = match HashGeneratorOption::from_i32(idx) {
            Some(HashGeneratorOption::Md5) => {
                format!("{:x}", md5::compute(input.as_bytes()))
            }
            Some(HashGeneratorOption::Sha1) => hex::encode(Sha1::digest(input.as_bytes())),
            Some(HashGeneratorOption::Sha256) => hex::encode(Sha256::digest(input.as_bytes())),
            Some(HashGeneratorOption::Sha512) => hex::encode(Sha512::digest(input.as_bytes())),
            Some(HashGeneratorOption::Sha3256) => hex::encode(Sha3_256::digest(input.as_bytes())),
            Some(HashGeneratorOption::Sha3512) => hex::encode(Sha3_512::digest(input.as_bytes())),
            Some(HashGeneratorOption::Adler) => {
                let mut adler = Adler32::new();
                adler.write_slice(input.as_bytes());
                format!("{:08x}", adler.checksum())
            }
            Some(HashGeneratorOption::Crc) => {
                let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
                format!("{:08x}", crc.checksum(input.as_bytes()))
            }
            None => String::new(),
        };

        state.set_output(converted.into());
    });

    let ui_handle = app.as_weak();
    app.global::<HashGeneratorState>().on_paste_input(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<HashGeneratorState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        state.set_input(clipboard.get_text().unwrap_or_default().to_shared_string());
    });

    let ui_handle = app.as_weak();
    app.global::<HashGeneratorState>().on_copy_output(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<HashGeneratorState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });

    let ui_handle = app.as_weak();
    app.global::<HashGeneratorState>()
        .on_open_from_file(move || {
            let ui = ui_handle.unwrap();
            let state = ui.global::<HashGeneratorState>();

            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select a file to import from")
                .pick_file()
            {
                let content = std::fs::read_to_string(path).unwrap_or_else(|e| e.to_string());
                state.set_input(content.into());
            }
        });
}
