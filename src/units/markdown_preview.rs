// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Markdown to a flat list of display blocks, and to HTML.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph = 0,
    Heading = 1,
    Code = 2,
    Quote = 3,
    ListItem = 4,
    Rule = 5,
    TableRow = 6,
}

/// One displayable block. `level` is the heading level, the list nesting depth, or 1 for a table header row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub level: i32,
    pub text: String,
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

fn heading_level(level: HeadingLevel) -> i32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    text: String,
    /// One entry per open list: the next number of an ordered list, `None` for a bullet list.
    lists: Vec<Option<u64>>,
    quote_depth: usize,
    item_marker: Option<(String, i32)>,
    heading: Option<i32>,
    in_code_block: bool,
    in_table_head: bool,
    cells: Vec<String>,
    links: Vec<String>,
}

impl Builder {
    fn push(&mut self, kind: BlockKind, level: i32) {
        let text = std::mem::take(&mut self.text);
        let text = text.trim().to_string();
        if !text.is_empty() {
            self.blocks.push(Block { kind, level, text });
        }
    }

    /// Emit the text collected for the current list item, if any.
    fn flush_item(&mut self) {
        if let Some((marker, depth)) = self.item_marker.take() {
            let text = std::mem::take(&mut self.text);
            let text = text.trim();
            if !text.is_empty() {
                self.blocks.push(Block {
                    kind: BlockKind::ListItem,
                    level: depth,
                    text: format!("{marker}{text}"),
                });
            }
        }
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) | Event::Code(text) => self.text.push_str(&text),
            Event::SoftBreak => self.text.push(' '),
            Event::HardBreak => self.text.push('\n'),
            Event::Rule => self.blocks.push(Block {
                kind: BlockKind::Rule,
                level: 0,
                text: String::new(),
            }),
            Event::TaskListMarker(checked) => {
                if let Some((marker, _)) = self.item_marker.as_mut() {
                    *marker = if checked { "☑ " } else { "☐ " }.to_string();
                }
            }
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.push(BlockKind::Paragraph, 0);
                self.heading = Some(heading_level(level));
            }
            Tag::BlockQuote(_) => self.quote_depth += 1,
            Tag::CodeBlock(kind) => {
                self.push(BlockKind::Paragraph, 0);
                self.in_code_block = true;
                if let CodeBlockKind::Fenced(language) = kind
                    && !language.is_empty()
                {
                    // Keep the language visible as a first line comment-free hint.
                    self.text.push_str(&format!("[{language}]\n"));
                }
            }
            Tag::List(start) => {
                // Text of the parent item before a nested list is its own block.
                self.flush_item_keep_open();
                self.lists.push(start);
            }
            Tag::Item => {
                let depth = self.lists.len().saturating_sub(1) as i32;
                let marker = match self.lists.last_mut() {
                    Some(Some(number)) => {
                        let marker = format!("{number}. ");
                        *number += 1;
                        marker
                    }
                    _ => "• ".to_string(),
                };
                self.item_marker = Some((marker, depth));
            }
            Tag::Table(_) => self.push(BlockKind::Paragraph, 0),
            Tag::TableHead => self.in_table_head = true,
            Tag::Link { dest_url, .. } => self.links.push(dest_url.to_string()),
            Tag::Image { .. } => self.text.push_str("🖼 "),
            _ => {}
        }
    }

    /// Emit the pending item text but keep the marker slot closed, so the nested list starts clean.
    fn flush_item_keep_open(&mut self) {
        self.flush_item();
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                let level = self.heading.take().unwrap_or(1);
                self.push(BlockKind::Heading, level);
            }
            TagEnd::Paragraph => {
                if self.item_marker.is_some() {
                    self.text.push('\n');
                } else if self.quote_depth > 0 {
                    self.push(BlockKind::Quote, self.quote_depth as i32);
                } else {
                    self.push(BlockKind::Paragraph, 0);
                }
            }
            TagEnd::BlockQuote(_) => self.quote_depth = self.quote_depth.saturating_sub(1),
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let text = std::mem::take(&mut self.text);
                let text = text.trim_end().to_string();
                if !text.is_empty() {
                    self.blocks.push(Block {
                        kind: BlockKind::Code,
                        level: 0,
                        text,
                    });
                }
            }
            TagEnd::Item => self.flush_item(),
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.text);
                self.cells.push(cell.trim().to_string());
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                let header = matches!(tag, TagEnd::TableHead);
                if header {
                    self.in_table_head = false;
                }
                let cells = std::mem::take(&mut self.cells);
                self.blocks.push(Block {
                    kind: BlockKind::TableRow,
                    level: i32::from(header),
                    text: cells.join(" │ "),
                });
            }
            TagEnd::Link => {
                if let Some(url) = self.links.pop()
                    && !url.is_empty()
                {
                    self.text.push_str(&format!(" ({url})"));
                }
            }
            _ => {}
        }
    }
}

