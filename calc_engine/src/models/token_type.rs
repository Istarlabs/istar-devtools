// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Integer,
    FloatingPoint,
    String,
    Operation,
    LeftBracket,
    RightBracket,
    Separator,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}