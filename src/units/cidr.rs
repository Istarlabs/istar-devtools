// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! IPv4 and IPv6 network arithmetic.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// A network as an address-family-independent integer range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Network {
    pub v6: bool,
    /// The address as typed, before masking.
    pub address: u128,
    pub prefix: u32,
}

impl Network {
    fn bits(self) -> u32 {
        if self.v6 { 128 } else { 32 }
    }

    fn host_mask(self) -> u128 {
        let host_bits = self.bits() - self.prefix;
        if host_bits >= 128 {
            u128::MAX
        } else {
            (1u128 << host_bits) - 1
        }
    }

    fn network_address(self) -> u128 {
        self.address & !self.host_mask() & self.all_ones()
    }

    fn last_address(self) -> u128 {
        self.network_address() | self.host_mask()
    }

    fn all_ones(self) -> u128 {
        if self.v6 { u128::MAX } else { u32::MAX as u128 }
    }

    pub fn format(self, value: u128) -> String {
        if self.v6 {
            Ipv6Addr::from(value).to_string()
        } else {
            Ipv4Addr::from(value as u32).to_string()
        }
    }

    pub fn contains(self, other: IpAddr) -> bool {
        let (v6, value) = to_integer(other);
        v6 == self.v6 && value >= self.network_address() && value <= self.last_address()
    }
}

fn to_integer(address: IpAddr) -> (bool, u128) {
    match address {
        IpAddr::V4(v4) => (false, u32::from(v4) as u128),
        IpAddr::V6(v6) => (true, u128::from(v6)),
    }
}

fn netmask_to_prefix(mask: Ipv4Addr) -> Option<u32> {
    let value = u32::from(mask);
    let prefix = value.leading_ones();
    // A valid mask is a run of ones followed by zeros.
    (value.checked_shl(prefix).unwrap_or(0) == 0).then_some(prefix)
}

/// Parse `address/prefix`, `address prefix` or `address netmask`. A bare address is a host (/32 or /128).
pub fn parse_network(text: &str) -> Result<Network, String> {
    let text = text.trim();
    let (address_text, prefix_text) = match text.split_once(['/', ' ']) {
        Some((address, prefix)) => (address.trim(), Some(prefix.trim())),
        None => (text, None),
    };
    let address: IpAddr = address_text
        .parse()
        .map_err(|_| format!("\"{address_text}\" is not an IP address"))?;
    let (v6, value) = to_integer(address);
    let max = if v6 { 128 } else { 32 };

    let prefix = match prefix_text {
        None | Some("") => max,
        Some(prefix_text) => match prefix_text.parse::<u32>() {
            Ok(prefix) => prefix,
            Err(_) => {
                let mask: Ipv4Addr = prefix_text
                    .parse()
                    .map_err(|_| format!("\"{prefix_text}\" is not a prefix length or netmask"))?;
                if v6 {
                    return Err("A dotted netmask only works with IPv4".to_string());
                }
                netmask_to_prefix(mask).ok_or("The netmask is not contiguous")?
            }
        },
    };
    if prefix > max {
        return Err(format!("The prefix length must be between 0 and {max}"));
    }
    Ok(Network {
        v6,
        address: value,
        prefix,
    })
}

pub fn parse_address(text: &str) -> Result<IpAddr, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("\"{}\" is not an IP address", text.trim()))
}

fn count_text(host_bits: u32) -> String {
    if host_bits < 64 {
        (1u128 << host_bits).to_string()
    } else {
        format!("2^{host_bits}")
    }
}

fn scope(address: IpAddr) -> &'static str {
    match address {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            if v4.is_unspecified() {
                "Unspecified"
            } else if v4.is_loopback() {
                "Loopback"
            } else if v4.is_private() {
                "Private (RFC 1918)"
            } else if octets[0] == 100 && (64..128).contains(&octets[1]) {
                "Shared address space (CGNAT, RFC 6598)"
            } else if v4.is_link_local() {
                "Link-local"
            } else if v4.is_multicast() {
                "Multicast"
            } else if v4.is_broadcast() {
                "Broadcast"
            } else if matches!(
                (octets[0], octets[1], octets[2]),
                (192, 0, 2) | (198, 51, 100) | (203, 0, 113)
            ) {
                "Documentation (TEST-NET)"
            } else if octets[0] >= 240 {
                "Reserved"
            } else {
                "Public"
            }
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            if v6.is_unspecified() {
                "Unspecified"
            } else if v6.is_loopback() {
                "Loopback"
            } else if v6.is_multicast() {
                "Multicast"
            } else if first & 0xfe00 == 0xfc00 {
                "Unique local (fc00::/7)"
            } else if first & 0xffc0 == 0xfe80 {
                "Link-local (fe80::/10)"
            } else if first == 0x2001 && v6.segments()[1] == 0x0db8 {
                "Documentation (2001:db8::/32)"
            } else {
                "Global unicast"
            }
        }
    }
}

