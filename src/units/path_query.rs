// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! JSONPath (RFC 9535) and XPath 1.0 queries.

use serde_json::Value;
use serde_json_path::JsonPath;
use uppsala::{NodeKind, XPathEvaluator, XPathValue};

/// Parse a JSON document, falling back to YAML.
fn parse_document(text: &str) -> Result<Value, String> {
    serde_json::from_str(text)
        .or_else(|json_error| serde_norway::from_str(text).map_err(|_| json_error))
        .map_err(|e| format!("Document: {e}"))
}

/// Matches as `(normalized path, value)` rows.
pub fn jsonpath(document: &str, expression: &str) -> Result<Vec<(String, String)>, String> {
    let value = parse_document(document)?;
    let path = JsonPath::parse(expression).map_err(|e| format!("JSONPath: {e}"))?;
    path.query_located(&value)
        .iter()
        .map(|located| {
            let text = serde_json::to_string_pretty(located.node()).map_err(|e| e.to_string())?;
            Ok((located.location().to_string(), text))
        })
        .collect()
}

/// Matches as `(kind, serialised node)` rows; non-node results give a single `result` row.
pub fn xpath(document: &str, expression: &str) -> Result<Vec<(String, String)>, String> {
    let mut doc = uppsala::parse(document).map_err(|e| format!("Document: {e}"))?;
    // Attribute nodes only exist for XPath after this call.
    doc.prepare_xpath();
    let result = XPathEvaluator::new()
        .evaluate(&doc, doc.root(), expression)
        .map_err(|e| format!("XPath: {e}"))?;

    match result {
        XPathValue::NodeSet(nodes) => Ok(nodes
            .into_iter()
            .map(|id| {
                let string_value = || XPathValue::NodeSet(vec![id]).to_string_value(&doc);
                match doc.node_kind(id) {
                    Some(NodeKind::Element(_)) => ("element".to_string(), doc.node_to_xml(id)),
                    Some(NodeKind::Attribute { .. }) => ("attribute".to_string(), string_value()),
                    Some(NodeKind::Text(_)) => ("text".to_string(), string_value()),
                    _ => ("node".to_string(), string_value()),
                }
            })
            .collect()),
        other => Ok(vec![("result".to_string(), other.to_string_value(&doc))]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"store":{"book":[{"title":"A","price":8},{"title":"B","price":12}]}}"#;

    #[test]
    fn jsonpath_returns_values_with_locations() {
        let rows = jsonpath(JSON, "$.store.book[?@.price > 10].title").unwrap();
        assert_eq!(
            rows,
            vec![(
                "$['store']['book'][1]['title']".to_string(),
                "\"B\"".to_string()
            )]
        );
        assert_eq!(jsonpath(JSON, "$..price").unwrap().len(), 2);
        assert!(jsonpath(JSON, "$.nothing").unwrap().is_empty());
    }

    #[test]
    fn jsonpath_reads_yaml_and_reports_errors() {
        let rows = jsonpath("a:\n  b: 1\n", "$.a.b").unwrap();
        assert_eq!(rows[0].1, "1");
        assert!(jsonpath(JSON, "$[").is_err());
        assert!(jsonpath("{ not: [valid", "$").is_err());
    }

    #[test]
    fn xpath_nodes_and_scalars() {
        let xml = r#"<lib><book id="1"><t>A</t></book><book id="2"><t>B</t></book></lib>"#;
        let books = xpath(xml, "//book[@id='2']").unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].0, "element");
        assert!(books[0].1.contains("<t>B</t>"));

        let ids = xpath(xml, "//book/@id").unwrap();
        assert_eq!(
            ids.iter().map(|r| r.1.as_str()).collect::<Vec<_>>(),
            ["1", "2"]
        );

        let texts = xpath(xml, "//t/text()").unwrap();
        assert_eq!(
            texts.iter().map(|r| r.1.as_str()).collect::<Vec<_>>(),
            ["A", "B"]
        );

        assert_eq!(
            xpath(xml, "count(//book)").unwrap(),
            vec![("result".to_string(), "2".to_string())]
        );
        assert!(xpath(xml, "//[").is_err());
        assert!(xpath("<a>", "/a").is_err());
    }
}
