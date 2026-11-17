// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conversion between JSON, YAML, TOML, CSV and XML through a shared value tree.
//!
//! XML mapping: an element becomes an object, attributes get an `@` prefix, text next to
//! attributes or children is stored as `#text`, an element with only text becomes a string and an
//! empty element becomes `null`. Repeated sibling elements become an array.

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer, XmlVersion};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
    Toml,
    Csv,
    Xml,
}

impl Format {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Format::Yaml,
            2 => Format::Toml,
            3 => Format::Csv,
            4 => Format::Xml,
            _ => Format::Json,
        }
    }
}

pub fn delimiter_from_index(index: i32) -> u8 {
    match index {
        1 => b';',
        2 => b'\t',
        _ => b',',
    }
}

pub fn convert(input: &str, from: Format, to: Format, delimiter: u8) -> Result<String, String> {
    let value = parse(input, from, delimiter)?;
    render(&value, to, delimiter)
}

pub fn parse(input: &str, format: Format, delimiter: u8) -> Result<Value, String> {
    match format {
        Format::Json => serde_json::from_str(input).map_err(|e| format!("JSON: {e}")),
        Format::Yaml => serde_norway::from_str(input).map_err(|e| format!("YAML: {e}")),
        Format::Toml => toml::from_str(input).map_err(|e| format!("TOML: {e}")),
        Format::Csv => parse_csv(input, delimiter),
        Format::Xml => parse_xml(input),
    }
}

pub fn render(value: &Value, format: Format, delimiter: u8) -> Result<String, String> {
    match format {
        Format::Json => serde_json::to_string_pretty(value).map_err(|e| format!("JSON: {e}")),
        Format::Yaml => serde_norway::to_string(value).map_err(|e| format!("YAML: {e}")),
        Format::Toml => {
            if !value.is_object() {
                return Err("TOML: the document root must be an object (table)".to_string());
            }
            toml::to_string_pretty(value).map_err(|e| format!("TOML: {e} (TOML has no null)"))
        }
        Format::Csv => render_csv(value, delimiter),
        Format::Xml => render_xml(value),
    }
}

/// CSV cells that look like numbers, booleans or `null` become typed values.
fn infer(cell: &str) -> Value {
    match cell {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        "null" => return Value::Null,
        _ => {}
    }
    if let Ok(integer) = cell.parse::<i64>()
        && integer.to_string() == cell
    {
        return Value::from(integer);
    }
    if let Ok(float) = cell.parse::<f64>()
        && cell.contains('.')
        && !cell.ends_with('.')
        && !cell.starts_with('.')
        && let Some(number) = serde_json::Number::from_f64(float)
    {
        return Value::Number(number);
    }
    Value::String(cell.to_string())
}

fn parse_csv(input: &str, delimiter: u8) -> Result<Value, String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(input.as_bytes());
    let headers = reader.headers().map_err(|e| format!("CSV: {e}"))?.clone();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| format!("CSV: {e}"))?;
        let mut object = Map::new();
        for (header, cell) in headers.iter().zip(record.iter()) {
            object.insert(header.to_string(), infer(cell));
        }
        rows.push(Value::Object(object));
    }
    Ok(Value::Array(rows))
}

fn cell_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn render_csv(value: &Value, delimiter: u8) -> Result<String, String> {
    let rows = value
        .as_array()
        .ok_or("CSV: the document must be an array of objects")?;
    let mut headers: Vec<&str> = Vec::new();
    for row in rows {
        let object = row
            .as_object()
            .ok_or("CSV: every array item must be an object")?;
        for key in object.keys() {
            if !headers.contains(&key.as_str()) {
                headers.push(key);
            }
        }
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());
    writer
        .write_record(&headers)
        .map_err(|e| format!("CSV: {e}"))?;
    for row in rows {
        let record: Vec<String> = headers
            .iter()
            .map(|header| row.get(*header).map(cell_text).unwrap_or_default())
            .collect();
        writer
            .write_record(&record)
            .map_err(|e| format!("CSV: {e}"))?;
    }
    let bytes = writer.into_inner().map_err(|e| format!("CSV: {e}"))?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

struct Frame {
    name: String,
    members: Map<String, Value>,
    text: String,
}

impl Frame {
    fn new(start: &BytesStart) -> Result<Self, String> {
        let mut members = Map::new();
        for attribute in start.attributes() {
            let attribute = attribute.map_err(|e| format!("XML: {e}"))?;
            let key = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|e| format!("XML: {e}"))?
                .into_owned();
            members.insert(format!("@{key}"), Value::String(value));
        }
        Ok(Frame {
            name: String::from_utf8_lossy(start.name().as_ref()).into_owned(),
            members,
            text: String::new(),
        })
    }

    fn finish(mut self) -> (String, Value) {
        let text = self.text.trim().to_string();
        let value = if self.members.is_empty() {
            if text.is_empty() {
                Value::Null
            } else {
                Value::String(text)
            }
        } else {
            if !text.is_empty() {
                self.members
                    .insert("#text".to_string(), Value::String(text));
            }
            Value::Object(self.members)
        };
        (self.name, value)
    }
}