/// The detail rows for a network.
pub fn describe(network: Network) -> Vec<(String, String)> {
    let host_bits = network.bits() - network.prefix;
    let first = network.network_address();
    let last = network.last_address();
    let mut rows = vec![
        (
            "CIDR".to_string(),
            format!("{}/{}", network.format(first), network.prefix),
        ),
        ("Network".to_string(), network.format(first)),
    ];

    if !network.v6 {
        let mask = !network.host_mask() & network.all_ones();
        rows.push(("Netmask".to_string(), network.format(mask)));
        rows.push(("Wildcard".to_string(), network.format(network.host_mask())));
        if network.prefix <= 30 {
            rows.push(("Broadcast".to_string(), network.format(last)));
        }
    }

    // /31 (RFC 3021) and /32 networks have no network/broadcast addresses to subtract.
    let (first_host, last_host, hosts) = if host_bits >= 2 {
        let hosts = if host_bits >= 64 {
            format!("2^{host_bits} - 2")
        } else {
            ((1u128 << host_bits) - 2).to_string()
        };
        (first + 1, if network.v6 { last } else { last - 1 }, hosts)
    } else {
        (first, last, count_text(host_bits))
    };
    rows.push(("First host".to_string(), network.format(first_host)));
    rows.push(("Last host".to_string(), network.format(last_host)));
    rows.push(("Addresses".to_string(), count_text(host_bits)));
    rows.push(("Usable hosts".to_string(), hosts));
    rows.push((
        "Scope".to_string(),
        scope(if network.v6 {
            IpAddr::V6(Ipv6Addr::from(first))
        } else {
            IpAddr::V4(Ipv4Addr::from(first as u32))
        })
        .to_string(),
    ));
    rows
}

/// Split into subnets of `new_prefix`, at most `limit` of them. The second value is how many exist in total.
pub fn split(
    network: Network,
    new_prefix: u32,
    limit: usize,
) -> Result<(Vec<String>, String), String> {
    if new_prefix <= network.prefix || new_prefix > network.bits() {
        return Err(format!(
            "The new prefix must be between {} and {}",
            network.prefix + 1,
            network.bits()
        ));
    }
    let step_bits = network.bits() - new_prefix;
    let count_bits = new_prefix - network.prefix;
    let total = count_text(count_bits);

    let shown = if count_bits >= 64 {
        limit
    } else {
        limit.min(1usize << count_bits.min(32))
    };
    let start = network.network_address();
    let subnets = (0..shown as u128)
        .map(|index| {
            format!(
                "{}/{}",
                network.format(start + (index << step_bits)),
                new_prefix
            )
        })
        .collect();
    Ok((subnets, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str) -> Vec<(String, String)> {
        describe(parse_network(text).unwrap())
    }

    fn get<'a>(rows: &'a [(String, String)], label: &str) -> &'a str {
        rows.iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| v.as_str())
            .unwrap()
    }

    #[test]
    fn ipv4_network_details() {
        let rows = rows("192.168.1.130/26");
        assert_eq!(get(&rows, "CIDR"), "192.168.1.128/26");
        assert_eq!(get(&rows, "Netmask"), "255.255.255.192");
        assert_eq!(get(&rows, "Wildcard"), "0.0.0.63");
        assert_eq!(get(&rows, "Broadcast"), "192.168.1.191");
        assert_eq!(get(&rows, "First host"), "192.168.1.129");
        assert_eq!(get(&rows, "Last host"), "192.168.1.190");
        assert_eq!(get(&rows, "Addresses"), "64");
        assert_eq!(get(&rows, "Usable hosts"), "62");
        assert_eq!(get(&rows, "Scope"), "Private (RFC 1918)");
    }

    #[test]
    fn netmask_notation_and_edge_prefixes() {
        assert_eq!(parse_network("10.0.0.1 255.255.0.0").unwrap().prefix, 16);
        assert!(parse_network("10.0.0.1 255.0.255.0").is_err());
        assert!(parse_network("10.0.0.1/33").is_err());

        let host = rows("8.8.8.8");
        assert_eq!(get(&host, "Usable hosts"), "1");
        assert_eq!(get(&host, "Scope"), "Public");
        let point_to_point = rows("10.0.0.0/31");
        assert_eq!(get(&point_to_point, "First host"), "10.0.0.0");
        assert_eq!(get(&point_to_point, "Last host"), "10.0.0.1");
        assert_eq!(get(&point_to_point, "Usable hosts"), "2");
        let everything = rows("0.0.0.0/0");
        assert_eq!(get(&everything, "Addresses"), "4294967296");
        assert_eq!(get(&everything, "Last host"), "255.255.255.254");
    }

    #[test]
    fn ipv6_network_details() {
        let rows = rows("2001:db8::1234/32");
        assert_eq!(get(&rows, "CIDR"), "2001:db8::/32");
        assert_eq!(get(&rows, "First host"), "2001:db8::1");
        assert_eq!(
            get(&rows, "Last host"),
            "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff"
        );
        assert_eq!(get(&rows, "Addresses"), "2^96");
        assert_eq!(get(&rows, "Scope"), "Documentation (2001:db8::/32)");
        assert!(parse_network("::1 255.255.255.0").is_err());
        assert_eq!(get(&self::rows("::/0"), "Addresses"), "2^128");
    }

    #[test]
    fn containment_respects_the_address_family() {
        let net = parse_network("10.1.0.0/16").unwrap();
        assert!(net.contains(parse_address("10.1.255.255").unwrap()));
        assert!(!net.contains(parse_address("10.2.0.0").unwrap()));
        assert!(!net.contains(parse_address("::ffff:10.1.0.1").unwrap()));
        assert!(parse_address("10.1.0").is_err());
    }

    #[test]
    fn splitting_into_subnets() {
        let net = parse_network("192.168.0.0/24").unwrap();
        let (subnets, total) = split(net, 26, 64).unwrap();
        assert_eq!(
            subnets,
            [
                "192.168.0.0/26",
                "192.168.0.64/26",
                "192.168.0.128/26",
                "192.168.0.192/26"
            ]
        );
        assert_eq!(total, "4");
        let (limited, total) = split(parse_network("10.0.0.0/8").unwrap(), 24, 3).unwrap();
        assert_eq!(limited.len(), 3);
        assert_eq!(total, "65536");
        assert!(split(net, 24, 4).is_err());
        assert!(split(net, 33, 4).is_err());
    }
}
