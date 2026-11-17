// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

pub fn barycentric(val1: f64, val2: f64, val3: f64, amount1: f64, amount2: f64) -> f64 {
    val1 + (val2 - val1) * amount1 + (val3 - val1) * amount2
}

pub fn catmull_rom(val1: f64, val2: f64, val3: f64, val4: f64, amount: f64) -> f64 {
    let amount_squared = amount * amount;
    let amount_cubed = amount_squared * amount;
    0.5 * (2.0 * val2
        + (val3 - val1) * amount
        + (2.0 * val1 - 5.0 * val2 + 4.0 * val3 - val4) * amount_squared
        + (3.0 * val2 - val1 - 3.0 * val3 + val4) * amount_cubed)
}

pub fn hermite(val1: f64, tangent1: f64, val2: f64, tangent2: f64, amount: f64) -> f64 {
    let v1 = val1;
    let v2 = val2;
    let t1 = tangent1;
    let t2 = tangent2;
    let s = amount;
    let s_cubed = s * s * s;
    let s_squared = s * s;

    if amount == 0.0 {
        val1
    } else if amount == 1.0 {
        val2
    } else {
        (2.0 * v1 - 2.0 * v2 + t2 + t1) * s_cubed
            + (3.0 * v2 - 3.0 * v1 - 2.0 * t1 - t2) * s_squared
            + t1 * s
            + v1
    }
}

pub fn lerp(val1: f64, val2: f64, amount: f64) -> f64 {
    val1 + (val2 - val1) * amount
}

pub fn smooth_step(val1: f64, val2: f64, amount: f64) -> f64 {
    hermite(val1, 0.0, val2, 0.0, amount.clamp(0.0, 1.0))
}

/// Solves the Pythagorean theorem for the missing side.
/// Pass `0.0` for the unknown: `(a, b, 0)` → hypotenuse; `(hyp, 0, leg)` or `(0, hyp, leg)` → other leg.
/// Returns `None` when all three are non-zero or the result would be imaginary.
pub fn pythagorean_theorem(a: f64, b: f64, c: f64) -> Option<f64> {
    if c == 0.0 {
        Some((a.powi(2) + b.powi(2)).sqrt())
    } else if b == 0.0 {
        let discriminant = a.powi(2) - c.powi(2);
        if discriminant < 0.0 { None } else { Some(discriminant.sqrt()) }
    } else if a == 0.0 {
        let discriminant = b.powi(2) - c.powi(2);
        if discriminant < 0.0 { None } else { Some(discriminant.sqrt()) }
    } else {
        None
    }
}

pub fn euclidean_theorem(a: f64, b: f64, take_sqrt: bool) -> f64 {
    if take_sqrt { (a * b).sqrt() } else { a * b }
}

pub fn triangle_altitude(p: f64, q: f64, take_sqrt: bool) -> f64 {
    euclidean_theorem(p, q, take_sqrt)
}

pub fn triangle_area(g: f64, h: f64) -> f64 {
    (g * h) / 2.0
}

pub fn circle_area(radius: f64) -> f64 {
    std::f64::consts::PI * radius.powi(2)
}

pub fn sphere_weight(radius: f64, specific_weight: f64) -> f64 {
    specific_weight * (4.0 / 3.0 * std::f64::consts::PI * radius.powi(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn pythagorean_hypotenuse() {
        let h = pythagorean_theorem(3.0, 4.0, 0.0).unwrap();
        assert!((h - 5.0).abs() < 1e-10);
    }

    #[test]
    fn pythagorean_leg() {
        let leg = pythagorean_theorem(5.0, 0.0, 3.0).unwrap();
        assert!((leg - 4.0).abs() < 1e-10);
    }

    #[test]
    fn pythagorean_imaginary_returns_none() {
        assert!(pythagorean_theorem(1.0, 0.0, 5.0).is_none());
    }

    #[test]
    fn pythagorean_all_nonzero_returns_none() {
        assert!(pythagorean_theorem(3.0, 4.0, 5.0).is_none());
    }
}
