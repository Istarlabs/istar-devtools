// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::data_type::DataType;
use crate::models::operation::Operation;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OperationMeta {
    pub data_type: DataType,
    pub depends_on_variables: bool,
    pub is_idempotent: bool,
}

impl OperationMeta {
    pub fn new(data_type: DataType, left: &Operation, right: &Operation) -> Self {
        Self {
            data_type,
            depends_on_variables: left.depends_on_variables() || right.depends_on_variables(),
            is_idempotent: left.is_idempotent() && right.is_idempotent(),
        }
    }

    pub fn unary(data_type: DataType, arg: &Operation) -> Self {
        Self {
            data_type,
            depends_on_variables: arg.depends_on_variables(),
            is_idempotent: arg.is_idempotent(),
        }
    }

    pub fn binary(data_type: DataType, left: &Operation, right: &Operation) -> Self {
        Self::new(data_type, left, right)
    }
}