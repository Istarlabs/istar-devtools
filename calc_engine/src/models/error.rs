// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),
    #[error("Unexpected character '{0}' at position {1}")]
    UnexpectedChar(char, usize),
    #[error("Empty input")]
    EmptyInput,
}