/// Add a child under `name`; a second child with the same name turns the entry into an array.
fn insert_child(members: &mut Map<String, Value>, name: String, value: Value) {
    match members.get_mut(&name) {
        None => {
            members.insert(name, value);
        }
        Some(Value::Array(items)) => items.push(value),
        Some(existing) => {
            let first = existing.take();
            *existing = Value::Array(vec![first, value]);
        }
    }
}

fn parse_xml(input: &str) -> Result<Value, String> {
    let mut reader = Reader::from_str(input);
    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Option<(String, Value)> = None;

    loop {
        match reader.read_event().map_err(|e| format!("XML: {e}"))? {
            Event::Start(start) => stack.push(Frame::new(&start)?),
            Event::Empty(start) => {
                let (name, value) = Frame::new(&start)?.finish();
                match stack.last_mut() {
                    Some(parent) => insert_child(&mut parent.members, name, value),
                    None => root = Some((name, value)),
                }
            }
            Event::End(_) => {
                let (name, value) = stack.pop().ok_or("XML: unbalanced end tag")?.finish();
                match stack.last_mut() {
                    Some(parent) => insert_child(&mut parent.members, name, value),
                    None => root = Some((name, value)),
                }
            }
            Event::Text(text) => {
                if let Some(frame) = stack.last_mut() {
                    frame
                        .text
                        .push_str(&text.decode().map_err(|e| format!("XML: {e}"))?);
                }
            }
            Event::CData(data) => {
                if let Some(frame) = stack.last_mut() {
                    frame
                        .text
                        .push_str(&data.decode().map_err(|e| format!("XML: {e}"))?);
                }
            }
            Event::GeneralRef(reference) => {
                let resolved = match reference
                    .resolve_char_ref()
                    .map_err(|e| format!("XML: {e}"))?
                {
                    Some(character) => character.to_string(),
                    None => match reference
                        .decode()
                        .map_err(|e| format!("XML: {e}"))?
                        .as_ref()
                    {
                        "amp" => "&".to_string(),
                        "lt" => "<".to_string(),
                        "gt" => ">".to_string(),
                        "quot" => "\"".to_string(),
                        "apos" => "'".to_string(),
                        other => return Err(format!("XML: unknown entity &{other};")),
                    },
                };
                if let Some(frame) = stack.last_mut() {
                    frame.text.push_str(&resolved);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let (name, value) = root.ok_or("XML: no root element")?;
    let mut object = Map::new();
    object.insert(name, value);
    Ok(Value::Object(object))
}

fn render_xml(value: &Value) -> Result<String, String> {
    let (name, content) = match value {
        Value::Object(object) if object.len() == 1 => {
            let (key, content) = object.iter().next().expect("one entry");
            if key.starts_with('@') || key == "#text" || content.is_array() {
                ("root".to_string(), value)
            } else {
                (key.clone(), content)
            }
        }
        _ => ("root".to_string(), value),
    };

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    write_element(&mut writer, &name, content)?;
    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn check_xml_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let valid_start = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_');
    if valid_start && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')) {
        Ok(())
    } else {
        Err(format!(
            "XML: \"{name}\" is not a valid element or attribute name"
        ))
    }
}

fn write_element(writer: &mut Writer<Vec<u8>>, name: &str, value: &Value) -> Result<(), String> {
    check_xml_name(name)?;
    let error = |e: std::io::Error| format!("XML: {e}");

    match value {
        Value::Array(items) => {
            for item in items {
                write_element(writer, name, item)?;
            }
        }
        Value::Null => {
            writer
                .write_event(Event::Empty(BytesStart::new(name)))
                .map_err(error)?;
        }
        Value::Object(members) => {
            let mut start = BytesStart::new(name);
            for (key, attribute) in members {
                if let Some(attribute_name) = key.strip_prefix('@') {
                    check_xml_name(attribute_name)?;
                    start.push_attribute((attribute_name, scalar_text(attribute).as_str()));
                }
            }
            writer.write_event(Event::Start(start)).map_err(error)?;
            for (key, child) in members {
                if key == "#text" {
                    writer
                        .write_event(Event::Text(BytesText::new(&scalar_text(child))))
                        .map_err(error)?;
                } else if !key.starts_with('@') {
                    write_element(writer, key, child)?;
                }
            }
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(error)?;
        }
        scalar => {
            writer
                .write_event(Event::Start(BytesStart::new(name)))
                .map_err(error)?;
            writer
                .write_event(Event::Text(BytesText::new(&scalar_text(scalar))))
                .map_err(error)?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(error)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn json_yaml_toml_round_trip_keeps_key_order() {
        let source = r#"{"b":1,"a":{"x":[1,2],"y":"t"}}"#;
        let yaml = convert(source, Format::Json, Format::Yaml, b',').unwrap();
        assert_eq!(yaml, "b: 1\na:\n  x:\n  - 1\n  - 2\n  y: t\n");
        let toml_text = convert(&yaml, Format::Yaml, Format::Toml, b',').unwrap();
        assert_eq!(parse(&toml_text, Format::Toml, b',').unwrap(), json(source));
        assert!(toml_text.find("b = 1").unwrap() < toml_text.find("[a]").unwrap());
    }

    #[test]
    fn toml_rejects_null_and_non_objects() {
        assert!(convert(r#"{"a":null}"#, Format::Json, Format::Toml, b',').is_err());
        assert!(convert("[1,2]", Format::Json, Format::Toml, b',').is_err());
    }

    #[test]
    fn csv_to_json_infers_types() {
        let csv = "id,name,score,active\n1,Ann,2.5,true\n2,\"Bob, Jr.\",,false\n";
        let value = parse(csv, Format::Csv, b',').unwrap();
        assert_eq!(
            value,
            json(
                r#"[{"id":1,"name":"Ann","score":2.5,"active":true},{"id":2,"name":"Bob, Jr.","score":"","active":false}]"#
            )
        );
    }

    #[test]
    fn json_to_csv_unions_headers_and_honours_delimiter() {
        let out = convert(
            r#"[{"a":1,"b":"x"},{"a":2,"c":[1]}]"#,
            Format::Json,
            Format::Csv,
            b';',
        )
        .unwrap();
        assert_eq!(out, "a;b;c\n1;x;\n2;;[1]\n");
        assert!(convert(r#"{"a":1}"#, Format::Json, Format::Csv, b',').is_err());
    }

    #[test]
    fn xml_to_json_mapping() {
        let xml = r#"<shop id="7"><item>a &amp; b</item><item>c</item><note lang="en">hi</note><empty/></shop>"#;
        assert_eq!(
            parse(xml, Format::Xml, b',').unwrap(),
            json(
                r##"{"shop":{"@id":"7","item":["a & b","c"],"note":{"@lang":"en","#text":"hi"},"empty":null}}"##
            )
        );
    }

    #[test]
    fn json_to_xml_and_back() {
        let value = json(
            r##"{"shop":{"@id":"7","item":["a & b","c"],"note":{"@lang":"en","#text":"hi"},"empty":null}}"##,
        );
        let xml = render(&value, Format::Xml, b',').unwrap();
        assert!(xml.starts_with("<shop id=\"7\">"));
        assert!(xml.contains("<item>a &amp; b</item>"));
        assert_eq!(parse(&xml, Format::Xml, b',').unwrap(), value);
        // Several roots get wrapped, bad names are rejected.
        assert!(
            render(&json(r#"{"a":1,"b":2}"#), Format::Xml, b',')
                .unwrap()
                .starts_with("<root>")
        );
        assert!(render(&json(r#"{"1bad":1}"#), Format::Xml, b',').is_err());
    }
}
