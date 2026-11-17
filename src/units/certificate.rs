// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Decoding of PEM blocks and X.509 certificates.

use std::fmt::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::DateTime;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use x509_parser::certificate::X509Certificate;
use x509_parser::extensions::GeneralName;
use x509_parser::objects::{oid_registry, oid2sn};
use x509_parser::pem::Pem;
use x509_parser::prelude::FromDer;
use x509_parser::public_key::PublicKey;
use x509_parser::x509::AlgorithmIdentifier;

type Rows = Vec<(String, String)>;

fn row(rows: &mut Rows, label: &str, value: impl Into<String>) {
    rows.push((label.to_string(), value.into()));
}

fn hex_fingerprint(digest: &[u8]) -> String {
    let mut out = String::with_capacity(digest.len() * 3);
    for (index, byte) in digest.iter().enumerate() {
        if index > 0 {
            out.push(':');
        }
        let _ = write!(out, "{byte:02X}");
    }
    out
}

fn algorithm_name(algorithm: &AlgorithmIdentifier) -> String {
    oid2sn(&algorithm.algorithm, oid_registry())
        .map(str::to_string)
        .unwrap_or_else(|_| algorithm.algorithm.to_id_string())
}

fn format_time(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}

fn validity_text(not_after: i64, now: i64) -> String {
    let days = (not_after - now).div_euclid(86_400);
    if not_after < now {
        format!("expired {} day(s) ago", -days)
    } else {
        format!("expires in {days} day(s)")
    }
}

fn general_name(name: &GeneralName) -> String {
    match name {
        GeneralName::DNSName(dns) => format!("DNS:{dns}"),
        GeneralName::RFC822Name(mail) => format!("email:{mail}"),
        GeneralName::URI(uri) => format!("URI:{uri}"),
        GeneralName::IPAddress(bytes) => match bytes.len() {
            4 => format!(
                "IP:{}",
                IpAddr::V4(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]))
            ),
            16 => {
                let mut octets = [0u8; 16];
                octets.copy_from_slice(bytes);
                format!("IP:{}", IpAddr::V6(Ipv6Addr::from(octets)))
            }
            _ => format!("IP:{}", hex_fingerprint(bytes)),
        },
        GeneralName::DirectoryName(name) => format!("dirName:{name}"),
        other => format!("{other:?}"),
    }
}

fn describe_certificate(der: &[u8], now: i64, rows: &mut Rows) -> Result<(), String> {
    let (_, certificate) =
        X509Certificate::from_der(der).map_err(|e| format!("Not a valid certificate: {e}"))?;

    row(rows, "Subject", certificate.subject().to_string());
    row(rows, "Issuer", certificate.issuer().to_string());
    if certificate.subject() == certificate.issuer() {
        row(rows, "Self-signed", "yes (subject equals issuer)");
    }
    row(rows, "Serial", certificate.raw_serial_as_string());
    row(rows, "Version", format!("v{}", certificate.version().0 + 1));

    let validity = certificate.validity();
    let not_before = validity.not_before.timestamp();
    let not_after = validity.not_after.timestamp();
    row(rows, "Not before", format_time(not_before));
    row(rows, "Not after", format_time(not_after));
    let status = if now < not_before {
        "not valid yet".to_string()
    } else {
        validity_text(not_after, now)
    };
    row(rows, "Status", status);

    row(
        rows,
        "Signature algorithm",
        algorithm_name(&certificate.signature_algorithm),
    );
    let public_key = certificate.public_key();
    let key_size = match public_key.parsed() {
        Ok(PublicKey::RSA(rsa)) => format!("{} bit", rsa.key_size()),
        Ok(PublicKey::EC(point)) => format!("{} bit", point.key_size()),
        Ok(PublicKey::DSA(_)) => "DSA".to_string(),
        _ => "unknown size".to_string(),
    };
    row(
        rows,
        "Public key",
        format!("{} ({key_size})", algorithm_name(&public_key.algorithm)),
    );

    if let Ok(Some(san)) = certificate.subject_alternative_name() {
        let names: Vec<String> = san.value.general_names.iter().map(general_name).collect();
        row(rows, "Subject alt. names", names.join(", "));
    }
    if let Ok(Some(constraints)) = certificate.basic_constraints() {
        let mut text = if constraints.value.ca {
            "CA".to_string()
        } else {
            "not a CA".to_string()
        };
        if let Some(length) = constraints.value.path_len_constraint {
            let _ = write!(text, ", path length {length}");
        }
        row(rows, "Basic constraints", text);
    }
    if let Ok(Some(usage)) = certificate.key_usage() {
        row(rows, "Key usage", usage.value.to_string());
    }
    if let Ok(Some(extended)) = certificate.extended_key_usage() {
        let usage = extended.value;
        let mut names: Vec<String> = [
            (usage.any, "any"),
            (usage.server_auth, "serverAuth"),
            (usage.client_auth, "clientAuth"),
            (usage.code_signing, "codeSigning"),
            (usage.email_protection, "emailProtection"),
            (usage.time_stamping, "timeStamping"),
            (usage.ocsp_signing, "OCSPSigning"),
        ]
        .into_iter()
        .filter(|(set, _)| *set)
        .map(|(_, name)| name.to_string())
        .collect();
        names.extend(usage.other.iter().map(|oid| oid.to_id_string()));
        row(rows, "Extended key usage", names.join(", "));
    }

    row(rows, "SHA-256", hex_fingerprint(&Sha256::digest(der)));
    row(rows, "SHA-1", hex_fingerprint(&Sha1::digest(der)));
    Ok(())
}

