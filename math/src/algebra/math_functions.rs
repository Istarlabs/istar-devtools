// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

pub fn factorial(n: u64) -> Result<u64, String> {
    let mut result = 1u64;
    for i in 2..=n {
        result = result.checked_mul(i).ok_or(format!("Overflow at {i}"))?;
    }
    Ok(result)
}