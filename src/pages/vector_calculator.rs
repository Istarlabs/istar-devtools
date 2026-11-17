// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, VectorCalculatorState, VectorInput as SlintVectorInput, VectorResult as SlintVectorResult};
use math::algebra::vector::Vector;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

const CANVAS_W: f64 = 450.0;
const CANVAS_H: f64 = 400.0;
const ORIGIN_X: f64 = CANVAS_W / 2.0;
const ORIGIN_Y: f64 = CANVAS_H / 2.0;

struct SharedCalcState {
    va: Option<Vector>,
    vb: Option<Vector>,
    vr: Option<Vector>,
    lerp_point: Option<Vector>,
}

impl Default for SharedCalcState {
    fn default() -> Self {
        Self { va: None, vb: None, vr: None, lerp_point: None }
    }
}

fn to_canvas(sx: f64, sy: f64, scale: f64) -> (f32, f32) {
    let cx = sx / scale * (CANVAS_W / 2.0) + ORIGIN_X;
    let cy = -sy / scale * (CANVAS_H / 2.0) + ORIGIN_Y;
    (cx as f32, cy as f32)
}

fn project_2d(a: [f64; 3], plane: i32) -> (f64, f64) {
    const C30: f64 = 0.8660254037844387;
    const S30: f64 = 0.5;
    match plane {
        0 => (a[0], a[1]),
        1 => (a[0], a[2]),
        2 => (a[1], a[2]),
        _ => (a[0] - a[2] * C30, a[1] + a[2] * S30),
    }
}

fn project(v: Vector, plane: i32, scale: f64) -> (f32, f32) {
    let (sx, sy) = project_2d(v.as_array(), plane);
    to_canvas(sx, sy, scale)
}

fn arrowhead(ox: f32, oy: f32, tx: f32, ty: f32) -> (f32, f32, f32, f32) {
    let dx = tx - ox;
    let dy = ty - oy;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 {
        return (tx, ty, tx, ty);
    }
    let (ndx, ndy) = (dx / len, dy / len);
    let base_x = tx - ndx * 14.0;
    let base_y = ty - ndy * 14.0;
    let (px, py) = (-ndy, ndx);
    (base_x + px * 6.0, base_y + py * 6.0, base_x - px * 6.0, base_y - py * 6.0)
}

fn compute_scale(vectors: &[Vector], plane: i32) -> f64 {
    let mut max = 1.0_f64;
    for v in vectors {
        let (sx, sy) = project_2d(v.as_array(), plane);
        max = max.max(sx.abs()).max(sy.abs());
    }
    (max * 1.25).max(1.0)
}

fn build_canvas_points(
    shared: &SharedCalcState,
    plane: i32,
    shows_b: bool,
    shows_result: bool,
) -> ModelRc<f32> {
    let mut vis: Vec<Vector> = Vec::new();
    if let Some(va) = shared.va { vis.push(va); }
    if shows_b { if let Some(vb) = shared.vb { vis.push(vb); } }
    if shows_result { if let Some(vr) = shared.vr { vis.push(vr); } }
    if let Some(lp) = shared.lerp_point { vis.push(lp); }

    let scale = if vis.is_empty() { 1.0 } else { compute_scale(&vis, plane) };
    let (ox, oy) = (ORIGIN_X as f32, ORIGIN_Y as f32);

    let mut pts = vec![0.0f32; 26];

    if let Some(va) = shared.va {
        let (ax, ay) = project(va, plane, scale);
        pts[0] = ax; pts[1] = ay;
        let (h1x, h1y, h2x, h2y) = arrowhead(ox, oy, ax, ay);
        pts[2] = h1x; pts[3] = h1y; pts[4] = h2x; pts[5] = h2y;
    }
    if shows_b {
        if let Some(vb) = shared.vb {
            let (bx, by) = project(vb, plane, scale);
            pts[6] = bx; pts[7] = by;
            let (h1x, h1y, h2x, h2y) = arrowhead(ox, oy, bx, by);
            pts[8] = h1x; pts[9] = h1y; pts[10] = h2x; pts[11] = h2y;
        }
    }
    if shows_result {
        if let Some(vr) = shared.vr {
            let (rx, ry) = project(vr, plane, scale);
            pts[12] = rx; pts[13] = ry;
            let (h1x, h1y, h2x, h2y) = arrowhead(ox, oy, rx, ry);
            pts[14] = h1x; pts[15] = h1y; pts[16] = h2x; pts[17] = h2y;
        }
    }
    if let Some(lp) = shared.lerp_point {
        let (lx, ly) = project(lp, plane, scale);
        pts[18] = lx; pts[19] = ly;
    }

    ModelRc::new(VecModel::from(pts))
}

