// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::constant_registry::ConstantRegistry;
use crate::models::data_type::DataType;
use crate::models::error::Error;
use crate::models::function_registry::FunctionRegistry;
use crate::models::operation::Operation;
use crate::models::operation_meta::OperationMeta;
use crate::models::token::Token;
use crate::models::token_type::TokenType;
use crate::models::token_value::TokenValue;

pub struct AstBuilder<'a> {
    operation_stack: Vec<Token>,
    result_stack: Vec<Operation>,
    parameter_stack: Vec<i32>,
    function_registry: &'a FunctionRegistry,
    constant_registry: &'a ConstantRegistry,
}

impl<'a> AstBuilder<'a> {
    pub fn new(
        function_registry: &'a FunctionRegistry,
        constant_registry: &'a ConstantRegistry,
    ) -> Self {
        Self {
            operation_stack: Vec::new(),
            result_stack: Vec::new(),
            parameter_stack: Vec::new(),
            function_registry,
            constant_registry,
        }
    }

    // Priorities follow C-like ordering (higher = evaluated first).
    // Bitwise ops (1–3) are below arithmetic (5–7) so that
    // `a + b & mask` parses as `(a + b) & mask`.
    const OPERATION_PRIORITY: &'static [(char, u8)] = &[
        ('(', 0),
        ('|', 1),  // bitwise OR  — lowest
        ('&', 2),  // bitwise AND
        ('<', 3),  // left shift (<<)
        ('>', 3),  // right shift (>>)
        ('+', 5),
        ('-', 5),
        ('*', 6),
        ('/', 6),
        ('%', 6),
        ('^', 7),  // exponentiation
        ('_', 8),  // unary minus
        ('~', 8),  // bitwise NOT (unary)
    ];

    fn get_priority(op: char) -> u8 {
        Self::OPERATION_PRIORITY
            .iter()
            .find(|(c, _)| *c == op)
            .map(|(_, p)| *p)
            .unwrap_or(0)
    }

    pub fn build(&mut self, tokens: Vec<Token>) -> Result<Operation, Error> {
        self.result_stack.clear();
        self.operation_stack.clear();
        self.parameter_stack.clear();

        for t in tokens {
            match (&t.token_type, &t.token_value) {
                (TokenType::Integer, TokenValue::Integer(v)) => {
                    self.result_stack.push(Operation::IntegerConstant(*v));
                }
                (TokenType::FloatingPoint, TokenValue::Float(v)) => {
                    self.result_stack.push(Operation::FloatingPointConstant(*v));
                }
                (TokenType::String, TokenValue::Str(txt)) => {
                    if self.function_registry.is_valid_function(&txt) {
                        self.operation_stack.push(t.clone());
                        self.parameter_stack.push(1);
                    } else if let Ok(info) = self.constant_registry.get_constant_value(txt) {
                        self.result_stack
                            .push(Operation::FloatingPointConstant(info.values))
                    }
                }
                (TokenType::Operation, TokenValue::Char(c)) => {
                    let op1_name = *c;

                    while !self.operation_stack.is_empty()
                        && matches!(
                            self.operation_stack.last().unwrap().token_type,
                            TokenType::Operation | TokenType::String
                        )
                    {
                        let is_function =
                            self.operation_stack.last().unwrap().token_type == TokenType::String;

                        if !is_function {
                            let op2_name = match self.operation_stack.last().unwrap().token_value {
                                TokenValue::Char(c) => c,
                                _ => break,
                            };

                            let left_assoc = Self::is_left_associative(op1_name);
                            let op1_prio = Self::get_priority(op1_name);
                            let op2_prio = Self::get_priority(op2_name);

                            if (left_assoc && op1_prio <= op2_prio) || op1_prio < op2_prio {
                                let op2 = self.operation_stack.pop().unwrap();
                                let converted = self.convert_operation(&op2)?;
                                self.result_stack.push(converted);
                            } else {
                                break;
                            }
                        } else {
                            let op2 = self.operation_stack.pop().unwrap();
                            let converted = self.convert_function(&op2)?;
                            self.result_stack.push(converted);
                        }
                    }
                    self.operation_stack.push(t.clone());
                }
                (TokenType::LeftBracket, _) => {
                    self.operation_stack.push(t.clone());
                }
                (TokenType::RightBracket, _) => {
                    self.pop_operation(true)?;
                }
                _ => {}
            }
        }
        self.pop_operation(false)?;

        if self.operation_stack.iter().any(|t| t.token_type == TokenType::LeftBracket) {
            return Err(Error::ParseError("Unclosed left bracket".into()));
        }

        if self.result_stack.len() > 1 {
            return Err(Error::InvalidSyntax(
                "Result stack has more than one item".into(),
            ));
        }

        self.result_stack
            .pop()
            .ok_or(Error::InvalidSyntax("Empty result stack".into()))
    }

    fn pop_operation(&mut self, until_left_bracket: bool) -> Result<(), Error> {
        while !self.operation_stack.is_empty()
            && self.operation_stack.last().unwrap().token_type != TokenType::LeftBracket
        {
            let token = self.operation_stack.pop().unwrap();
            match token.token_type {
                TokenType::Operation => {
                    let op = self.convert_operation(&token)?;
                    self.result_stack.push(op);
                }
                TokenType::String => {
                    let op = self.convert_function(&token)?;
                    self.result_stack.push(op);
                }
                _ => {}
            }
        }

        if until_left_bracket {
            if matches!(self.operation_stack.last(), Some(t) if t.token_type == TokenType::LeftBracket)
            {
                self.operation_stack.pop();
            } else {
                return Err(Error::ParseError("No matching left bracket found".into()));
            }
        }

        Ok(())
    }

    fn convert_operation(&mut self, token: &Token) -> Result<Operation, Error> {
        let c = match token.token_value {
            TokenValue::Char(c) => c,
            _ => return Err(Error::ParseError("Expected char token value".into())),
        };

        let result = match c {
            '+' => {
                let arg2 = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let arg1 = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let dt = Self::get_result_data_type(&arg1, &arg2);
                Operation::Addition {
                    meta: OperationMeta::new(dt, &arg1, &arg2),
                    left: Box::new(arg1),
                    right: Box::new(arg2),
                }
            }
            '-' => {
                let arg2 = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let arg1 = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let dt = Self::get_result_data_type(&arg1, &arg2);
                Operation::Subtraction {
                    meta: OperationMeta::new(dt, &arg1, &arg2),
                    left: Box::new(arg1),
                    right: Box::new(arg2),
                }
            }
            '*' => {
                let arg2 = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let arg1 = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let dt = Self::get_result_data_type(&arg1, &arg2);
                Operation::Multiplication {
                    meta: OperationMeta::new(dt, &arg1, &arg2),
                    left: Box::new(arg1),
                    right: Box::new(arg2),
                }
            }
            '/' => {
                let divisor = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let dividend = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::Division {
                    meta: OperationMeta::binary(DataType::FloatingPoint, &dividend, &divisor),
                    dividend: Box::new(dividend),
                    divisor: Box::new(divisor),
                }
            }
            '%' => {
                let divisor = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let dividend = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::Modulo {
                    meta: OperationMeta::binary(DataType::FloatingPoint, &dividend, &divisor),
                    dividend: Box::new(dividend),
                    divisor: Box::new(divisor),
                }
            }
            '_' => {
                let arg = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::UnaryMinus {
                    meta: OperationMeta::unary(arg.get_data_type(), &arg),
                    argument: Box::new(arg),
                }
            }
            '^' => {
                let exponent = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let base = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::Exponentiation {
                    meta: OperationMeta::binary(DataType::FloatingPoint, &base, &exponent),
                    base: Box::new(base),
                    exponent: Box::new(exponent),
                }
            }
            '&' => {
                let right = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let left = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::BitwiseAnd {
                    meta: OperationMeta::new(DataType::Integer, &left, &right),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            '|' => {
                let right = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let left = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::BitwiseOr {
                    meta: OperationMeta::new(DataType::Integer, &left, &right),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            '~' => {
                let arg = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::BitwiseNot {
                    meta: OperationMeta::unary(DataType::Integer, &arg),
                    argument: Box::new(arg),
                }
            }
            '<' => {
                let right = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let left = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::ShiftLeft {
                    meta: OperationMeta::new(DataType::Integer, &left, &right),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            '>' => {
                let right = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                let left = self
                    .result_stack
                    .pop()
                    .ok_or(Error::ParseError("Missing argument".into()))?;
                Operation::ShiftRight {
                    meta: OperationMeta::new(DataType::Integer, &left, &right),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            _ => {
                return Err(Error::ParseError(format!(
                    "Unknown operation '{c}' at position {}",
                    token.position
                )));
            }
        };

        Ok(result)
    }

    fn convert_function(&mut self, token: &Token) -> Result<Operation, Error> {
        let name = match &token.token_value {
            TokenValue::Str(s) => s.clone(),
            _ => return Err(Error::ParseError("Expected string token value".into())),
        };

        let info = self
            .function_registry
            .get_function_info(&name)
            .ok_or(Error::ParseError(format!("Unknown function \"{name}\"")))?;

        let mut operations = Vec::new();
        for _ in 0..info.param_count {
            operations.push(self.result_stack.pop().ok_or(Error::ParseError(format!(
                "Syntax issue for function \"{name}\" at position {}",
                token.position
            )))?);
        }
        operations.reverse();

        Ok(Operation::Function {
            meta: OperationMeta {
                data_type: DataType::FloatingPoint,
                depends_on_variables: operations.iter().any(|o| o.depends_on_variables()),
                is_idempotent: info.is_idempotent && operations.iter().all(|o| o.is_idempotent()),
            },
            name,
            arguments: operations,
        })
    }

    fn get_result_data_type(arg1: &Operation, arg2: &Operation) -> DataType {
        if arg1.get_data_type() == DataType::FloatingPoint
            || arg2.get_data_type() == DataType::FloatingPoint
        {
            DataType::FloatingPoint
        } else {
            DataType::Integer
        }
    }

    fn is_left_associative(c: char) -> bool {
        "*+-/%&|<>".contains(c)
    }
}
