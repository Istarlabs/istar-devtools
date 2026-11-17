// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::token::Token;
use crate::models::token_type::TokenType;
use crate::models::token_value::TokenValue;

pub struct TokenBuilder {
    decimal_separator: char,
    argument_separator: char,
}

impl TokenBuilder {
    pub fn new(decimal_separator: char, argument_separator: char) -> Self {
        Self {
            decimal_separator,
            argument_separator,
        }
    }

    pub fn build(&self, input: &str) -> Result<Vec<Token>, String> {
        if input.is_empty() {
            return Err("Input is empty!".to_string());
        }

        let mut tokens = Vec::<Token>::new();
        let mut chars = input.char_indices().peekable();
        let mut is_formular_sub_part = true;
        let mut is_scientific = false;

        while let Some((i, c)) = chars.peek().copied() {
            // --- Hex literal: 0x… / 0X… ---
            if c == '0' && (input[i..].starts_with("0x") || input[i..].starts_with("0X")) {
                chars.next(); // '0'
                chars.next(); // 'x'/'X'
                let mut buf = String::new();
                while let Some((_, nc)) = chars.peek().copied() {
                    if nc.is_ascii_hexdigit() {
                        buf.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if buf.is_empty() {
                    return Err("Empty hex literal after '0x'".to_string());
                }
                let val = i64::from_str_radix(&buf, 16)
                    .map_err(|_| format!("Invalid hex literal: 0x{buf}"))?;
                tokens.push(Token {
                    token_type: TokenType::Integer,
                    token_value: TokenValue::Integer(val),
                    position: i as i32,
                    length: (buf.len() + 2) as i32,
                });
                is_formular_sub_part = false;
                continue;
            }

            // --- Binary literal: 0b… / 0B… ---
            if c == '0' && (input[i..].starts_with("0b") || input[i..].starts_with("0B")) {
                chars.next(); // '0'
                chars.next(); // 'b'/'B'
                let mut buf = String::new();
                while let Some((_, nc)) = chars.peek().copied() {
                    if nc == '0' || nc == '1' {
                        buf.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if buf.is_empty() {
                    return Err("Empty binary literal after '0b'".to_string());
                }
                let val = i64::from_str_radix(&buf, 2)
                    .map_err(|_| format!("Invalid binary literal: 0b{buf}"))?;
                tokens.push(Token {
                    token_type: TokenType::Integer,
                    token_value: TokenValue::Integer(val),
                    position: i as i32,
                    length: (buf.len() + 2) as i32,
                });
                is_formular_sub_part = false;
                continue;
            }

            // --- Regular decimal / float ---
            if self.is_numeric_or_part(c, true, is_formular_sub_part) {
                let start = i;
                let mut buf = String::new();

                while let Some((_, c)) = chars.peek().copied() {
                    if !self.is_numeric_or_part(c, buf.is_empty(), is_formular_sub_part) {
                        break;
                    }

                    if is_scientific && Self::is_scientific_notation(c) {
                        return Err("UnexcpectedChar".to_string());
                    }

                    if Self::is_scientific_notation(c) {
                        is_scientific = true;
                        buf.push(c);
                        chars.next();

                        if let Some((_, '-')) = chars.peek().copied() {
                            buf.push('-');
                            chars.next();
                        }

                        continue;
                    }

                    buf.push(c);
                    chars.next();
                }

                let length = buf.len();
                if let Ok(val) = buf.parse::<i64>() {
                    tokens.push(Token {
                        token_type: TokenType::Integer,
                        token_value: TokenValue::Integer(val),
                        position: start as i32,
                        length: length as i32,
                    });
                    is_formular_sub_part = false;
                } else if let Ok(val) = buf.parse::<f64>() {
                    tokens.push(Token {
                        token_type: TokenType::FloatingPoint,
                        token_value: TokenValue::Float(val),
                        position: start as i32,
                        length: length as i32,
                    });
                    is_scientific = false;
                    is_formular_sub_part = false;
                } else if buf == "-" {
                    tokens.push(Token {
                        token_type: TokenType::Operation,
                        token_value: TokenValue::Char('_'),
                        position: start as i32,
                        length: 1,
                    });
                }

                continue;
            }

            chars.next();

            if Self::is_part_of_function(c, true) {
                let start = i as i32;
                let mut buf = String::new();
                buf.push(c);

                while let Some((_, nc)) = chars.peek().copied() {
                    if !Self::is_part_of_function(nc, false) {
                        break;
                    }
                    buf.push(nc);
                    chars.next();
                }

                let length = buf.len() as i32;
                tokens.push(Token {
                    token_type: TokenType::String,
                    token_value: TokenValue::Str(buf),
                    position: start,
                    length,
                });
                is_formular_sub_part = false;
                continue;
            }

            if c == self.argument_separator {
                tokens.push(Token {
                    token_type: TokenType::Separator,
                    token_value: TokenValue::Char(c),
                    position: i as i32,
                    length: 1,
                });
                is_formular_sub_part = true;
                continue;
            }

            match c {
                ' ' => {}
                '+' | '-' | '*' | '/' | '^' | '%' => {
                    let value = if Self::is_unary_minus(c, &tokens) {
                        TokenValue::Str("_".into())
                    } else {
                        TokenValue::Char(c)
                    };
                    tokens.push(Token {
                        token_type: TokenType::Operation,
                        token_value: value,
                        position: i as i32,
                        length: 1,
                    });
                    is_formular_sub_part = true;
                }
                '&' | '|' => {
                    tokens.push(Token {
                        token_type: TokenType::Operation,
                        token_value: TokenValue::Char(c),
                        position: i as i32,
                        length: 1,
                    });
                    is_formular_sub_part = true;
                }
                '~' => {
                    tokens.push(Token {
                        token_type: TokenType::Operation,
                        token_value: TokenValue::Char('~'),
                        position: i as i32,
                        length: 1,
                    });
                    is_formular_sub_part = true;
                }
                '<' => {
                    if let Some((_, '<')) = chars.peek() {
                        chars.next();
                        tokens.push(Token {
                            token_type: TokenType::Operation,
                            token_value: TokenValue::Char('<'),
                            position: i as i32,
                            length: 2,
                        });
                        is_formular_sub_part = true;
                    } else {
                        return Err("UnexpectedChar: single '<' (did you mean '<<'?)".to_string());
                    }
                }
                '>' => {
                    if let Some((_, '>')) = chars.peek() {
                        chars.next();
                        tokens.push(Token {
                            token_type: TokenType::Operation,
                            token_value: TokenValue::Char('>'),
                            position: i as i32,
                            length: 2,
                        });
                        is_formular_sub_part = true;
                    } else {
                        return Err("UnexpectedChar: single '>' (did you mean '>>'?)".to_string());
                    }
                }
                '(' => {
                    tokens.push(Token {
                        token_type: TokenType::LeftBracket,
                        token_value: TokenValue::Char(c),
                        position: i as i32,
                        length: 1,
                    });
                    is_formular_sub_part = true;
                }
                ')' => {
                    tokens.push(Token {
                        token_type: TokenType::RightBracket,
                        token_value: TokenValue::Char(c),
                        position: i as i32,
                        length: 1,
                    });
                    is_formular_sub_part = false;
                }
                _ => return Err("UnexcpectedChar".to_string()),
            }
        }

        Ok(tokens)
    }

    fn is_numeric_or_part(&self, c: char, is_first: bool, is_subpart: bool) -> bool {
        c == self.decimal_separator
            || c.is_ascii_digit()
            || (is_subpart && is_first && c == '-')
            || (!is_first && (c == 'e' || c == 'E'))
    }

    fn is_part_of_function(c: char, is_first: bool) -> bool {
        c.is_alphabetic() || (!is_first && c.is_ascii_digit()) || (!is_first && c == '_')
    }

    fn is_unary_minus(c: char, tokens: &[Token]) -> bool {
        c == '-'
            && tokens.last().map_or(true, |t| {
                !matches!(
                    t.token_type,
                    TokenType::FloatingPoint
                        | TokenType::Integer
                        | TokenType::String
                        | TokenType::RightBracket
                )
            })
    }

    fn is_scientific_notation(c: char) -> bool {
        c.eq_ignore_ascii_case(&'e')
    }
}
