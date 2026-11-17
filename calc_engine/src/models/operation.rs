// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::data_type::DataType;
use crate::models::operation_meta::OperationMeta;

#[derive(Debug, Clone)]
pub enum Operation {
    IntegerConstant(i64),
    FloatingPointConstant(f64),
    Addition {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
    Subtraction {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
    Multiplication {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
    Division {
        meta: OperationMeta,
        dividend: Box<Operation>,
        divisor: Box<Operation>,
    },
    Modulo {
        meta: OperationMeta,
        dividend: Box<Operation>,
        divisor: Box<Operation>,
    },
    Exponentiation {
        meta: OperationMeta,
        base: Box<Operation>,
        exponent: Box<Operation>,
    },
    UnaryMinus {
        meta: OperationMeta,
        argument: Box<Operation>,
    },
    Function {
        meta: OperationMeta,
        name: String,
        arguments: Vec<Operation>,
    },
    BitwiseAnd {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
    BitwiseOr {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
    BitwiseNot {
        meta: OperationMeta,
        argument: Box<Operation>,
    },
    ShiftLeft {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
    ShiftRight {
        meta: OperationMeta,
        left: Box<Operation>,
        right: Box<Operation>,
    },
}

impl Operation {
    pub(crate) fn depends_on_variables(&self) -> bool {
        match self {
            Operation::Function { arguments, .. } => {
                arguments.iter().any(|a| a.depends_on_variables())
            }
            Operation::IntegerConstant(_) => false,
            Operation::FloatingPointConstant(_) => false,
            Operation::Addition { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
            Operation::Subtraction { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
            Operation::Multiplication { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
            Operation::Division {
                dividend, divisor, ..
            } => dividend.depends_on_variables() || divisor.depends_on_variables(),
            Operation::Modulo {
                dividend, divisor, ..
            } => dividend.depends_on_variables() || divisor.depends_on_variables(),
            Operation::Exponentiation { base, exponent, .. } => {
                base.depends_on_variables() || exponent.depends_on_variables()
            }
            Operation::UnaryMinus { argument, .. } => argument.depends_on_variables(),
            Operation::BitwiseAnd { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
            Operation::BitwiseOr { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
            Operation::BitwiseNot { argument, .. } => argument.depends_on_variables(),
            Operation::ShiftLeft { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
            Operation::ShiftRight { left, right, .. } => {
                left.depends_on_variables() || right.depends_on_variables()
            }
        }
    }
    
    pub(crate) fn get_data_type(&self) -> DataType {
        match self {
            Operation::IntegerConstant(_) => {DataType::Integer}
            Operation::FloatingPointConstant(_) => {DataType::FloatingPoint}
            Operation::Addition {meta, .. } => {meta.data_type}
            Operation::Subtraction {meta, .. } => {meta.data_type}
            Operation::Multiplication {meta, .. } => {meta.data_type}
            Operation::Division {meta, .. } => {meta.data_type}
            Operation::Modulo {meta, .. } => {meta.data_type}
            Operation::Exponentiation {meta, .. } => {meta.data_type}
            Operation::UnaryMinus {meta, .. } => {meta.data_type}
            Operation::Function {meta, .. } => {meta.data_type}
            Operation::BitwiseAnd {meta, .. } => {meta.data_type}
            Operation::BitwiseOr {meta, .. } => {meta.data_type}
            Operation::BitwiseNot {meta, .. } => {meta.data_type}
            Operation::ShiftLeft {meta, .. } => {meta.data_type}
            Operation::ShiftRight {meta, .. } => {meta.data_type}
        }
    }

    pub(crate) fn is_idempotent(&self) -> bool {
        match self {
            Operation::IntegerConstant(_) => true,
            Operation::FloatingPointConstant(_) => true,
            Operation::Addition { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
            Operation::Subtraction { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
            Operation::Multiplication { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
            Operation::Division {
                dividend, divisor, ..
            } => dividend.is_idempotent() && divisor.is_idempotent(),
            Operation::Modulo {
                dividend, divisor, ..
            } => dividend.is_idempotent() && divisor.is_idempotent(),
            Operation::Exponentiation { base, exponent, .. } => {
                base.is_idempotent() && exponent.is_idempotent()
            }
            Operation::UnaryMinus { argument, .. } => argument.is_idempotent(),
            Operation::Function { meta, .. } => meta.is_idempotent,
            Operation::BitwiseAnd { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
            Operation::BitwiseOr { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
            Operation::BitwiseNot { argument, .. } => argument.is_idempotent(),
            Operation::ShiftLeft { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
            Operation::ShiftRight { left, right, .. } => {
                left.is_idempotent() && right.is_idempotent()
            }
        }
    }
}