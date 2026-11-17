// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! HMAC, password and passphrase generation.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::seq::SliceRandom;
use rand::{Rng, RngCore};
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HmacAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl HmacAlgorithm {
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => HmacAlgorithm::Md5,
            1 => HmacAlgorithm::Sha1,
            3 => HmacAlgorithm::Sha512,
            _ => HmacAlgorithm::Sha256,
        }
    }

    fn block_size(self) -> usize {
        match self {
            HmacAlgorithm::Sha512 => 128,
            _ => 64,
        }
    }

    fn hash(self, data: &[u8]) -> Vec<u8> {
        match self {
            HmacAlgorithm::Md5 => md5::compute(data).0.to_vec(),
            HmacAlgorithm::Sha1 => Sha1::digest(data).to_vec(),
            HmacAlgorithm::Sha256 => Sha256::digest(data).to_vec(),
            HmacAlgorithm::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
}

/// HMAC as defined in RFC 2104.
pub fn hmac(algorithm: HmacAlgorithm, key: &[u8], message: &[u8]) -> Vec<u8> {
    let block = algorithm.block_size();
    let mut key_block = if key.len() > block {
        algorithm.hash(key)
    } else {
        key.to_vec()
    };
    key_block.resize(block, 0);

    let mut inner: Vec<u8> = key_block.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(message);
    let inner_hash = algorithm.hash(&inner);

    let mut outer: Vec<u8> = key_block.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&inner_hash);
    algorithm.hash(&outer)
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(text: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    hex::decode(&cleaned).map_err(|e| format!("Key is not valid hex: {e}"))
}

/// Hex and Base64 of the HMAC.
pub fn hmac_rows(algorithm: HmacAlgorithm, key: &[u8], message: &[u8]) -> Vec<(String, String)> {
    let mac = hmac(algorithm, key, message);
    vec![
        ("Hex".to_string(), to_hex(&mac)),
        ("Base64".to_string(), STANDARD.encode(&mac)),
    ]
}

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.<>?/~";
const LOOK_ALIKES: &str = "Il1O0o|";

#[derive(Debug, Clone)]
pub struct PasswordOptions {
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_look_alikes: bool,
}

impl PasswordOptions {
    fn classes(&self) -> Vec<Vec<char>> {
        [
            (self.lower, LOWER),
            (self.upper, UPPER),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ]
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, set)| {
            set.chars()
                .filter(|c| !(self.exclude_look_alikes && LOOK_ALIKES.contains(*c)))
                .collect::<Vec<char>>()
        })
        .collect()
    }

    /// Entropy in bits of a password drawn uniformly from the whole pool.
    /// The "one of each class" rule makes the real value slightly lower.
    pub fn entropy_bits(&self) -> f64 {
        let pool: usize = self.classes().iter().map(Vec::len).sum();
        if pool == 0 {
            0.0
        } else {
            self.length as f64 * (pool as f64).log2()
        }
    }
}

pub fn generate_password(
    options: &PasswordOptions,
    rng: &mut impl RngCore,
) -> Result<String, String> {
    let classes = options.classes();
    if classes.is_empty() {
        return Err("Select at least one character class".to_string());
    }
    if options.length < classes.len() {
        return Err(format!(
            "The length must be at least {} to include every selected class",
            classes.len()
        ));
    }
    if options.length > 1024 {
        return Err("The length must be at most 1024".to_string());
    }

    let pool: Vec<char> = classes.iter().flatten().copied().collect();
    // One character from each class first, then the rest from the pool, then shuffle.
    let mut password: Vec<char> = classes
        .iter()
        .map(|class| class[rng.gen_range(0..class.len())])
        .collect();
    while password.len() < options.length {
        password.push(pool[rng.gen_range(0..pool.len())]);
    }
    password.shuffle(rng);
    Ok(password.into_iter().collect())
}

pub fn wordlist() -> Vec<&'static str> {
    include_str!("wordlist.txt")
        .lines()
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .collect()
}

#[derive(Debug, Clone)]
pub struct PassphraseOptions {
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub add_digit: bool,
}

pub fn passphrase_entropy_bits(options: &PassphraseOptions) -> f64 {
    let mut bits = options.words as f64 * (wordlist().len() as f64).log2();
    if options.add_digit {
        bits += 10f64.log2();
    }
    bits
}

