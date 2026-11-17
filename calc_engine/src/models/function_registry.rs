// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

pub struct FunctionInfo {
    pub name: String,
    pub param_count: usize,
    pub is_idempotent: bool,
    pub is_rewritable: bool,
    pub func: Box<dyn Fn(Vec<f64>) -> f64>,
}

pub struct FunctionRegistry {
    functions: HashMap<String, FunctionInfo>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }
    pub fn get_function_info(&self, name: &str) -> Option<&FunctionInfo> {
        self.functions.get(name)
    }
    
    pub fn register(
        &mut self,
        name: &str,
        param_count: usize,
        is_idempotent: bool,
        is_rewritable: bool,
        func: impl Fn(Vec<f64>) -> f64 + 'static,
    ) -> Result<(), String> {
        if name.is_empty() {
            return Err("Name cannot be empty".into());
        }
        if let Some(existing) = self.functions.get(name) {
            if !existing.is_rewritable {
                return Err("The function cannot be overwritten".into());
            }
            if existing.param_count != param_count {
                return Err("Parameter count cannot change on overwrite".into());
            }
        }
        self.functions.insert(
            name.to_string(),
            FunctionInfo {
                name: name.to_string(),
                param_count,
                is_idempotent,
                is_rewritable,
                func: Box::new(func),
            },
        );
        Ok(())
    }

    pub fn invoke(&self, name: &str, args: Vec<f64>) -> Option<f64> {
        self.functions.get(name).map(|info| (info.func)(args))
    }

    pub fn is_valid_function(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        self.functions.contains_key(name)
    }
}
