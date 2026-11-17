// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use calc_engine::engine::calculator_engine::CalculatorEngine;

fn eval(expr: &str) -> f64 {
    CalculatorEngine::new()
        .eval(expr)
        .unwrap_or_else(|e| panic!("eval({expr:?}) failed: {e}"))
}

fn eval_approx(expr: &str, expected: f64) {
    let result = eval(expr);
    assert!(
        (result - expected).abs() < 1e-9,
        "eval({expr:?}) = {result}, expected {expected}"
    );
}

// --- basic arithmetic ---

#[test]
fn integer_addition() {
    assert_eq!(eval("2+3"), 5.0);
}

#[test]
fn integer_subtraction() {
    assert_eq!(eval("10-4"), 6.0);
}

#[test]
fn integer_multiplication() {
    assert_eq!(eval("3*4"), 12.0);
}

#[test]
fn integer_division() {
    assert_eq!(eval("10/2"), 5.0);
}

#[test]
fn modulo() {
    assert_eq!(eval("10%3"), 1.0);
}

#[test]
fn exponentiation() {
    assert_eq!(eval("2^10"), 1024.0);
}

// --- operator precedence ---

#[test]
fn mul_before_add() {
    // 2 + 3*4 must be 14, not 20
    assert_eq!(eval("2+3*4"), 14.0);
}

#[test]
fn div_before_sub() {
    // 20 - 10/2 must be 15, not 5
    assert_eq!(eval("20-10/2"), 15.0);
}

#[test]
fn chained_mixed_precedence() {
    // 1 + 2*3 - 4/2 = 1 + 6 - 2 = 5
    assert_eq!(eval("1+2*3-4/2"), 5.0);
}

#[test]
fn exponent_before_mul() {
    // 2 * 3^2 = 2 * 9 = 18
    assert_eq!(eval("2*3^2"), 18.0);
}

// --- parentheses ---

#[test]
fn parentheses_override_precedence() {
    // (2+3)*4 = 20
    assert_eq!(eval("(2+3)*4"), 20.0);
}

#[test]
fn nested_parentheses() {
    // ((2+3))*4 = 20
    assert_eq!(eval("((2+3))*4"), 20.0);
}

#[test]
fn inner_parens_only() {
    assert_eq!(eval("(5)"), 5.0);
}

// --- unary minus ---

#[test]
fn unary_minus_literal() {
    assert_eq!(eval("-5"), -5.0);
}

#[test]
fn unary_minus_expression() {
    assert_eq!(eval("-(3+2)"), -5.0);
}

#[test]
fn unary_minus_after_operator() {
    // 3 + -2 = 1
    assert_eq!(eval("3+-2"), 1.0);
}

#[test]
fn unary_minus_times_value() {
    // -3 * 4 = -12
    assert_eq!(eval("-3*4"), -12.0);
}

// --- floating point ---

#[test]
fn float_addition() {
    eval_approx("1.5+2.5", 4.0);
}

#[test]
fn float_division() {
    eval_approx("1/3", 1.0 / 3.0);
}

// --- scientific notation ---

#[test]
fn scientific_notation_positive() {
    eval_approx("1e3", 1000.0);
}

#[test]
fn scientific_notation_negative_exp() {
    eval_approx("1e-3", 0.001);
}

// --- built-in constants (covers constant_registry insert bug fix) ---

#[test]
fn constant_pi() {
    eval_approx("PI", std::f64::consts::PI);
}

#[test]
fn constant_e() {
    eval_approx("E", std::f64::consts::E);
}

#[test]
fn constant_in_expression() {
    // 2 * PI
    eval_approx("2*PI", 2.0 * std::f64::consts::PI);
}

// --- built-in functions (covers register_default_functions never-called bug fix) ---

#[test]
fn sin_zero() {
    eval_approx("sin(0)", 0.0);
}

#[test]
fn cos_zero() {
    eval_approx("cos(0)", 1.0);
}

#[test]
fn tan_zero() {
    eval_approx("tan(0)", 0.0);
}

