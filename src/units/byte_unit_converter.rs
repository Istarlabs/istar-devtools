// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteUnit {
    // Decimal (1000-based)
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,
    Exabyte,
    // Binary (1024-based)
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    Pebibyte,
    Exbibyte,
}

impl ByteUnit {
    pub fn factor(&self) -> u64 {
        match self {
            ByteUnit::Byte => 1,
            ByteUnit::Kilobyte => 1_000,
            ByteUnit::Megabyte => 1_000_000,
            ByteUnit::Gigabyte => 1_000_000_000,
            ByteUnit::Terabyte => 1_000_000_000_000,
            ByteUnit::Petabyte => 1_000_000_000_000_000,
            ByteUnit::Exabyte => 1_000_000_000_000_000_000,
            ByteUnit::Kibibyte => 1_024,
            ByteUnit::Mebibyte => 1_048_576,
            ByteUnit::Gibibyte => 1_073_741_824,
            ByteUnit::Tebibyte => 1_099_511_627_776,
            ByteUnit::Pebibyte => 1_125_899_906_842_624,
            ByteUnit::Exbibyte => 1_152_921_504_606_846_976,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            ByteUnit::Byte => "B",
            ByteUnit::Kilobyte => "KB",
            ByteUnit::Megabyte => "MB",
            ByteUnit::Gigabyte => "GB",
            ByteUnit::Terabyte => "TB",
            ByteUnit::Petabyte => "PB",
            ByteUnit::Exabyte => "EB",
            ByteUnit::Kibibyte => "KiB",
            ByteUnit::Mebibyte => "MiB",
            ByteUnit::Gibibyte => "GiB",
            ByteUnit::Tebibyte => "TiB",
            ByteUnit::Pebibyte => "PiB",
            ByteUnit::Exbibyte => "EiB",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteUnitConverter {
    total_bytes: u64,
}

impl ByteUnitConverter {
    pub fn new(bytes: u64) -> Self {
        Self { total_bytes: bytes }
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn from_bytes(value: u64) -> Self {
        Self::new(value)
    }
    pub fn from_kilobytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Kilobyte.factor())
    }
    pub fn from_megabytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Megabyte.factor())
    }
    pub fn from_gigabytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Gigabyte.factor())
    }
    pub fn from_terabytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Terabyte.factor())
    }
    pub fn from_petabytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Petabyte.factor())
    }
    pub fn from_exabytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Exabyte.factor())
    }

    pub fn from_kibibytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Kibibyte.factor())
    }
    pub fn from_mebibytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Mebibyte.factor())
    }
    pub fn from_gibibytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Gibibyte.factor())
    }
    pub fn from_tebibytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Tebibyte.factor())
    }
    pub fn from_pebibytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Pebibyte.factor())
    }
    pub fn from_exbibytes(value: u64) -> Self {
        Self::new(value * ByteUnit::Exbibyte.factor())
    }

    pub fn best_unit(&self, use_binary: bool) -> ByteUnit {
        let ordered: &[ByteUnit] = if use_binary {
            &[
                ByteUnit::Exbibyte,
                ByteUnit::Pebibyte,
                ByteUnit::Tebibyte,
                ByteUnit::Gibibyte,
                ByteUnit::Mebibyte,
                ByteUnit::Kibibyte,
                ByteUnit::Byte,
            ]
        } else {
            &[
                ByteUnit::Exabyte,
                ByteUnit::Petabyte,
                ByteUnit::Terabyte,
                ByteUnit::Gigabyte,
                ByteUnit::Megabyte,
                ByteUnit::Kilobyte,
                ByteUnit::Byte,
            ]
        };

        *ordered
            .iter()
            .find(|u| self.total_bytes >= u.factor())
            .unwrap_or(&ByteUnit::Byte)
    }

    pub fn format(&self, unit: ByteUnit, decimals: usize) -> String {
        let value = self.total_bytes as f64 / unit.factor() as f64;
        format!("{:.prec$} {}", value, unit.symbol(), prec = decimals)
    }

    pub fn to_string_auto(&self, use_binary: bool) -> String {
        self.format(self.best_unit(use_binary), 2)
    }
}

impl fmt::Display for ByteUnitConverter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_auto(false))
    }
}

impl std::ops::Add for ByteUnitConverter {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.total_bytes + rhs.total_bytes)
    }
}

impl std::ops::Sub for ByteUnitConverter {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.total_bytes.saturating_sub(rhs.total_bytes))
    }
}

impl std::ops::Mul<u64> for ByteUnitConverter {
    type Output = Self;
    fn mul(self, rhs: u64) -> Self {
        Self::new(self.total_bytes * rhs)
    }
}

impl std::ops::Div<u64> for ByteUnitConverter {
    type Output = Self;
    fn div(self, rhs: u64) -> Self {
        Self::new(self.total_bytes / rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_gigabytes() {
        let b = ByteUnitConverter::from_gigabytes(2);
        assert_eq!(b.total_bytes(), 2_000_000_000);
    }

    #[test]
    fn test_best_unit_decimal() {
        let b = ByteUnitConverter::from_megabytes(512);
        assert_eq!(b.best_unit(false), ByteUnit::Megabyte);
    }

    #[test]
    fn test_best_unit_binary() {
        let b = ByteUnitConverter::from_gibibytes(1);
        assert_eq!(b.best_unit(true), ByteUnit::Gibibyte);
    }

    #[test]
    fn test_display() {
        let b = ByteUnitConverter::from_megabytes(1500);
        assert_eq!(b.to_string(), "1.50 GB");
    }

    #[test]
    fn test_sub_saturating() {
        let a = ByteUnitConverter::from_bytes(100);
        let b = ByteUnitConverter::from_bytes(200);
        assert_eq!((a - b).total_bytes(), 0);
    }
}
