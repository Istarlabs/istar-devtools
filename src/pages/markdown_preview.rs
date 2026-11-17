// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ModelRc, ToSharedString, VecModel};

use crate::units::markdown_preview;
use crate::{AppWindow, MarkdownState, MdBlock};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<MarkdownState>().on_render(move || {
        let ui = handle.unwrap();
        render(&ui.global::<MarkdownState>());
    });

    let handle = app.as_weak();
    app.global::<MarkdownState>().on_paste_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<MarkdownState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            state.set_input(text.to_shared_string());
            render(&state);
        }
    });

    let handle = app.as_weak();
    app.global::<MarkdownState>().on_open_file(move || {
        let ui = handle.unwrap();
        let state = ui.global::<MarkdownState>();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                state.set_input(text.to_shared_string());
                render(&state);
            }
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    });

    let handle = app.as_weak();
    app.global::<MarkdownState>().on_copy_html(move || {
        let ui = handle.unwrap();
        let html = markdown_preview::to_html(ui.global::<MarkdownState>().get_input().as_str());
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(html);
        }
    });

    let handle = app.as_weak();
    app.global::<MarkdownState>().on_open_in_browser(move || {
        let ui = handle.unwrap();
        let page = markdown_preview::html_page(ui.global::<MarkdownState>().get_input().as_str());
        let path = std::env::temp_dir().join("istar-devtools-markdown-preview.html");
        if std::fs::write(&path, page).is_ok() {
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
        }
    });
}

fn render(state: &MarkdownState) {
    let blocks: Vec<MdBlock> = markdown_preview::blocks(state.get_input().as_str())
        .into_iter()
        .map(|block| MdBlock {
            kind: block.kind as i32,
            level: block.level,
            text: block.text.to_shared_string(),
        })
        .collect();
    state.set_blocks(ModelRc::new(VecModel::from(blocks)));
}
