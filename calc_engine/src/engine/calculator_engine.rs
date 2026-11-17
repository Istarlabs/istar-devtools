// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::engine::ast_builder::AstBuilder;
use crate::models::constant_registry::ConstantRegistry;
use crate::models::error::Error;
use crate::models::function_registry::FunctionRegistry;
use crate::models::operation::Operation;
use crate::models::token_builder::TokenBuilder;
use std::f64::consts::{E, PI};

pub struct CalculatorEngine {
    function_registry: FunctionRegistry,
    constant_registry: ConstantRegistry,
}

impl CalculatorEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            function_registry: FunctionRegistry::new(),
            constant_registry: ConstantRegistry::new(),
        };
        engine.register_default_constants();
        engine.register_default_functions();
        engine
    }

    fn register_default_functions(&mut self) {
        self.function_registry
            .register("sin", 1, true, false, |args| args[0].sin())
            .unwrap();
        self.function_registry
            .register("cos", 1, true, false, |args| args[0].cos())
            .unwrap();
        self.function_registry
            .register("tan", 1, true, false, |args| args[0].tan())
            .unwrap();
        self.function_registry
            .register("sqrt", 1, true, false, |args| args[0].sqrt())
            .unwrap();
        self.function_registry
            .register("log", 2, true, false, |args| args[0].log(args[1]))
            .unwrap();
        self.function_registry
            .register("ln", 1, true, false, |args| args[0].ln())
            .unwrap();
        self.function_registry
            .register("fact", 1, true, false, |args| {
                let n = args[0] as u64;
                (1..=n).fold(1.0f64, |acc, i| acc * i as f64)
            })
            .unwrap();
        self.function_registry
            .register("xor", 2, true, false, |args| {
                (args[0] as i64 ^ args[1] as i64) as f64
            })
            .unwrap();
    }

    fn register_default_constants(&mut self) {
        self.constant_registry.register("E", E, false).unwrap();
        self.constant_registry.register("PI", PI, false).unwrap();
    }

    fn execute_function(&self, name: &str, raw_args: Vec<f64>) -> f64 {
        self.function_registry.invoke(name, raw_args).unwrap_or(0.0)
    }

    pub fn execute(&self, op: &Operation) -> f64 {
        match op {
            Operation::IntegerConstant(v) => *v as f64,
            Operation::FloatingPointConstant(v) => *v,
            Operation::Addition { left, right, .. } => self.execute(left) + self.execute(right),
            Operation::Subtraction { left, right, .. } => self.execute(left) - self.execute(right),
            Operation::Multiplication { left, right, .. } => {
                self.execute(left) * self.execute(right)
            }
            Operation::Division {
                dividend, divisor, ..
            } => self.execute(dividend) / self.execute(divisor),
            Operation::Modulo {
                dividend, divisor, ..
            } => self.execute(dividend) % self.execute(divisor),
            Operation::Exponentiation { base, exponent, .. } => {
                self.execute(base).powf(self.execute(exponent))
            }
            Operation::UnaryMinus { argument, .. } => -self.execute(argument),
            Operation::Function {
                name, arguments, ..
            } => self.execute_function(name, arguments.iter().map(|a| self.execute(a)).collect()),
            Operation::BitwiseAnd { left, right, .. } => {
                (self.execute(left) as i64 & self.execute(right) as i64) as f64
            }
            Operation::BitwiseOr { left, right, .. } => {
                (self.execute(left) as i64 | self.execute(right) as i64) as f64
            }
            Operation::BitwiseNot { argument, .. } => (!( self.execute(argument) as i64)) as f64,
            Operation::ShiftLeft { left, right, .. } => {
                let lhs = self.execute(left) as i64;
                let rhs = (self.execute(right) as i64).clamp(0, 63) as u32;
                lhs.wrapping_shl(rhs) as f64
            }
            Operation::ShiftRight { left, right, .. } => {
                let lhs = self.execute(left) as i64;
                let rhs = (self.execute(right) as i64).clamp(0, 63) as u32;
                lhs.wrapping_shr(rhs) as f64
            }
        }
    }

    pub fn parse(&self, input: &str) -> Result<Operation, Error> {
        let token_builder = TokenBuilder::new('.', ',');
        let tokens = token_builder
            .build(input)
            .map_err(|e| Error::ParseError(e))?;
        let mut ast_builder = AstBuilder::new(&self.function_registry, &self.constant_registry);
        ast_builder.build(tokens)
    }

    pub fn eval(&self, input: &str) -> Result<f64, Error> {
        let token_builder = TokenBuilder::new('.', ',');
        let tokens = token_builder
            .build(input)
            .map_err(|e| Error::ParseError(e))?;
        let mut ast_builder = AstBuilder::new(&self.function_registry, &self.constant_registry);
        let op = ast_builder.build(tokens)?;
        Ok(self.execute(&op))
    }
}

impl Default for CalculatorEngine {
    fn default() -> Self {
        Self::new()
    }
}
