// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use serde_json::Value;
use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, JwtState};

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<JwtState>().on_decode(move || {
        let ui = handle.unwrap();
        let state = ui.global::<JwtState>();
        let token = state.get_token_input();

        match decode_token(token.as_str().trim()) {
            Ok((header, payload, signature)) => {
                state.set_header_json(header.to_shared_string());
                state.set_payload_json(payload.to_shared_string());
                state.set_signature_raw(signature.to_shared_string());
                state.set_decode_error("".to_shared_string());
            }
            Err(e) => {
                state.set_header_json("".to_shared_string());
                state.set_payload_json("".to_shared_string());
                state.set_signature_raw("".to_shared_string());
                state.set_decode_error(e.to_shared_string());
            }
        }
    });

    let handle = app.as_weak();
    app.global::<JwtState>().on_verify(move || {
        let ui = handle.unwrap();
        let state = ui.global::<JwtState>();

        let token = state.get_token_input();
        let alg = map_algorithm(state.get_verify_algorithm_index());
        let secret = state.get_verify_secret();

        match build_decoding_key(alg, secret.as_str()) {
            Ok(key) => {
                let mut validation = Validation::new(alg);
                validation.validate_exp = false;
                validation.validate_nbf = false;
                validation.validate_aud = false;
                validation.required_spec_claims.clear();

                match decode::<Value>(token.as_str().trim(), &key, &validation) {
                    Ok(_) => {
                        state.set_verify_status(1);
                        state.set_verify_message("".to_shared_string());
                    }
                    Err(e) => {
                        state.set_verify_status(2);
                        state.set_verify_message(e.to_shared_string());
                    }
                }
            }
            Err(e) => {
                state.set_verify_status(3);
                state.set_verify_message(e.to_shared_string());
            }
        }
    });

    let handle = app.as_weak();
    app.global::<JwtState>().on_encode(move || {
        let ui = handle.unwrap();
        let state = ui.global::<JwtState>();

        let alg = map_algorithm(state.get_encode_algorithm_index());
        let header_json = state.get_encode_header();
        let payload_json = state.get_encode_payload();
        let secret = state.get_encode_secret();

        match encode_token(alg, header_json.as_str(), payload_json.as_str(), secret.as_str()) {
            Ok(token) => {
                state.set_encode_result(token.to_shared_string());
                state.set_encode_error("".to_shared_string());
            }
            Err(e) => {
                state.set_encode_result("".to_shared_string());
                state.set_encode_error(e.to_shared_string());
            }
        }
    });

    let handle = app.as_weak();
    app.global::<JwtState>().on_copy_result(move || {
        let ui = handle.unwrap();
        let state = ui.global::<JwtState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(state.get_encode_result().as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<JwtState>().on_paste_token(move || {
        let ui = handle.unwrap();
        let state = ui.global::<JwtState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                state.set_token_input(text.to_shared_string());
            }
        }
    });
}

fn map_algorithm(index: i32) -> Algorithm {
    match index {
        1 => Algorithm::HS384,
        2 => Algorithm::HS512,
        3 => Algorithm::RS256,
        4 => Algorithm::RS384,
        5 => Algorithm::RS512,
        6 => Algorithm::ES256,
        7 => Algorithm::ES384,
        _ => Algorithm::HS256,
    }
}

/// Structural decode of a JWT without signature verification.
/// Returns pretty-printed header, pretty-printed payload, hex signature.
fn decode_token(token: &str) -> Result<(String, String, String), String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Token must have three dot-separated segments".to_string());
    }

    let header = decode_segment_json(parts[0]).map_err(|e| format!("Header: {e}"))?;
    let payload = decode_segment_json(parts[1]).map_err(|e| format!("Payload: {e}"))?;

    let signature_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| format!("Signature: {e}"))?;
    let signature = hex::encode(signature_bytes);

    Ok((header, payload, signature))
}

fn decode_segment_json(segment: &str) -> Result<String, String> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

fn build_decoding_key(alg: Algorithm, secret: &str) -> Result<DecodingKey, String> {
    match alg {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            Ok(DecodingKey::from_secret(secret.as_bytes()))
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 | Algorithm::PS256
        | Algorithm::PS384 | Algorithm::PS512 => {
            DecodingKey::from_rsa_pem(secret.as_bytes()).map_err(|e| e.to_string())
        }
        Algorithm::ES256 | Algorithm::ES384 => {
            DecodingKey::from_ec_pem(secret.as_bytes()).map_err(|e| e.to_string())
        }
        Algorithm::EdDSA => DecodingKey::from_ed_pem(secret.as_bytes()).map_err(|e| e.to_string()),
    }
}

fn build_encoding_key(alg: Algorithm, secret: &str) -> Result<EncodingKey, String> {
    match alg {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            Ok(EncodingKey::from_secret(secret.as_bytes()))
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 | Algorithm::PS256
        | Algorithm::PS384 | Algorithm::PS512 => {
            EncodingKey::from_rsa_pem(secret.as_bytes()).map_err(|e| e.to_string())
        }
        Algorithm::ES256 | Algorithm::ES384 => {
            EncodingKey::from_ec_pem(secret.as_bytes()).map_err(|e| e.to_string())
        }
        Algorithm::EdDSA => EncodingKey::from_ed_pem(secret.as_bytes()).map_err(|e| e.to_string()),
    }
}

fn encode_token(
    alg: Algorithm,
    header_json: &str,
    payload_json: &str,
    secret: &str,
) -> Result<String, String> {
    // Start from the user-supplied header so fields like `typ`/`kid` are kept,
    // then force the algorithm to the selected one.
    let mut header: Header =
        serde_json::from_str(header_json).unwrap_or_else(|_| Header::new(alg));
    header.alg = alg;

    let payload: Value =
        serde_json::from_str(payload_json).map_err(|e| format!("Payload JSON: {e}"))?;

    let key = build_encoding_key(alg, secret)?;
    encode(&header, &payload, &key).map_err(|e| e.to_string())
}