#[test]
fn sqrt_four() {
    eval_approx("sqrt(4)", 2.0);
}

#[test]
fn ln_e() {
    eval_approx("ln(E)", 1.0);
}

#[test]
fn log_base2_eight() {
    // log(8, 2) — first arg is value, second is base
    eval_approx("log(8,2)", 3.0);
}

#[test]
fn sqrt_in_expression() {
    // sqrt(9) + 1 = 4
    eval_approx("sqrt(9)+1", 4.0);
}

#[test]
fn function_with_expression_arg() {
    // sin(0 * 5) = sin(0) = 0
    eval_approx("sin(0*5)", 0.0);
}

#[test]
fn nested_function_calls() {
    // sqrt(sqrt(16)) = sqrt(4) = 2
    eval_approx("sqrt(sqrt(16))", 2.0);
}

// --- factorial ---

#[test]
fn factorial_five() {
    eval_approx("fact(5)", 120.0);
}

#[test]
fn factorial_zero() {
    eval_approx("fact(0)", 1.0);
}

// --- unary minus in function args (covers separator is_formular_sub_part bug fix) ---

#[test]
fn negative_arg_to_function() {
    // sin(-PI/2) = -1 (uses unary minus after separator-like start)
    eval_approx("sin(-PI/2)", -1.0);
}

// --- hex / binary literals ---

#[test]
fn hex_literal_basic() {
    assert_eq!(eval("0xFF"), 255.0);
}

#[test]
fn hex_literal_lowercase() {
    assert_eq!(eval("0xff"), 255.0);
}

#[test]
fn hex_literal_arithmetic() {
    assert_eq!(eval("0x1A * 2"), 52.0);
}

#[test]
fn hex_literal_subtraction() {
    assert_eq!(eval("0xFF - 0x0F"), 240.0);
}

#[test]
fn binary_literal_basic() {
    assert_eq!(eval("0b1010"), 10.0);
}

#[test]
fn binary_literal_addition() {
    assert_eq!(eval("0b1010 + 0b0101"), 15.0);
}

#[test]
fn binary_literal_mixed_decimal() {
    assert_eq!(eval("0b1111 + 1"), 16.0);
}

// --- bitwise operators ---

#[test]
fn bitwise_and() {
    assert_eq!(eval("0b1111 & 0b1010"), 10.0);
}

#[test]
fn bitwise_or() {
    assert_eq!(eval("0b0101 | 0b1010"), 15.0);
}

#[test]
fn bitwise_not_zero() {
    assert_eq!(eval("~0"), -1.0);
}

#[test]
fn bitwise_not_value() {
    assert_eq!(eval("~0b1111"), -16.0);
}

#[test]
fn shift_left() {
    assert_eq!(eval("1 << 4"), 16.0);
}

#[test]
fn shift_right() {
    assert_eq!(eval("256 >> 2"), 64.0);
}

#[test]
fn shift_left_binary() {
    assert_eq!(eval("0b0001 << 3"), 8.0);
}

#[test]
fn xor_function() {
    assert_eq!(eval("xor(0xFF, 0x0F)"), 240.0);
}

#[test]
fn xor_function_binary() {
    assert_eq!(eval("xor(0b1010, 0b1100)"), 6.0);
}

#[test]
fn bitwise_precedence_and_over_or() {
    // 5 | 3 & 6 → 5 | (3 & 6) = 5 | 2 = 7
    assert_eq!(eval("5 | 3 & 6"), 7.0);
}

#[test]
fn bitwise_with_arithmetic() {
    // arithmetic binds tighter than bitwise: 1 + 2 & 3 = (1+2) & 3 = 3 & 3 = 3
    assert_eq!(eval("1 + 2 & 3"), 3.0);
}

// --- error cases ---

#[test]
fn mismatched_right_bracket() {
    assert!(CalculatorEngine::new().eval("2+3)").is_err());
}

#[test]
fn mismatched_left_bracket() {
    assert!(CalculatorEngine::new().eval("(2+3").is_err());
}

#[test]
fn empty_input() {
    assert!(CalculatorEngine::new().eval("").is_err());
}