pub fn generate_passphrase(
    options: &PassphraseOptions,
    rng: &mut impl RngCore,
) -> Result<String, String> {
    if !(1..=32).contains(&options.words) {
        return Err("The number of words must be between 1 and 32".to_string());
    }
    let list = wordlist();
    let mut parts: Vec<String> = (0..options.words)
        .map(|_| {
            let word = list[rng.gen_range(0..list.len())];
            if options.capitalize {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            } else {
                word.to_string()
            }
        })
        .collect();
    if options.add_digit {
        let index = rng.gen_range(0..parts.len());
        parts[index].push_str(&rng.gen_range(0..10).to_string());
    }
    Ok(parts.join(&options.separator))
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    fn mac(algorithm: HmacAlgorithm, key: &[u8], message: &[u8]) -> String {
        to_hex(&hmac(algorithm, key, message))
    }

    #[test]
    fn hmac_matches_rfc_test_vectors() {
        // RFC 2202, test case 2 (MD5, SHA-1) and RFC 4231, test case 2 (SHA-256, SHA-512).
        let (key, message) = (
            b"Jefe".as_slice(),
            b"what do ya want for nothing?".as_slice(),
        );
        assert_eq!(
            mac(HmacAlgorithm::Md5, key, message),
            "750c783e6ab0b503eaa86e310a5db738"
        );
        assert_eq!(
            mac(HmacAlgorithm::Sha1, key, message),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
        assert_eq!(
            mac(HmacAlgorithm::Sha256, key, message),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            mac(HmacAlgorithm::Sha512, key, message),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea250554\
9758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
    }

    #[test]
    fn hmac_handles_keys_longer_than_the_block() {
        // RFC 4231, test case 6: 131-byte key of 0xaa.
        let key = [0xaa; 131];
        let message = b"Test Using Larger Than Block-Size Key - Hash Key First";
        assert_eq!(
            mac(HmacAlgorithm::Sha256, &key, message),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
        assert_eq!(from_hex("de ad BE ef").unwrap(), [0xde, 0xad, 0xbe, 0xef]);
        assert!(from_hex("xyz").is_err());
    }

    #[test]
    fn passwords_follow_the_options() {
        let mut rng = StdRng::seed_from_u64(7);
        let options = PasswordOptions {
            length: 24,
            lower: true,
            upper: true,
            digits: true,
            symbols: true,
            exclude_look_alikes: true,
        };
        for _ in 0..50 {
            let password = generate_password(&options, &mut rng).unwrap();
            assert_eq!(password.chars().count(), 24);
            assert!(password.chars().any(|c| c.is_ascii_lowercase()));
            assert!(password.chars().any(|c| c.is_ascii_uppercase()));
            assert!(password.chars().any(|c| c.is_ascii_digit()));
            assert!(password.chars().any(|c| SYMBOLS.contains(c)));
            assert!(!password.chars().any(|c| LOOK_ALIKES.contains(c)));
        }
        assert!(options.entropy_bits() > 140.0);

        let digits_only = PasswordOptions {
            lower: false,
            upper: false,
            symbols: false,
            ..options.clone()
        };
        assert!(
            generate_password(&digits_only, &mut rng)
                .unwrap()
                .chars()
                .all(|c| c.is_ascii_digit())
        );
        let nothing = PasswordOptions {
            lower: false,
            upper: false,
            digits: false,
            symbols: false,
            ..options.clone()
        };
        assert!(generate_password(&nothing, &mut rng).is_err());
        assert!(
            generate_password(
                &PasswordOptions {
                    length: 3,
                    ..options.clone()
                },
                &mut rng
            )
            .is_err()
        );
    }

    #[test]
    fn passphrases_use_the_word_list() {
        let list = wordlist();
        assert!(list.len() >= 500);
        assert!(
            list.iter()
                .all(|w| w.chars().all(|c| c.is_ascii_lowercase()))
        );
        let mut sorted = list.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), list.len());

        let mut rng = StdRng::seed_from_u64(1);
        let options = PassphraseOptions {
            words: 5,
            separator: "-".into(),
            capitalize: true,
            add_digit: true,
        };
        let phrase = generate_passphrase(&options, &mut rng).unwrap();
        let parts: Vec<&str> = phrase.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert!(
            parts
                .iter()
                .all(|p| p.chars().next().unwrap().is_ascii_uppercase())
        );
        assert_eq!(phrase.chars().filter(char::is_ascii_digit).count(), 1);
        assert!(passphrase_entropy_bits(&options) > 45.0);
        assert!(
            generate_passphrase(
                &PassphraseOptions {
                    words: 0,
                    ..options
                },
                &mut rng
            )
            .is_err()
        );
    }
}
