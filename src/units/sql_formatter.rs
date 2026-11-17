// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! SQL formatting and minifying.

use sqlformat::{Dialect, FormatOptions, Indent, QueryParams};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordCase {
    Keep,
    Upper,
    Lower,
}

impl KeywordCase {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => KeywordCase::Upper,
            2 => KeywordCase::Lower,
            _ => KeywordCase::Keep,
        }
    }
}

pub fn dialect_from_index(index: i32) -> Dialect {
    match index {
        1 => Dialect::PostgreSql,
        2 => Dialect::SQLServer,
        _ => Dialect::Generic,
    }
}

pub fn indent_from_index(index: i32) -> Indent {
    match index {
        1 => Indent::Spaces(4),
        2 => Indent::Tabs,
        _ => Indent::Spaces(2),
    }
}

pub fn format(sql: &str, indent: Indent, case: KeywordCase, dialect: Dialect) -> String {
    let options = FormatOptions {
        indent,
        uppercase: match case {
            KeywordCase::Keep => None,
            KeywordCase::Upper => Some(true),
            KeywordCase::Lower => Some(false),
        },
        lines_between_queries: 2,
        dialect,
        ..Default::default()
    };
    sqlformat::format(sql, &QueryParams::None, &options)
}

/// Copy `text` up to and including the closing `terminator`; an unterminated literal runs to the end.
fn take_until(
    chars: &mut std::iter::Peekable<std::str::CharIndices>,
    sql: &str,
    start: usize,
    terminator: &str,
) -> String {
    let mut end = sql.len();
    while let Some((index, _)) = chars.next() {
        if sql[index..].starts_with(terminator) {
            for _ in 1..terminator.chars().count() {
                chars.next();
            }
            end = index + terminator.len();
            break;
        }
    }
    sql[start..end].to_string()
}

/// Strip comments and collapse whitespace outside of string literals and quoted identifiers.
pub fn minify(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.char_indices().peekable();
    let mut pending_space = false;

    let push_space = |out: &mut String, pending: &mut bool| {
        if *pending && !out.is_empty() {
            out.push(' ');
        }
        *pending = false;
    };

    while let Some((start, c)) = chars.next() {
        match c {
            c if c.is_whitespace() => pending_space = true,
            '-' if sql[start..].starts_with("--") => {
                for (_, next) in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
                pending_space = true;
            }
            '/' if sql[start..].starts_with("/*") => {
                chars.next();
                let _ = take_until(&mut chars, sql, start, "*/");
                pending_space = true;
            }
            '\'' | '"' | '`' => {
                push_space(&mut out, &mut pending_space);
                // A doubled quote inside the literal is an escaped quote, which the loop handles by
                // closing and immediately reopening the literal.
                let terminator = c.to_string();
                out.push_str(&take_until(&mut chars, sql, start, &terminator));
            }
            '[' => {
                push_space(&mut out, &mut pending_space);
                out.push_str(&take_until(&mut chars, sql, start, "]"));
            }
            '$' => {
                push_space(&mut out, &mut pending_space);
                // Dollar quoting: $tag$ ... $tag$
                let tag_end = sql[start + 1..].find('$').filter(|end| {
                    sql[start + 1..start + 1 + end]
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_')
                });
                match tag_end {
                    Some(end) => {
                        let tag = &sql[start..start + end + 2];
                        for _ in 0..end + 1 {
                            chars.next();
                        }
                        out.push_str(tag);
                        out.push_str(&take_until(&mut chars, sql, start + tag.len(), tag));
                    }
                    None => out.push('$'),
                }
            }
            c => {
                push_space(&mut out, &mut pending_space);
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_query() {
        let formatted = format(
            "select a,b from t where a=1 and b in (select x from y) order by a",
            Indent::Spaces(2),
            KeywordCase::Upper,
            Dialect::Generic,
        );
        assert_eq!(
            formatted,
            "SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  a = 1\n  AND b IN (\n    SELECT\n      x\n    FROM\n      y\n  )\nORDER BY\n  a"
        );
    }

    #[test]
    fn indent_and_case_options() {
        let tabs = format(
            "select a from t",
            Indent::Tabs,
            KeywordCase::Keep,
            Dialect::Generic,
        );
        assert_eq!(tabs, "select\n\ta\nfrom\n\tt");
        let lower = format(
            "SELECT a FROM t",
            Indent::Spaces(4),
            KeywordCase::Lower,
            Dialect::Generic,
        );
        assert_eq!(lower, "select\n    a\nfrom\n    t");
        assert!(matches!(indent_from_index(2), Indent::Tabs));
    }

    #[test]
    fn dialect_specific_syntax() {
        let pg = format(
            "select arr[1] from t",
            Indent::Spaces(2),
            KeywordCase::Keep,
            Dialect::PostgreSql,
        );
        assert!(pg.contains("arr[1]"));
        let mssql = format(
            "select [my col] from [t]",
            Indent::Spaces(2),
            KeywordCase::Keep,
            Dialect::SQLServer,
        );
        assert!(mssql.contains("[my col]"));
    }

    #[test]
    fn minify_strips_comments_and_whitespace_but_not_literals() {
        let sql = "SELECT  a, -- first\n       'it''s   here' AS \"my  col\" /* hint */\nFROM   t\n WHERE x = $$a  --b$$";
        assert_eq!(
            minify(sql),
            "SELECT a, 'it''s   here' AS \"my  col\" FROM t WHERE x = $$a  --b$$"
        );
        assert_eq!(minify("  select 1  ;  "), "select 1 ;");
        assert_eq!(minify("select [a  b] from t"), "select [a  b] from t");
        assert_eq!(minify("select 'open"), "select 'open");
    }
}
