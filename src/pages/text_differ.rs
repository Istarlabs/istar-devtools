// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use similar::{ChangeTag, TextDiff};
use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::{AppWindow, DiffLine, DiffLineKind, TextDifferState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<TextDifferState>().on_diff(move || {
        let ui = handle.unwrap();
        let state = ui.global::<TextDifferState>();

        let left = state.get_left_text();
        let right = state.get_right_text();

        let diff = TextDiff::from_lines(left.as_str(), right.as_str());

        let mut lines: Vec<DiffLine> = Vec::new();

        for group in diff.grouped_ops(3) {
            for op in &group {
                for change in diff.iter_changes(op) {
                    let (kind, prefix) = match change.tag() {
                        ChangeTag::Equal => (DiffLineKind::Equal, " "),
                        ChangeTag::Insert => (DiffLineKind::Added, "+"),
                        ChangeTag::Delete => (DiffLineKind::Removed, "-"),
                    };
                    let content = format!("{prefix} {}", change.value().trim_end_matches('\n'));
                    lines.push(DiffLine { kind, content: content.to_shared_string() });
                }
            }
            lines.push(DiffLine {
                kind: DiffLineKind::Header,
                content: "".to_shared_string(),
            });
        }

        if lines.last().is_some_and(|l| l.content.is_empty()) {
            lines.pop();
        }

        let unified = diff
            .unified_diff()
            .header("Original", "Modified")
            .to_string();

        state.set_unified_output(unified.to_shared_string());
        state.set_diff_lines(ModelRc::new(VecModel::from(lines)));
        state.set_error("".to_shared_string());
    });

    let handle = app.as_weak();
    app.global::<TextDifferState>().on_copy_unified(move || {
        let ui = handle.unwrap();
        let state = ui.global::<TextDifferState>();
        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(state.get_unified_output().as_str()) {
            eprintln!("Clipboard error: {e}");
        }
    });

    let handle = app.as_weak();
    app.global::<TextDifferState>().on_clear(move || {
        let ui = handle.unwrap();
        let state = ui.global::<TextDifferState>();
        state.set_left_text("".to_shared_string());
        state.set_right_text("".to_shared_string());
        state.set_diff_lines(ModelRc::new(VecModel::from(vec![])));
        state.set_unified_output("".to_shared_string());
        state.set_error("".to_shared_string());
    });
}
