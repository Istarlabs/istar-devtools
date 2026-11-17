// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! ULID, NanoID and Snowflake IDs.

use chrono::DateTime;
use rand::{Rng, RngCore};

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Largest ULID timestamp: 48 bits of milliseconds.
const ULID_MAX_TIME: u64 = (1 << 48) - 1;

pub fn encode_ulid(timestamp_ms: u64, randomness: u128) -> String {
    let value = (u128::from(timestamp_ms) << 80) | (randomness & ((1u128 << 80) - 1));
    (0..26)
        .rev()
        .map(|index| CROCKFORD[((value >> (index * 5)) & 31) as usize] as char)
        .collect()
}

/// `n` ULIDs for the given millisecond. IDs created in the same millisecond count up from a random start,
/// so a batch sorts in the order it was generated.
pub fn generate_ulids(count: usize, timestamp_ms: u64, rng: &mut impl RngCore) -> Vec<String> {
    let mut randomness = rng.r#gen::<u128>() & ((1u128 << 80) - 1);
    (0..count)
        .map(|_| {
            let ulid = encode_ulid(timestamp_ms, randomness);
            // Keep clear of the top of the range so a batch never overflows into the timestamp.
            randomness = (randomness + 1) & ((1u128 << 80) - 1);
            ulid
        })
        .collect()
}

pub struct DecodedUlid {
    pub timestamp_ms: u64,
    pub randomness: u128,
}

pub fn decode_ulid(text: &str) -> Result<DecodedUlid, String> {
    let text = text.trim();
    if text.chars().count() != 26 {
        return Err("A ULID has exactly 26 characters".to_string());
    }
    let mut value = 0u128;
    for (index, c) in text.chars().enumerate() {
        let digit = CROCKFORD
            .iter()
            .position(|&b| b as char == c.to_ascii_uppercase())
            .ok_or_else(|| format!("\"{c}\" is not a Crockford base32 character"))?;
        if index == 0 && digit > 7 {
            return Err("The first character must be 0-7 (the value exceeds 128 bits)".to_string());
        }
        value = (value << 5) | digit as u128;
    }
    Ok(DecodedUlid {
        timestamp_ms: (value >> 80) as u64,
        randomness: value & ((1u128 << 80) - 1),
    })
}

pub fn ulid_rows(text: &str) -> Result<Vec<(String, String)>, String> {
    let decoded = decode_ulid(text)?;
    debug_assert!(decoded.timestamp_ms <= ULID_MAX_TIME);
    let time = DateTime::from_timestamp_millis(decoded.timestamp_ms as i64)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string())
        .unwrap_or_else(|| "out of range".to_string());
    let value = (u128::from(decoded.timestamp_ms) << 80) | decoded.randomness;
    let uuid = uuid::Uuid::from_u128(value);
    Ok(vec![
        ("Time".to_string(), time),
        ("Unix (ms)".to_string(), decoded.timestamp_ms.to_string()),
        (
            "Randomness".to_string(),
            format!("{:020x}", decoded.randomness),
        ),
        ("As UUID".to_string(), uuid.to_string()),
    ])
}

pub const NANOID_URL_SAFE: &str =
    "_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const NANOID_ALPHANUMERIC: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const NANOID_LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
pub const NANOID_DIGITS: &str = "0123456789";
pub const NANOID_HEX: &str = "0123456789abcdef";

fn alphabet_chars(alphabet: &str) -> Result<Vec<char>, String> {
    let chars: Vec<char> = alphabet.chars().collect();
    if chars.len() < 2 || chars.len() > 255 {
        return Err("The alphabet needs between 2 and 255 characters".to_string());
    }
    let mut unique = chars.clone();
    unique.sort_unstable();
    unique.dedup();
    if unique.len() != chars.len() {
        return Err("The alphabet must not repeat characters".to_string());
    }
    Ok(chars)
}

/// NanoID: draw random bytes, mask them to the next power of two and discard values outside the alphabet,
/// which keeps every character equally likely.
pub fn generate_nanoid(
    alphabet: &str,
    length: usize,
    rng: &mut impl RngCore,
) -> Result<String, String> {
    let chars = alphabet_chars(alphabet)?;
    if !(1..=1024).contains(&length) {
        return Err("The length must be between 1 and 1024".to_string());
    }
    let mask = (chars.len().next_power_of_two() - 1) as u8;
    let mut id = String::with_capacity(length);
    while id.chars().count() < length {
        let byte = rng.r#gen::<u8>() & mask;
        if let Some(c) = chars.get(byte as usize) {
            id.push(*c);
        }
    }
    Ok(id)
}

pub fn nanoid_entropy_bits(alphabet: &str, length: usize) -> f64 {
    length as f64 * (alphabet.chars().count() as f64).log2()
}

pub const SNOWFLAKE_TWITTER_EPOCH: i64 = 1_288_834_974_657;
pub const SNOWFLAKE_DISCORD_EPOCH: i64 = 1_420_070_400_000;

pub fn snowflake_epoch(index: i32, custom: &str) -> Result<i64, String> {
    match index {
        0 => Ok(SNOWFLAKE_TWITTER_EPOCH),
        1 => Ok(SNOWFLAKE_DISCORD_EPOCH),
        2 => Ok(0),
        _ => custom
            .trim()
            .parse::<i64>()
            .map_err(|_| "The custom epoch must be a Unix timestamp in milliseconds".to_string()),
    }
}

