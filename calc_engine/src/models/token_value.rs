// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum TokenValue {
    Integer(i64),
    Float(f64),
    Char(char),
    Str(String),
}

impl Display for TokenValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenValue::Integer(value) => write!(f, "{}", value),
            TokenValue::Float(value) => write!(f, "{}", value),
            TokenValue::Char(value) => write!(f, "{}", value),
            TokenValue::Str(value) => write!(f, "{}", value),
        }
    }
}