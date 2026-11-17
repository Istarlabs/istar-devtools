// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    a: f64,
    b: f64,
    c: f64,
    alpha: f64,
    beta: f64,
    gamma: f64,
}

impl Triangle {
    pub fn new(a: f64, b: f64, c: f64, alpha: f64, beta: f64, gamma: f64) -> Self {
        Self { a, b, c, alpha, beta, gamma }
    }

    pub fn get_perimeter(&self) -> f64 {
        self.a + self.b + self.c
    }

    pub fn get_perimeter_half(&self) -> f64 {
        self.get_perimeter() * 0.5
    }
}
