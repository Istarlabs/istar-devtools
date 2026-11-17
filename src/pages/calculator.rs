// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, AstNode, CalculatorState};
use calc_engine::engine::calculator_engine::CalculatorEngine;
use calc_engine::models::operation::Operation;
use slint::{ComponentHandle, Model, ModelRc, ToSharedString, VecModel};
use std::rc::Rc;

fn flatten_ast(op: &Operation, depth: i32, nodes: &mut Vec<AstNode>) {
    let label: slint::SharedString = match op {
        Operation::IntegerConstant(v) => v.to_string(),
        Operation::FloatingPointConstant(v) => format_float(*v),
        Operation::Addition { .. } => "+ (add)".to_string(),
        Operation::Subtraction { .. } => "- (sub)".to_string(),
        Operation::Multiplication { .. } => "× (mul)".to_string(),
        Operation::Division { .. } => "÷ (div)".to_string(),
        Operation::Modulo { .. } => "% (mod)".to_string(),
        Operation::Exponentiation { .. } => "^ (pow)".to_string(),
        Operation::UnaryMinus { .. } => "- (neg)".to_string(),
        Operation::Function { name, .. } => format!("fn {name}()"),
        Operation::BitwiseAnd { .. } => "& (and)".to_string(),
        Operation::BitwiseOr { .. } => "| (or)".to_string(),
        Operation::BitwiseNot { .. } => "~ (not)".to_string(),
        Operation::ShiftLeft { .. } => "<< (shl)".to_string(),
        Operation::ShiftRight { .. } => ">> (shr)".to_string(),
    }
    .into();

    nodes.push(AstNode { depth, label });

    match op {
        Operation::Addition { left, right, .. }
        | Operation::Subtraction { left, right, .. }
        | Operation::Multiplication { left, right, .. }
        | Operation::BitwiseAnd { left, right, .. }
        | Operation::BitwiseOr { left, right, .. }
        | Operation::ShiftLeft { left, right, .. }
        | Operation::ShiftRight { left, right, .. } => {
            flatten_ast(left, depth + 1, nodes);
            flatten_ast(right, depth + 1, nodes);
        }
        Operation::Division { dividend, divisor, .. }
        | Operation::Modulo { dividend, divisor, .. } => {
            flatten_ast(dividend, depth + 1, nodes);
            flatten_ast(divisor, depth + 1, nodes);
        }
        Operation::Exponentiation { base, exponent, .. } => {
            flatten_ast(base, depth + 1, nodes);
            flatten_ast(exponent, depth + 1, nodes);
        }
        Operation::UnaryMinus { argument, .. } | Operation::BitwiseNot { argument, .. } => {
            flatten_ast(argument, depth + 1, nodes);
        }
        Operation::Function { arguments, .. } => {
            for arg in arguments {
                flatten_ast(arg, depth + 1, nodes);
            }
        }
        _ => {}
    }
}

fn format_float(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.10}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn format_result(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.10}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

pub fn register(app: &AppWindow) {
    let engine = Rc::new(CalculatorEngine::new());

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_append(move |text| {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        let current = state.get_expression().as_str().to_string();
        state.set_expression((current + text.as_str()).into());
    });

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_clear(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        state.set_expression("".into());
        state.set_result("0".into());
        state.set_is_error(false);
        state.set_has_ast(false);
        state.set_ast_nodes(ModelRc::new(VecModel::default()));
    });

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_backspace(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        let expr = state.get_expression();
        let s = expr.as_str();
        let new_expr = match s.char_indices().next_back() {
            Some((idx, _)) => &s[..idx],
            None => "",
        };
        state.set_expression(new_expr.into());
    });

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_copy_display(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        let text = format!(
            "{} = {}",
            state.get_expression().as_str(),
            state.get_result().as_str()
        );
        let mut clipboard = arboard::Clipboard::new().unwrap();
        let _ = clipboard.set_text(text);
    });

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_copy_ast(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        let nodes = state.get_ast_nodes();
        let mut text = String::new();
        for i in 0..nodes.row_count() {
            if let Some(node) = nodes.row_data(i) {
                let indent = "  ".repeat(node.depth as usize);
                text.push_str(&format!("{}{}\n", indent, node.label.as_str()));
            }
        }
        let mut clipboard = arboard::Clipboard::new().unwrap();
        let _ = clipboard.set_text(text.trim_end().to_string());
    });

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_keyboard_input(move |text| {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        let ch = text.as_str();
        if ch.len() == 1 {
            let c = ch.chars().next().unwrap();
            if c.is_ascii_digit() || c.is_ascii_alphabetic() || "+-*/.%^(),&|~<>".contains(c) {
                let current = state.get_expression().as_str().to_string();
                state.set_expression((current + ch).into());
            }
        }
    });

    let ui_handle = app.as_weak();
    app.global::<CalculatorState>().on_calculate(move || {
        let ui = ui_handle.unwrap();
        let state = ui.global::<CalculatorState>();
        let expression = state.get_expression();
        let expr_str = expression.as_str();

        if expr_str.is_empty() {
            return;
        }

        let op = match engine.parse(expr_str) {
            Ok(op) => op,
            Err(e) => {
                state.set_result(format!("Error: {e}").to_shared_string());
                state.set_is_error(true);
                state.set_has_ast(false);
                state.set_ast_nodes(ModelRc::new(VecModel::default()));
                return;
            }
        };

        let result = engine.execute(&op);

        let mut nodes: Vec<AstNode> = Vec::new();
        flatten_ast(&op, 0, &mut nodes);

        state.set_result(format_result(result).to_shared_string());
        state.set_is_error(false);
        state.set_ast_nodes(ModelRc::new(VecModel::from(nodes)));
        state.set_has_ast(true);
    });
}
