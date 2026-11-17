// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use regex::RegexBuilder;
use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, RegexTesterState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<RegexTesterState>().on_run(move || {
        let ui = handle.unwrap();
        let state = ui.global::<RegexTesterState>();

        let pattern = state.get_pattern();
        let pattern = pattern.as_str();

        if pattern.is_empty() {
            state.set_error("".to_shared_string());
            state.set_output("".to_shared_string());
            state.set_match_label("".to_shared_string());
            return;
        }

        let re = RegexBuilder::new(pattern)
            .case_insensitive(state.get_flag_case_insensitive())
            .multi_line(state.get_flag_multiline())
            .dot_matches_new_line(state.get_flag_dot_all())
            .build();

        match re {
            Err(err) => {
                state.set_error(err.to_string().to_shared_string());
                state.set_output("".to_shared_string());
                state.set_match_label("".to_shared_string());
            }
            Ok(re) => {
                state.set_error("".to_shared_string());

                let input = state.get_test_input();
                let input = input.as_str();

                if state.get_replace_mode() {
                    let replacement = state.get_replacement();
                    let result = re.replace_all(input, replacement.as_str());
                    state.set_output(result.into_owned().to_shared_string());
                    state.set_match_label("Replace preview".to_shared_string());
                } else {
                    let captures: Vec<_> = re.captures_iter(input).collect();
                    let count = captures.len();

                    let label = match count {
                        0 => "No matches".to_string(),
                        1 => "1 match found".to_string(),
                        n => format!("{n} matches found"),
                    };

                    let output = captures
                        .iter()
                        .enumerate()
                        .map(|(i, cap)| {
                            let m = cap.get(0).unwrap();
                            let mut s = format!(
                                "[{}] {:?}  (offset: {}–{})",
                                i,
                                m.as_str(),
                                m.start(),
                                m.end()
                            );
                            for g in 1..cap.len() {
                                match cap.get(g) {
                                    Some(gm) => {
                                        s.push_str(&format!("\n    Group {g}: {:?}", gm.as_str()))
                                    }
                                    None => s.push_str(&format!("\n    Group {g}: <no match>")),
                                }
                            }
                            s
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    state.set_match_label(label.to_shared_string());
                    state.set_output(output.to_shared_string());
                }
            }
        }
    });

    let handle = app.as_weak();
    app.global::<RegexTesterState>().on_copy_output(move || {
        let ui = handle.unwrap();
        let state = ui.global::<RegexTesterState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });
}
