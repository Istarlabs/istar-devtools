// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::Value;
use uppsala::XsdValidator;
use slint::{ComponentHandle, ModelRc, SharedString, ToSharedString, VecModel};

use crate::{AppWindow, SchemaValidatorState};

const MODE_JSON: i32 = 0;
const MODE_XML: i32 = 1;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<SchemaValidatorState>().on_validate(move || {
        let ui = handle.unwrap();
        let state = ui.global::<SchemaValidatorState>();

        match state.get_mode() {
            MODE_JSON => validate_json(&state),
            MODE_XML => validate_xml(&state),
            _ => validate_yaml(&state),
        }
    });

    let handle = app.as_weak();
    app.global::<SchemaValidatorState>()
        .on_pretty_print_document(move || {
            let ui = handle.unwrap();
            let state = ui.global::<SchemaValidatorState>();

            let result = match state.get_mode() {
                MODE_JSON => {
                    let doc = state.get_json_document();
                    pretty_print_json(doc.as_str()).map(|p| state.set_json_document(p.to_shared_string()))
                }
                MODE_XML => {
                    let doc = state.get_xml_document();
                    pretty_print_xml(doc.as_str()).map(|p| state.set_xml_document(p.to_shared_string()))
                }
                _ => {
                    let doc = state.get_yaml_document();
                    super::formatter::format_yaml_document(doc.as_str())
                        .map(|p| state.set_yaml_document(p.to_shared_string()))
                }
            };
            match result {
                Ok(()) => state.set_parse_error("".to_shared_string()),
                Err(e) => state.set_parse_error(e.to_shared_string()),
            }
        });
}

fn set_errors(state: &SchemaValidatorState, errors: Vec<SharedString>) {
    state.set_errors(ModelRc::new(VecModel::from(errors)));
}

fn validate_json(state: &SchemaValidatorState) {
    let document_text = state.get_json_document();
    let schema_text = state.get_json_schema();

    let document: Value = match serde_json::from_str(document_text.as_str()) {
        Ok(value) => value,
        Err(e) => return report_parse_error(state, format!("Document JSON: {e}")),
    };

    let schema: Value = match serde_json::from_str(schema_text.as_str()) {
        Ok(value) => value,
        Err(e) => return report_parse_error(state, format!("Schema JSON: {e}")),
    };

    validate_against_json_schema(state, &document, &schema);
}

fn validate_yaml(state: &SchemaValidatorState) {
    let document_text = state.get_yaml_document();
    let schema_text = state.get_yaml_schema();

    let document: Value = match serde_norway::from_str(document_text.as_str()) {
        Ok(value) => value,
        Err(e) => return report_parse_error(state, format!("Document YAML: {e}")),
    };

    // The schema may be written as YAML or JSON (JSON is a subset of YAML).
    let schema: Value = match serde_norway::from_str(schema_text.as_str()) {
        Ok(value) => value,
        Err(e) => return report_parse_error(state, format!("Schema: {e}")),
    };

    validate_against_json_schema(state, &document, &schema);
}

fn report_parse_error(state: &SchemaValidatorState, message: String) {
    state.set_has_result(false);
    state.set_parse_error(message.to_shared_string());
}

fn validate_against_json_schema(state: &SchemaValidatorState, document: &Value, schema: &Value) {
    let validator = match jsonschema::validator_for(schema) {
        Ok(validator) => validator,
        Err(e) => return report_parse_error(state, format!("Invalid schema: {e}")),
    };

    state.set_parse_error("".to_shared_string());

    let messages: Vec<SharedString> = validator
        .iter_errors(document)
        .map(|error| {
            let path = error.instance_path().to_string();
            let location = if path.is_empty() {
                "(root)".to_string()
            } else {
                path
            };
            format!("{location}: {error}").to_shared_string()
        })
        .collect();

    state.set_is_valid(messages.is_empty());
    set_errors(state, messages);
    state.set_has_result(true);
}

fn validate_xml(state: &SchemaValidatorState) {
    let document_text = state.get_xml_document();
    let xsd_text = state.get_xsd_schema();

    let messages: Vec<SharedString> = xml_validation_errors(document_text.as_str(), xsd_text.as_str())
        .into_iter()
        .map(|m| m.to_shared_string())
        .collect();

    state.set_parse_error("".to_shared_string());
    state.set_is_valid(messages.is_empty());
    set_errors(state, messages);
    state.set_has_result(true);
}

/// Validate `document` against `xsd`. With an empty XSD only well-formedness
/// is checked.
fn xml_validation_errors(document: &str, xsd: &str) -> Vec<String> {
    let document = match uppsala::parse(document) {
        Ok(document) => document,
        Err(e) => return vec![format!("Document: {e}")],
    };

    if xsd.trim().is_empty() {
        return Vec::new();
    }

    let schema_doc = match uppsala::parse(xsd) {
        Ok(schema_doc) => schema_doc,
        Err(e) => return vec![format!("XSD: {e}")],
    };

    match XsdValidator::from_schema(&schema_doc) {
        Ok(validator) => validator
            .validate(&document)
            .iter()
            .map(|e| e.to_string())
            .collect(),
        Err(e) => vec![format!("XSD: {e}")],
    }
}

fn line_at_offset(text: &str, offset: usize) -> usize {
    let clamped = offset.min(text.len());
    text[..clamped].bytes().filter(|&b| b == b'\n').count() + 1
}

fn pretty_print_json(input: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

fn pretty_print_xml(input: &str) -> Result<String, String> {
    use quick_xml::Writer;

    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

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

#[cfg(test)]
mod tests {
    use super::*;

    const XSD: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="age" type="xs:integer"/>
</xs:schema>"#;

    #[test]
    fn xsd_accepts_valid_and_rejects_invalid() {
        assert!(xml_validation_errors("<age>42</age>", XSD).is_empty());
        assert!(!xml_validation_errors("<age>abc</age>", XSD).is_empty());
        assert!(!xml_validation_errors("<other/>", XSD).is_empty());
    }

    #[test]
    fn xml_without_xsd_checks_well_formedness_only() {
        assert!(xml_validation_errors("<a><b/></a>", "").is_empty());
        assert!(!xml_validation_errors("<a><b></a>", "").is_empty());
    }

    #[test]
    fn broken_xsd_is_reported() {
        assert!(xml_validation_errors("<a/>", "<xs:schema").iter().any(|m| m.starts_with("XSD:")));
    }
}
