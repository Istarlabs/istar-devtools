// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::{Display, Formatter};
use crate::models::token_type::TokenType;
use crate::models::token_value::TokenValue;

#[derive(Debug, Clone)]
pub struct Token {
    pub position: i32,
    pub length: i32,
    pub token_type: TokenType,
    pub token_value: TokenValue,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Type: {} = {} [At {} | Length {}]",
            self.token_type, self.token_value, self.position, self.length
        )
    }
}