pub fn snowflake_rows(text: &str, epoch_ms: i64) -> Result<Vec<(String, String)>, String> {
    let id: u64 = text
        .trim()
        .parse()
        .map_err(|_| "A Snowflake ID is an unsigned 64-bit number".to_string())?;
    let timestamp = (id >> 22) as i64 + epoch_ms;
    let time = DateTime::from_timestamp_millis(timestamp)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string())
        .unwrap_or_else(|| "out of range".to_string());
    Ok(vec![
        ("Time".to_string(), time),
        ("Unix (ms)".to_string(), timestamp.to_string()),
        (
            "Worker (5 bit)".to_string(),
            ((id >> 17) & 0x1f).to_string(),
        ),
        (
            "Process (5 bit)".to_string(),
            ((id >> 12) & 0x1f).to_string(),
        ),
        (
            "Machine (10 bit)".to_string(),
            ((id >> 12) & 0x3ff).to_string(),
        ),
        ("Sequence".to_string(), (id & 0xfff).to_string()),
    ])
}

/// The first Snowflake of a millisecond, e.g. as a range bound when querying by time.
pub fn snowflake_for_time(timestamp_ms: i64, epoch_ms: i64) -> Result<u64, String> {
    let offset = timestamp_ms - epoch_ms;
    if !(0..(1i64 << 41)).contains(&offset) {
        return Err("The time is outside the 41-bit range of this epoch".to_string());
    }
    Ok((offset as u64) << 22)
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    #[test]
    fn ulid_decodes_the_spec_example() {
        let decoded = decode_ulid("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
        assert_eq!(decoded.timestamp_ms, 1_469_922_850_259);
        assert_eq!(
            encode_ulid(decoded.timestamp_ms, decoded.randomness),
            "01ARZ3NDEKTSV4RRFFQ69G5FAV"
        );
        // Lower case is accepted.
        assert_eq!(
            decode_ulid("01arz3ndektsv4rrffq69g5fav")
                .unwrap()
                .timestamp_ms,
            1_469_922_850_259
        );

        let rows = ulid_rows("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
        assert_eq!(rows[0].1, "2016-07-30 23:54:10.259 UTC");
        assert_eq!(rows[3].1.len(), 36);
    }

    #[test]
    fn ulid_rejects_bad_input() {
        assert!(decode_ulid("short").is_err());
        assert!(decode_ulid("01ARZ3NDEKTSV4RRFFQ69G5FAU").is_err()); // U is not in the alphabet
        assert!(decode_ulid("81ARZ3NDEKTSV4RRFFQ69G5FAV").is_err()); // overflow
    }

    #[test]
    fn ulid_batches_sort_in_generation_order() {
        let mut rng = StdRng::seed_from_u64(3);
        let batch = generate_ulids(50, 1_700_000_000_000, &mut rng);
        let mut sorted = batch.clone();
        sorted.sort();
        assert_eq!(batch, sorted);
        assert!(batch.iter().all(|u| u.len() == 26));
        assert_eq!(
            decode_ulid(&batch[7]).unwrap().timestamp_ms,
            1_700_000_000_000
        );
    }

    #[test]
    fn nanoid_uses_only_the_alphabet() {
        let mut rng = StdRng::seed_from_u64(5);
        let id = generate_nanoid(NANOID_URL_SAFE, 21, &mut rng).unwrap();
        assert_eq!(id.chars().count(), 21);
        assert!(id.chars().all(|c| NANOID_URL_SAFE.contains(c)));
        let digits = generate_nanoid(NANOID_DIGITS, 200, &mut rng).unwrap();
        assert!(digits.chars().all(|c| c.is_ascii_digit()));
        // Every digit shows up, i.e. the rejection sampling does not skew towards a subset.
        assert!(NANOID_DIGITS.chars().all(|d| digits.contains(d)));
        assert!((nanoid_entropy_bits(NANOID_URL_SAFE, 21) - 126.0).abs() < 0.01);
        assert!(generate_nanoid("aab", 5, &mut rng).is_err());
        assert!(generate_nanoid("a", 5, &mut rng).is_err());
        assert!(generate_nanoid(NANOID_HEX, 0, &mut rng).is_err());
    }

    #[test]
    fn snowflake_decodes_discord_and_twitter_ids() {
        let rows = snowflake_rows("175928847299117063", SNOWFLAKE_DISCORD_EPOCH).unwrap();
        assert_eq!(rows[0].1, "2016-04-30 11:18:25.796 UTC");
        assert_eq!(rows[2].1, "1");
        assert_eq!(rows[3].1, "0");
        assert_eq!(rows[5].1, "7");

        let twitter = snowflake_rows("1541815603606036480", SNOWFLAKE_TWITTER_EPOCH).unwrap();
        assert_eq!(twitter[0].1, "2022-06-28 16:07:40.105 UTC");
        assert!(snowflake_rows("-1", 0).is_err());
    }

    #[test]
    fn snowflake_round_trips_through_time() {
        let id = snowflake_for_time(1_462_015_105_796, SNOWFLAKE_DISCORD_EPOCH).unwrap();
        assert_eq!(id >> 22, 1_462_015_105_796 - SNOWFLAKE_DISCORD_EPOCH as u64);
        assert!(
            snowflake_rows(&id.to_string(), SNOWFLAKE_DISCORD_EPOCH).unwrap()[0]
                .1
                .starts_with("2016-04-30 11:18:25.796")
        );
        assert!(snowflake_for_time(0, SNOWFLAKE_DISCORD_EPOCH).is_err());
        assert_eq!(snowflake_epoch(1, "").unwrap(), SNOWFLAKE_DISCORD_EPOCH);
        assert_eq!(snowflake_epoch(3, " 1000 ").unwrap(), 1000);
        assert!(snowflake_epoch(3, "x").is_err());
    }
}