fn format_float(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.6}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn format_vec(v: Vector) -> String {
    let a = v.as_array();
    format!("({}, {}, {})", format_float(a[0]), format_float(a[1]), format_float(a[2]))
}

fn do_calculate(app: &AppWindow, shared: &mut SharedCalcState) {
    let state = app.global::<VectorCalculatorState>();
    let va_inp = state.get_vector_a();
    let vb_inp = state.get_vector_b();
    let scalar_str = state.get_scalar_input().to_string();
    let lerp_t = state.get_lerp_t() as f64;
    let op_idx = state.get_operation_index();
    let plane = state.get_view_plane_index();

    let va = match parse_vector(&va_inp) {
        Ok(v) => v,
        Err(e) => {
            state.set_result(error_result(format!("A: {e}")));
            return;
        }
    };

    let needs_b = op_idx != 4 && op_idx != 5;
    let vb = if needs_b {
        match parse_vector(&vb_inp) {
            Ok(v) => Some(v),
            Err(e) => {
                state.set_result(error_result(format!("B: {e}")));
                return;
            }
        }
    } else {
        None
    };

    let (result_vec, scalar_result, lerp_point, shows_result_vector) = match op_idx {
        0 => (Some(va + vb.unwrap()), 0.0f64, None, true),
        1 => (Some(va - vb.unwrap()), 0.0, None, true),
        2 => (None, va.scalar_product(vb.unwrap()), None, false),
        3 => (Some(va.cross_product(vb.unwrap())), 0.0, None, true),
        4 => {
            let s = match scalar_str.trim().parse::<f64>() {
                Ok(v) => v,
                Err(e) => {
                    state.set_result(error_result(format!("Scalar: {e}")));
                    return;
                }
            };
            let mut r = va;
            r.scalar_multiply(s);
            (Some(r), 0.0, None, true)
        }
        5 => match va.normalize() {
            Ok(r) => (Some(r), 0.0, None, true),
            Err(e) => {
                state.set_result(error_result(e));
                return;
            }
        },
        6 => match va.angle_between(vb.unwrap()) {
            Ok(r) => (None, r.to_degrees(), None, false),
            Err(e) => {
                state.set_result(error_result(e));
                return;
            }
        },
        7 => (None, va.distance(vb.unwrap()), None, false),
        8 => {
            let normal = match vb.unwrap().normalize() {
                Ok(n) => n,
                Err(e) => {
                    state.set_result(error_result(format!("Normal: {e}")));
                    return;
                }
            };
            (Some(va.reflect(normal)), 0.0, None, true)
        }
        9 => {
            let r = Vector::lerp(va, vb.unwrap(), lerp_t);
            (Some(r), 0.0, Some(r), true)
        }
        _ => (None, 0.0, None, false),
    };

    let ra = result_vec.map(|v| v.as_array()).unwrap_or([0.0; 3]);
    let result_length = result_vec.map(|v| v.get_length()).unwrap_or(0.0);

    let dot = if let Some(vb) = vb {
        if op_idx == 2 { scalar_result } else { va.scalar_product(vb) }
    } else {
        0.0
    };

    let angle_deg = match op_idx {
        6 => scalar_result,
        _ => vb.and_then(|b| va.angle_between(b).ok()).map(|a| a.to_degrees()).unwrap_or(0.0),
    };

    let displayed_scalar = match op_idx {
        2 => scalar_result,
        6 => scalar_result,
        7 => scalar_result,
        _ => 0.0,
    };

    let normalized_str: slint::SharedString = match va.normalize() {
        Ok(n) => format_vec(n).into(),
        Err(_) => "—".into(),
    };

    let slint_result = SlintVectorResult {
        x: format_float(ra[0]).into(),
        y: format_float(ra[1]).into(),
        z: format_float(ra[2]).into(),
        length: format_float(result_length).into(),
        dot: format_float(dot).into(),
        angle_deg: format!("{:.4}°", angle_deg).into(),
        scalar_result: format_float(displayed_scalar).into(),
        normalized: normalized_str,
        is_valid: true,
        error: "".into(),
    };

    state.set_result(slint_result);
    state.set_shows_result_vector(shows_result_vector);
    state.set_lerp_t_display(format!("{:.2}", lerp_t).into());

    shared.va = Some(va);
    shared.vb = vb;
    shared.vr = result_vec;
    shared.lerp_point = lerp_point;

    let points = build_canvas_points(shared, plane, needs_b, shows_result_vector);
    state.set_canvas_points(points);
}

