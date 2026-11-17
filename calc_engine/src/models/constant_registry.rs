// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ConstantInfo {
    pub name: String,
    pub values: f64,
    pub is_over_writable: bool,
}
pub struct ConstantRegistry {
    constants: HashMap<String, ConstantInfo>,
}

impl ConstantRegistry {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }
    
    pub fn get_constant_value(&self, name:&str) -> Result<ConstantInfo, String> {
        if name.is_empty() {
            return Err("No name provided".into())
        }

        self.constants
            .get(name)
            .cloned()
            .ok_or_else(|| "Constant not found".into())
    }
    
    pub fn register(
        &mut self,
        name: &str,
        values: f64,
        is_over_writable: bool,
    ) -> Result<(), String> {
        if name.is_empty() {
            return Err("Name cannot be empty".into());
        }

        if let Some(existing) = self.constants.get(name) {
            if !existing.is_over_writable {
                return Err("Constant cannot be overwritten".into());
            }
        }

        self.constants.insert(
            name.to_string(),
            ConstantInfo {
                name: name.to_string(),
                values,
                is_over_writable,
            },
        );

        Ok(())
    }
}