/// Inline formatting (bold, italic, strike-through) is dropped; links keep their URL in parentheses.
pub fn blocks(markdown: &str) -> Vec<Block> {
    let mut builder = Builder::default();
    for event in Parser::new_ext(markdown, options()) {
        builder.event(event);
    }
    builder.push(BlockKind::Paragraph, 0);
    builder.blocks
}

pub fn to_html(markdown: &str) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, Parser::new_ext(markdown, options()));
    html
}

/// A self-contained page for viewing in a browser.
pub fn html_page(markdown: &str) -> String {
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>Markdown preview</title>\n\
<style>body{{font-family:sans-serif;max-width:50em;margin:2em auto;padding:0 1em;line-height:1.5}}\
pre,code{{background:#f0f0f0;border-radius:4px}}pre{{padding:.8em;overflow:auto}}code{{padding:.1em .3em}}\
pre code{{padding:0}}blockquote{{border-left:4px solid #ccc;margin-left:0;padding-left:1em;color:#555}}\
table{{border-collapse:collapse}}td,th{{border:1px solid #ccc;padding:.3em .6em}}\
img{{max-width:100%}}</style></head><body>\n{}</body></html>\n",
        to_html(markdown)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple(markdown: &str) -> Vec<(BlockKind, i32, String)> {
        blocks(markdown)
            .into_iter()
            .map(|b| (b.kind, b.level, b.text))
            .collect()
    }

    #[test]
    fn headings_paragraphs_and_inline_text() {
        let result = simple(
            "# Title\n\nSome **bold**, *italic*, `code` and [a link](https://x.org).\n\n### Sub",
        );
        assert_eq!(
            result,
            vec![
                (BlockKind::Heading, 1, "Title".to_string()),
                (
                    BlockKind::Paragraph,
                    0,
                    "Some bold, italic, code and a link (https://x.org).".to_string()
                ),
                (BlockKind::Heading, 3, "Sub".to_string()),
            ]
        );
    }

    #[test]
    fn lists_with_nesting_numbers_and_tasks() {
        let result =
            simple("- one\n- two\n  - nested\n\n1. first\n2. second\n\n- [x] done\n- [ ] todo\n");
        assert_eq!(
            result,
            vec![
                (BlockKind::ListItem, 0, "• one".to_string()),
                (BlockKind::ListItem, 0, "• two".to_string()),
                (BlockKind::ListItem, 1, "• nested".to_string()),
                (BlockKind::ListItem, 0, "1. first".to_string()),
                (BlockKind::ListItem, 0, "2. second".to_string()),
                (BlockKind::ListItem, 0, "☑ done".to_string()),
                (BlockKind::ListItem, 0, "☐ todo".to_string()),
            ]
        );
    }

    #[test]
    fn code_quotes_rules_and_tables() {
        let result = simple(
            "```rust\nlet x = 1;\nlet y = 2;\n```\n\n> quoted\n\n---\n\n| a | b |\n|---|---|\n| 1 | 2 |\n",
        );
        assert_eq!(
            result,
            vec![
                (
                    BlockKind::Code,
                    0,
                    "[rust]\nlet x = 1;\nlet y = 2;".to_string()
                ),
                (BlockKind::Quote, 1, "quoted".to_string()),
                (BlockKind::Rule, 0, String::new()),
                (BlockKind::TableRow, 1, "a │ b".to_string()),
                (BlockKind::TableRow, 0, "1 │ 2".to_string()),
            ]
        );
    }

    #[test]
    fn html_output() {
        assert_eq!(
            to_html("# Hi\n\n~~gone~~"),
            "<h1>Hi</h1>\n<p><del>gone</del></p>\n"
        );
        assert!(html_page("x").contains("<p>x</p>"));
        assert!(blocks("").is_empty());
    }
}