fn parse_vector(inp: &SlintVectorInput) -> Result<Vector, String> {
    Vector::from_string(inp.x.as_str(), inp.y.as_str(), inp.z.as_str())
        .map_err(|e| e.to_string())
}

fn error_result(msg: impl Into<slint::SharedString>) -> SlintVectorResult {
    SlintVectorResult {
        is_valid: false,
        error: msg.into(),
        x: "".into(),
        y: "".into(),
        z: "".into(),
        length: "".into(),
        dot: "".into(),
        angle_deg: "".into(),
        scalar_result: "".into(),
        normalized: "".into(),
    }
}

pub fn register(app: &AppWindow) {
    let shared = Rc::new(RefCell::new(SharedCalcState::default()));

    // Initial calculation with default vectors
    do_calculate(app, &mut shared.borrow_mut());

    // on_calculate
    let weak = app.as_weak();
    let shared_calc = shared.clone();
    app.global::<VectorCalculatorState>().on_calculate(move || {
        let app = weak.upgrade().unwrap();
        do_calculate(&app, &mut shared_calc.borrow_mut());
    });

    // on_view_plane_changed — re-project with new plane, no recalculation
    let weak = app.as_weak();
    let shared_view = shared.clone();
    app.global::<VectorCalculatorState>().on_view_plane_changed(move |plane| {
        let app = weak.upgrade().unwrap();
        let state = app.global::<VectorCalculatorState>();
        state.set_view_plane_index(plane);
        let op_idx = state.get_operation_index();
        let shows_b = op_idx != 4 && op_idx != 5;
        let shows_r = state.get_shows_result_vector();
        let points = build_canvas_points(&shared_view.borrow(), plane, shows_b, shows_r);
        state.set_canvas_points(points);
    });

    // on_swap_vectors
    let weak = app.as_weak();
    let shared_swap = shared.clone();
    app.global::<VectorCalculatorState>().on_swap_vectors(move || {
        let app = weak.upgrade().unwrap();
        let state = app.global::<VectorCalculatorState>();
        let va = state.get_vector_a();
        let vb = state.get_vector_b();
        state.set_vector_a(vb);
        state.set_vector_b(va);
        do_calculate(&app, &mut shared_swap.borrow_mut());
    });

    // on_copy_result
    let weak = app.as_weak();
    app.global::<VectorCalculatorState>().on_copy_result(move || {
        let app = weak.upgrade().unwrap();
        let state = app.global::<VectorCalculatorState>();
        let r = state.get_result();
        let text = if r.is_valid {
            if state.get_shows_result_vector() {
                format!(
                    "X: {}  Y: {}  Z: {}\nLength: {}\nDot A·B: {}\nAngle: {}\nNormalized A: {}",
                    r.x, r.y, r.z, r.length, r.dot, r.angle_deg, r.normalized
                )
            } else {
                format!("{}", r.scalar_result)
            }
        } else {
            r.error.to_string()
        };
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text);
        }
    });
}
