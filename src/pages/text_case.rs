// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, CursorPosition, TextAndCaseState, TextStatistics};
use slint::ComponentHandle;

#[repr(i32)]
#[derive(Copy, Clone)]
enum CaseMode {
    OriginalCase = 0,
    SentenceCase = 1,
    LowerCase = 2,
    UpperCase = 3,
    TitleCase = 4,
    CamelCase = 5,
    PascalCase = 6,
    SnakeCase = 7,
    ConstantCase = 8,
    KebabCase = 9,
    CobolCase = 10,
    TrainCase = 11,
    AlternatingCase = 12,
    ReverseAlternatingCase = 13,
    Reverse = 14,
}

impl CaseMode {
    fn from_i32(val: i32) -> Option<Self> {
        match val {
            0 => Some(Self::OriginalCase),
            1 => Some(Self::SentenceCase),
            2 => Some(Self::LowerCase),
            3 => Some(Self::UpperCase),
            4 => Some(Self::TitleCase),
            5 => Some(Self::CamelCase),
            6 => Some(Self::PascalCase),
            7 => Some(Self::SnakeCase),
            8 => Some(Self::ConstantCase),
            9 => Some(Self::KebabCase),
            10 => Some(Self::CobolCase),
            11 => Some(Self::TrainCase),
            12 => Some(Self::AlternatingCase),
            13 => Some(Self::ReverseAlternatingCase),
            14 => Some(Self::Reverse),
            _ => None,
        }
    }
}

fn to_camel_case(text: &str, reverse: bool) -> String {
    let mut output = String::new();
    let mut next_upper_case = reverse;

    for (_, c) in text.chars().enumerate() {
        if c.is_alphanumeric() {
            if next_upper_case {
                output.extend(c.to_uppercase());
                next_upper_case = false;
            } else {
                output.extend(c.to_lowercase());
            }
        } else {
            if c == '\n' {
                output.push(c);
                next_upper_case = false;
            } else {
                next_upper_case = true;
            }
        }
    }

    output
}

fn to_sentence_case(text: &str) -> String {
    let mut output = String::new();
    let mut new_sentence = true;

    text.chars().enumerate().into_iter().for_each(|(_, c)| {
        if c == '.' || c == '?' || c == '!' || c == '\n' {
            output.push(c);
            new_sentence = true;
        } else if c.is_alphanumeric() {
            if new_sentence {
                output.extend(c.to_uppercase());
                new_sentence = false;
            } else {
                output.extend(c.to_lowercase());
            }
        } else {
            output.push(c);
        }
    });

    output
}

fn to_title_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i == 0 || !chars[i - 1].is_alphanumeric() {
            output.extend(c.to_uppercase());
        } else {
            output.extend(c.to_lowercase());
        }
    }

    output
}

fn to_snake_constant_kebab_cobol_converter(
    text: &str,
    space_replacement: char,
    is_upper_case: bool,
) -> String {
    let mut ignore_next_non_alphanumeric = true;
    let mut output = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_alphanumeric() {
            ignore_next_non_alphanumeric = false;
            if is_upper_case {
                output.extend(c.to_uppercase())
            } else {
                output.extend(c.to_lowercase())
            }
        } else if *c == '\n' {
            ignore_next_non_alphanumeric = true;
            output.push(*c);
        } else if !ignore_next_non_alphanumeric {
            if i < chars.len() - 1 && chars[i + 1].is_alphanumeric() {
                ignore_next_non_alphanumeric = true;
                output.push(space_replacement);
            }
        }
    }

    output
}

fn to_train_case(text: &str) -> String {
    let mut ignore_next_non_alphanumeric = true;
    let mut output = String::new();
    let chars: Vec<char> = text.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if c.is_alphanumeric() {
            if ignore_next_non_alphanumeric {
                output.extend(c.to_uppercase())
            } else {
                output.extend(c.to_lowercase())
            }
            ignore_next_non_alphanumeric = false;
        } else if *c == '\n' {
            ignore_next_non_alphanumeric = true;
            output.push(*c);
        } else if !ignore_next_non_alphanumeric {
            if i < chars.len() - 1 && chars[i + 1].is_alphanumeric() {
                ignore_next_non_alphanumeric = true;
                output.push('-')
            }
        }
    }

    output
}

fn to_alternated_case(text: &str, mut start_with_lower: bool) -> String {
    let mut output = String::new();

    for (_, c) in text.chars().enumerate() {
        if start_with_lower {
            output.extend(c.to_lowercase())
        } else {
            output.extend(c.to_uppercase())
        }

        start_with_lower = !start_with_lower;
    }

    output
}

fn counter(text: &str) -> TextStatistics {
    TextStatistics {
        lines: text.lines().count() as i32,
        word_count: text.split_whitespace().count() as i32,
        char_count: text.chars().count() as i32,
    }
}

fn cursor_position_from_byte_offset(text: &str, byte_offset: usize) -> CursorPosition {
    let clamped = byte_offset.min(text.len());
    let before = &text[..clamped];
    let line = before.chars().filter(|&c| c == '\n').count() as i32 + 1;
    let row = before.rfind('\n').map(|pos| clamped - pos - 1).unwrap_or(clamped) as i32 + 1;
    CursorPosition { line, row }
}

pub fn register(app: &AppWindow) {
    let ui_handle = app.as_weak();
    app.global::<TextAndCaseState>().on_update_cursor(move |byte_offset| {
        let ui = ui_handle.unwrap();
        let state = ui.global::<TextAndCaseState>();
        let input = state.get_input();
        let pos = cursor_position_from_byte_offset(input.as_str(), byte_offset as usize);
        state.set_cursor_position(pos);
    });

    let ui_handle = app.as_weak();
    app.global::<TextAndCaseState>().on_convert(move |idx| {
        let ui = ui_handle.unwrap();
        let state = ui.global::<TextAndCaseState>();
        let input = state.get_input();
        let text = input.as_str();

        let converted = match CaseMode::from_i32(idx) {
            Some(CaseMode::LowerCase) => text.to_lowercase(),
            Some(CaseMode::UpperCase) => text.to_uppercase(),
            Some(CaseMode::CamelCase) => to_camel_case(text, false),
            Some(CaseMode::PascalCase) => to_camel_case(text, true),
            Some(CaseMode::SentenceCase) => to_sentence_case(text),
            Some(CaseMode::TitleCase) => to_title_case(text),
            Some(CaseMode::SnakeCase) => to_snake_constant_kebab_cobol_converter(text, '_', false),
            Some(CaseMode::ConstantCase) => {
                to_snake_constant_kebab_cobol_converter(text, '_', true)
            }
            Some(CaseMode::KebabCase) => to_snake_constant_kebab_cobol_converter(text, '-', false),
            Some(CaseMode::CobolCase) => to_snake_constant_kebab_cobol_converter(text, '-', true),
            Some(CaseMode::TrainCase) => to_train_case(text),
            Some(CaseMode::AlternatingCase) => to_alternated_case(text, true),
            Some(CaseMode::ReverseAlternatingCase) => to_alternated_case(text, false),
            Some(CaseMode::Reverse) => text.chars().rev().collect(),
            Some(CaseMode::OriginalCase) => text.to_string(),
            None => String::new(),
        };

        state.set_text_statistics(counter(converted.as_str()));
        state.set_input(converted.into());
    });
}