/// Describe every PEM block in `text`. `now` is a Unix timestamp, passed in for testability.
pub fn decode(text: &str, now: i64) -> Result<Rows, String> {
    let blocks: Vec<Pem> = Pem::iter_from_buffer(text.as_bytes())
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Invalid PEM: {e}"))?;
    if blocks.is_empty() {
        return Err("No PEM block found (expected -----BEGIN ...-----)".to_string());
    }

    let mut rows = Rows::new();
    for (index, block) in blocks.iter().enumerate() {
        row(
            &mut rows,
            &format!("Block {}", index + 1),
            block.label.clone(),
        );
        match block.label.as_str() {
            "CERTIFICATE" | "TRUSTED CERTIFICATE" | "X509 CERTIFICATE" => {
                describe_certificate(&block.contents, now, &mut rows)?;
            }
            label => {
                row(
                    &mut rows,
                    "Size",
                    format!("{} bytes (DER)", block.contents.len()),
                );
                if label.contains("PRIVATE KEY") {
                    row(&mut rows, "Note", "Private key: contents are not shown");
                } else {
                    row(
                        &mut rows,
                        "SHA-256",
                        hex_fingerprint(&Sha256::digest(&block.contents)),
                    );
                }
            }
        }
    }
    Ok(rows)
}

