// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, ChmodState};
use slint::{ComponentHandle, ToSharedString};

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<ChmodState>().on_calculate(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<ChmodState>();

        let owner_read = state.get_set_owner_read_checked();
        let owner_write = state.get_set_owner_write_checked();
        let owner_execute = state.get_set_owner_execute_checked();
        let group_read = state.get_set_group_read_checked();
        let group_write = state.get_set_group_write_checked();
        let group_execute = state.get_set_group_execute_checked();
        let other_read = state.get_set_other_read_checked();
        let other_write = state.get_set_other_write_checked();
        let other_execute = state.get_set_other_execute_checked();
        let set_uid = state.get_set_uid_checked();
        let set_gid = state.get_set_gid_checked();
        let sticky = state.get_set_sticy_checked();

        let numeric_value = (owner_read as i32 * 400)
            + (owner_write as i32 * 200)
            + (owner_execute as i32 * 100)
            + (group_read as i32 * 40)
            + (group_write as i32 * 20)
            + (group_execute as i32 * 10)
            + (other_read as i32 * 4)
            + (other_write as i32 * 2)
            + (other_execute as i32 * 1)
            + (set_uid as i32 * 4000)
            + (set_gid as i32 * 2000)
            + (sticky as i32 * 1000);

        let buf: [u8; 9] = [
            if owner_read { b'r' } else { b'-' },
            if owner_write { b'w' } else { b'-' },
            match (set_uid, owner_execute) {
                (true, true) => b's',
                (true, false) => b'S',
                (false, true) => b'x',
                (false, false) => b'-',
            },
            if group_read { b'r' } else { b'-' },
            if group_write { b'w' } else { b'-' },
            match (set_gid, group_execute) {
                (true, true) => b's',
                (true, false) => b'S',
                (false, true) => b'x',
                (false, false) => b'-',
            },
            if other_read { b'r' } else { b'-' },
            if other_write { b'w' } else { b'-' },
            match (sticky, other_execute) {
                (true, true) => b't',
                (true, false) => b'T',
                (false, true) => b'x',
                (false, false) => b'-',
            },
        ];

        state.set_numeric_notation(numeric_value);
        state.set_symbolic_notation(String::from_utf8_lossy(&buf).to_shared_string());
    });
}
