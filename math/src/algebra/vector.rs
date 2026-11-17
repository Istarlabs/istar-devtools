// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::algebra::helper;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vector {
    x: f64,
    y: f64,
    z: f64,
}

impl Vector {
    pub const ZERO: Vector = Vector { x: 0.0, y: 0.0, z: 0.0 };
    pub const MIN: Vector = Vector { x: f64::MIN, y: f64::MIN, z: f64::MIN };
    pub const MAX: Vector = Vector { x: f64::MAX, y: f64::MAX, z: f64::MAX };
    pub const EPSILON: Vector = Vector { x: f64::EPSILON, y: f64::EPSILON, z: f64::EPSILON };

    pub const UNIT_X: Vector = Vector { x: 1.0, y: 0.0, z: 0.0 };
    pub const UNIT_Y: Vector = Vector { x: 0.0, y: 1.0, z: 0.0 };
    pub const UNIT_Z: Vector = Vector { x: 0.0, y: 0.0, z: 1.0 };

    pub const UP: Vector = Vector { x: 0.0, y: 1.0, z: 0.0 };
    pub const DOWN: Vector = Vector { x: 0.0, y: -1.0, z: 0.0 };
    pub const RIGHT: Vector = Vector { x: 1.0, y: 0.0, z: 0.0 };
    pub const LEFT: Vector = Vector { x: -1.0, y: 0.0, z: 0.0 };
    pub const FORWARD: Vector = Vector { x: 0.0, y: 0.0, z: -1.0 };
    pub const BACKWARD: Vector = Vector { x: 0.0, y: 0.0, z: 1.0 };

    /// All-ones vector (1, 1, 1). Not a unit vector — length is sqrt(3).
    pub const ONE: Vector = Vector { x: 1.0, y: 1.0, z: 1.0 };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector { x, y, z }
    }

    pub fn from_value(val: f64) -> Self {
        Vector { x: val, y: val, z: val }
    }

    pub fn from_string(x: &str, y: &str, z: &str) -> Result<Self, std::num::ParseFloatError> {
        Ok(Vector::new(x.parse()?, y.parse()?, z.parse()?))
    }

    pub fn from_array(vector_axis: &[f64; 3]) -> Self {
        Vector { x: vector_axis[0], y: vector_axis[1], z: vector_axis[2] }
    }

    pub fn as_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    pub fn get_squared_length(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn get_length(&self) -> f64 {
        self.get_squared_length().sqrt()
    }

    pub fn normalize(&self) -> Result<Self, String> {
        let length = self.get_length();
        if length == 0.0 {
            Err("Can't normalize a zero-length vector.".to_string())
        } else {
            Ok(Vector::new(self.x / length, self.y / length, self.z / length))
        }
    }

    pub fn normalize_self(&mut self) -> Result<(), String> {
        let normalized = self.normalize()?;
        self.x = normalized.x;
        self.y = normalized.y;
        self.z = normalized.z;
        Ok(())
    }

    /// Scales this vector in-place by `scalar`.
    pub fn scalar_multiply(&mut self, scalar: f64) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }

    pub fn cross_product(&self, other: Vector) -> Self {
        Vector {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn distance_squared(&self, other: Vector) -> f64 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        let z = self.z - other.z;
        x * x + y * y + z * z
    }

    pub fn distance(&self, other: Vector) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn scalar_product(&self, other: Vector) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn is_orthogonal(&self, other: Vector) -> bool {
        let dot = self.scalar_product(other).abs();
        let scale = (self.get_length() * other.get_length()).max(1.0);
        dot <= f64::EPSILON * scale
    }

    pub fn is_unit_vector(&self) -> bool {
        (self.get_length() - 1.0).abs() <= f64::EPSILON
    }

    pub fn is_back_face(&self, line_of_sight: Vector) -> bool {
        self.scalar_product(line_of_sight) < 0.0
    }

    pub fn mixed_product(&self, v2: Vector, v3: Vector) -> f64 {
        self.cross_product(v2).scalar_product(v3)
    }

    pub fn interpolate(&self, other: Vector, factor: f64, allow_extrapolation: bool) -> Result<Vector, String> {
        if !allow_extrapolation && (factor < 0.0 || factor > 1.0) {
            return Err(
                "Interpolation factor must be between 0 and 1 when extrapolation is not allowed."
                    .to_string(),
            );
        }
        Ok(Vector::new(
            self.x + factor * (other.x - self.x),
            self.y + factor * (other.y - self.y),
            self.z + factor * (other.z - self.z),
        ))
    }

    pub fn interpolate_without_extrapolation(&self, other: Vector, factor: f64) -> Result<Vector, String> {
        self.interpolate(other, factor, false)
    }

    /// Returns the angle (in radians) between this vector and `other`.
    /// Returns `Err` if either vector has zero length.
    pub fn angle_between(&self, other: Vector) -> Result<f64, String> {
        let a = self.normalize()?;
        let b = other.normalize()?;
        Ok(a.scalar_product(b).clamp(-1.0, 1.0).acos())
    }

    /// Reflects this vector off a surface with the given unit normal.
    pub fn reflect(&self, normal: Vector) -> Self {
        let dot = self.scalar_product(normal);
        Vector::new(
            self.x - (2.0 * dot) * normal.x,
            self.y - (2.0 * dot) * normal.y,
            self.z - (2.0 * dot) * normal.z,
        )
    }

    pub fn clamp(&self, min: Vector, max: Vector) -> Self {
        Vector::new(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
        )
    }

    pub fn hermite(v1: Vector, tangent1: Vector, v2: Vector, tangent2: Vector, amount: f64) -> Self {
        Vector::new(
            helper::hermite(v1.x, tangent1.x, v2.x, tangent2.x, amount),
            helper::hermite(v1.y, tangent1.y, v2.y, tangent2.y, amount),
            helper::hermite(v1.z, tangent1.z, v2.z, tangent2.z, amount),
        )
    }

    pub fn lerp(v1: Vector, v2: Vector, amount: f64) -> Self {
        Vector::new(
            helper::lerp(v1.x, v2.x, amount),
            helper::lerp(v1.y, v2.y, amount),
            helper::lerp(v1.z, v2.z, amount),
        )
    }

    pub fn smooth_step(v1: Vector, v2: Vector, amount: f64) -> Self {
        Vector::new(
            helper::smooth_step(v1.x, v2.x, amount),
            helper::smooth_step(v1.y, v2.y, amount),
            helper::smooth_step(v1.z, v2.z, amount),
        )
    }

    pub fn barycentric(v1: Vector, v2: Vector, v3: Vector, amount1: f64, amount2: f64) -> Self {
        Vector::new(
            helper::barycentric(v1.x, v2.x, v3.x, amount1, amount2),
            helper::barycentric(v1.y, v2.y, v3.y, amount1, amount2),
            helper::barycentric(v1.z, v2.z, v3.z, amount1, amount2),
        )
    }

    pub fn to_formatted_string(&self, format: &str) -> String {
        if format.is_empty() {
            return format!("X: {:.2}  Y: {:.2}  Z: {:.2}", self.x, self.y, self.z);
        }
        match format.chars().next().unwrap() {
            'x' => self.x.to_string(),
            'y' => self.y.to_string(),
            'z' => self.z.to_string(),
            'r' => format!("X: {}  Y: {}  Z: {}", self.x, self.y, self.z),
            _ => format!("X: {:.2}  Y: {:.2}  Z: {:.2}", self.x, self.y, self.z),
        }
    }
}

