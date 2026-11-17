// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use quick_xml::events::Event;
use quick_xml::{Reader, Writer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, FormatterState};

const MODE_JSON: i32 = 0;
const MODE_XML: i32 = 1;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<FormatterState>().on_format(move || {
        let ui = handle.unwrap();
        let state = ui.global::<FormatterState>();
        run(&state, Action::Format);
    });

    let handle = app.as_weak();
    app.global::<FormatterState>().on_minify(move || {
        let ui = handle.unwrap();
        let state = ui.global::<FormatterState>();
        run(&state, Action::Minify);
    });

    let handle = app.as_weak();
    app.global::<FormatterState>().on_copy_output(move || {
        let ui = handle.unwrap();
        let state = ui.global::<FormatterState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(state.get_output().as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<FormatterState>().on_paste_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<FormatterState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                state.set_input(text.to_shared_string());
            }
        }
    });

    let handle = app.as_weak();
    app.global::<FormatterState>().on_use_output_as_input(move || {
        let ui = handle.unwrap();
        let state = ui.global::<FormatterState>();
        state.set_input(state.get_output());
        state.set_output("".to_shared_string());
    });
}

#[derive(Clone, Copy)]
enum Action {
    Format,
    Minify,
}

#[derive(Clone, Copy)]
enum Indent {
    Spaces(usize),
    Tab,
}

impl Indent {
    fn from_index(index: i32) -> Self {
        match index {
            1 => Indent::Spaces(4),
            2 => Indent::Tab,
            _ => Indent::Spaces(2),
        }
    }

    fn unit(self) -> (u8, usize) {
        match self {
            Indent::Spaces(n) => (b' ', n),
            Indent::Tab => (b'\t', 1),
        }
    }
}

fn run(state: &FormatterState, action: Action) {
    let input = state.get_input();
    let indent = Indent::from_index(state.get_indent_index());

    let result = match state.get_mode() {
        MODE_JSON => format_json(input.as_str(), indent, action),
        MODE_XML => format_xml(input.as_str(), indent, action),
        _ => format_yaml(input.as_str(), action),
    };

    match result {
        Ok(text) => {
            state.set_output(text.to_shared_string());
            state.set_error("".to_shared_string());
        }
        Err(e) => {
            state.set_output("".to_shared_string());
            state.set_error(e.to_shared_string());
        }
    }
}

fn format_json(input: &str, indent: Indent, action: Action) -> Result<String, String> {
    let value: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;

    match action {
        Action::Minify => serde_json::to_string(&value).map_err(|e| e.to_string()),
        Action::Format => {
            let (byte, count) = indent.unit();
            let indent_bytes = vec![byte; count];
            let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_bytes);
            let mut buffer = Vec::new();
            let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
            value.serialize(&mut serializer).map_err(|e| e.to_string())?;
            String::from_utf8(buffer).map_err(|e| e.to_string())
        }
    }
}

fn format_xml(input: &str, indent: Indent, action: Action) -> Result<String, String> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    let mut writer = match action {
        Action::Format => {
            let (byte, count) = indent.unit();
            Writer::new_with_indent(Vec::new(), byte, count)
        }
        Action::Minify => Writer::new(Vec::new()),
    };

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(event) => writer.write_event(event).map_err(|e| e.to_string())?,
            Err(e) => {
                let line = line_at_offset(input, reader.error_position() as usize);
                return Err(format!("line {line}: {e}"));
            }
        }
    }

    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

pub(super) fn format_yaml_document(input: &str) -> Result<String, String> {
    format_yaml(input, Action::Format)
}

fn format_yaml(input: &str, action: Action) -> Result<String, String> {
    if matches!(action, Action::Minify) {
        return Err("Minify is not supported for YAML".to_string());
    }

    // Multi-document streams are formatted document by document.
    let mut documents = Vec::new();
    for document in serde_norway::Deserializer::from_str(input) {
        let value = serde_norway::Value::deserialize(document).map_err(|e| e.to_string())?;
        documents.push(serde_norway::to_string(&value).map_err(|e| e.to_string())?);
    }
    Ok(documents.join("---\n"))
}

fn line_at_offset(text: &str, offset: usize) -> usize {
    let clamped = offset.min(text.len());
    text[..clamped].bytes().filter(|&b| b == b'\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_keeps_key_order_and_indent() {
        let out = format_json(r#"{"b":1,"a":[1,2]}"#, Indent::Spaces(4), Action::Format).unwrap();
        assert_eq!(out, "{\n    \"b\": 1,\n    \"a\": [\n        1,\n        2\n    ]\n}");
        let min = format_json(&out, Indent::Tab, Action::Minify).unwrap();
        assert_eq!(min, r#"{"b":1,"a":[1,2]}"#);
    }

    #[test]
    fn xml_format_and_minify() {
        let out = format_xml("<a><b>x</b><c/></a>", Indent::Spaces(2), Action::Format).unwrap();
        assert_eq!(out, "<a>\n  <b>x</b>\n  <c/>\n</a>");
        let min = format_xml(&out, Indent::Spaces(2), Action::Minify).unwrap();
        assert_eq!(min, "<a><b>x</b><c/></a>");
    }

    #[test]
    fn yaml_multi_document() {
        let out = format_yaml("a:   1\nb: [1,2]\n---\nc: 3\n", Action::Format).unwrap();
        assert_eq!(out, "a: 1\nb:\n- 1\n- 2\n---\nc: 3\n");
        assert!(format_yaml("a: [", Action::Format).is_err());
    }
}