/// Wrap raw DER bytes of a certificate into a PEM block, for files that are not text.
pub fn der_to_pem(der: &[u8]) -> String {
    let encoded = STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

#[cfg(test)]
mod tests {
    use super::*;

    // ECDSA P-256, valid 2026-10-22 .. 2126-09-28, generated for these tests.
    const CERT: &str = "-----BEGIN CERTIFICATE-----
MIICQDCCAeWgAwIBAgIUWIMz9ZeWI1kqP7EAXkaFSPWBXfMwCgYIKoZIzj0EAwIw
PjELMAkGA1UEBhMCREUxFDASBgNVBAoMC0V4YW1wbGUgT3JnMRkwFwYDVQQDDBB0
ZXN0LmV4YW1wbGUuY29tMCAXDTI2MTAyMjE1MzAyOVoYDzIxMjYwOTI4MTUzMDI5
WjA+MQswCQYDVQQGEwJERTEUMBIGA1UECgwLRXhhbXBsZSBPcmcxGTAXBgNVBAMM
EHRlc3QuZXhhbXBsZS5jb20wWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAARV4CtI
SysUq0DsSKNpEkifCJu0wrRZ0O/V1apNamuKyVrKzg8D57GINYiD0s7Vz9lZtB9f
R5J+iqyvd3F6/cteo4G+MIG7MB0GA1UdDgQWBBQH1kv7yY/e6plxRrB/t7TdqDPa
LjAfBgNVHSMEGDAWgBQH1kv7yY/e6plxRrB/t7TdqDPaLjA/BgNVHREEODA2ghB0
ZXN0LmV4YW1wbGUuY29tgg0qLmV4YW1wbGUuY29thwQKAAABgQ1hQGV4YW1wbGUu
Y29tMAsGA1UdDwQEAwIFoDAdBgNVHSUEFjAUBggrBgEFBQcDAQYIKwYBBQUHAwIw
DAYDVR0TAQH/BAIwADAKBggqhkjOPQQDAgNJADBGAiEAgzP327j9ViZBBeWASs+E
zEPplQP12NVBYDxR4WbU3u0CIQDtdE2YFanuG1A357DQclg5UentD1RhbRtCOsBO
E3ioog==
-----END CERTIFICATE-----
";
    // 2026-11-01 00:00:00 UTC
    const NOW: i64 = 1_793_491_200;

    fn get<'a>(rows: &'a Rows, label: &str) -> &'a str {
        rows.iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| v.as_str())
            .unwrap()
    }

    #[test]
    fn decodes_a_certificate() {
        let rows = decode(CERT, NOW).unwrap();
        assert_eq!(get(&rows, "Block 1"), "CERTIFICATE");
        assert!(get(&rows, "Subject").contains("CN=test.example.com"));
        assert_eq!(get(&rows, "Subject"), get(&rows, "Issuer"));
        assert_eq!(
            get(&rows, "Serial"),
            "58:83:33:f5:97:96:23:59:2a:3f:b1:00:5e:46:85:48:f5:81:5d:f3"
        );
        assert_eq!(get(&rows, "Not before"), "2026-10-22 15:30:29 UTC");
        assert_eq!(get(&rows, "Not after"), "2126-09-28 15:30:29 UTC");
        assert!(get(&rows, "Status").starts_with("expires in "));
        assert!(get(&rows, "Public key").contains("256 bit"));
        assert_eq!(
            get(&rows, "Subject alt. names"),
            "DNS:test.example.com, DNS:*.example.com, IP:10.0.0.1, email:a@example.com"
        );
        assert_eq!(get(&rows, "Basic constraints"), "not a CA");
        assert!(get(&rows, "Key usage").contains("Digital Signature"));
        assert_eq!(get(&rows, "Extended key usage"), "serverAuth, clientAuth");
        assert_eq!(
            get(&rows, "SHA-256"),
            "E9:51:F2:61:0A:27:84:24:FB:47:F3:8D:5A:D5:A9:E4:6E:C9:A8:13:B0:E5:D2:4C:0F:B2:75:BE:55:17:07:EC"
        );
    }

    #[test]
    fn validity_is_relative_to_now() {
        let before = decode(CERT, 1_700_000_000).unwrap();
        assert_eq!(get(&before, "Status"), "not valid yet");
        let after = decode(CERT, 5_000_000_000).unwrap();
        assert!(get(&after, "Status").starts_with("expired "));
        assert_eq!(
            validity_text(NOW + 86_400 * 3 + 5, NOW),
            "expires in 3 day(s)"
        );
    }

    #[test]
    fn other_blocks_and_errors() {
        let key = "-----BEGIN PRIVATE KEY-----\nAAECAwQF\n-----END PRIVATE KEY-----\n";
        let rows = decode(key, NOW).unwrap();
        assert_eq!(get(&rows, "Size"), "6 bytes (DER)");
        assert!(get(&rows, "Note").contains("not shown"));
        assert!(rows.iter().all(|(l, _)| l != "SHA-256"));

        assert!(decode("hello", NOW).is_err());
        let broken = "-----BEGIN CERTIFICATE-----\nAAECAwQF\n-----END CERTIFICATE-----\n";
        assert!(decode(broken, NOW).is_err());
    }

    #[test]
    fn der_files_are_wrapped_as_pem() {
        let der = STANDARD
            .decode(
                CERT.lines()
                    .filter(|l| !l.starts_with("-----"))
                    .collect::<String>(),
            )
            .unwrap();
        assert_eq!(
            decode(&der_to_pem(&der), NOW).unwrap(),
            decode(CERT, NOW).unwrap()
        );
    }
}