impl std::ops::Add for Vector {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Vector::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl std::ops::Sub for Vector {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl std::ops::Mul for Vector {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Vector::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl std::ops::MulAssign for Vector {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}

impl std::ops::Div for Vector {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Vector::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z)
    }
}

impl std::ops::DivAssign for Vector {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}

impl std::ops::Neg for Vector {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Vector::new(-self.x, -self.y, -self.z)
    }
}

impl PartialEq for Vector {
    fn eq(&self, other: &Self) -> bool {
        (self.x - other.x).abs() <= f64::EPSILON
            && (self.y - other.y).abs() <= f64::EPSILON
            && (self.z - other.z).abs() <= f64::EPSILON
    }
}

impl Eq for Vector {}

impl std::ops::Index<usize> for Vector {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Index out of bounds for Vector"),
        }
    }
}

impl From<[f64; 3]> for Vector {
    fn from(arr: [f64; 3]) -> Self {
        Vector::new(arr[0], arr[1], arr[2])
    }
}

impl From<f64> for Vector {
    fn from(scalar: f64) -> Self {
        Vector::new(scalar, scalar, scalar)
    }
}

impl Display for Vector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "X: {:.2}  Y: {:.2}  Z: {:.2}", self.x, self.y, self.z)
    }
}

impl PartialOrd for Vector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.get_squared_length().partial_cmp(&other.get_squared_length())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let v = Vector::new(1.0, 2.0, 3.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);
    }

    #[test]
    fn test_length() {
        let v = Vector::new(3.0, 4.0, 0.0);
        assert_eq!(v.get_length(), 5.0);
    }

    #[test]
    fn test_add() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = Vector::new(4.0, 5.0, 6.0);
        assert_eq!(v1 + v2, Vector::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn test_cross_product() {
        let v1 = Vector::new(1.0, 0.0, 0.0);
        let v2 = Vector::new(0.0, 1.0, 0.0);
        assert_eq!(v1.cross_product(v2), Vector::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_normalize() {
        let v = Vector::new(3.0, 0.0, 0.0);
        assert_eq!(v.normalize().unwrap(), Vector::UNIT_X);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let v = Vector::ZERO;
        assert!(v.normalize().is_err());
    }

    #[test]
    fn test_scalar_product() {
        let v1 = Vector::new(1.0, 2.0, 3.0);
        let v2 = Vector::new(4.0, 5.0, 6.0);
        assert_eq!(v1.scalar_product(v2), 32.0);
    }

    #[test]
    fn test_is_orthogonal() {
        assert!(Vector::UNIT_X.is_orthogonal(Vector::UNIT_Y));
    }

    #[test]
    fn test_is_unit_vector() {
        assert!(Vector::UNIT_X.is_unit_vector());
        assert!(!Vector::new(2.0, 0.0, 0.0).is_unit_vector());
        assert!(!Vector::ZERO.is_unit_vector());
    }

    #[test]
    fn test_one_is_not_unit_vector() {
        assert!(!Vector::ONE.is_unit_vector());
    }

    #[test]
    fn test_angle_between_orthogonal() {
        let angle = Vector::UNIT_X.angle_between(Vector::UNIT_Y).unwrap();
        assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn test_angle_between_zero_vector_returns_err() {
        assert!(Vector::UNIT_X.angle_between(Vector::ZERO).is_err());
    }
}
