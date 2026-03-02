use std::{fmt, net::IpAddr, num::NonZeroU32, sync::Arc};

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};

pub type AuthFailureRateLimiter =
    Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

pub type HealthCheckRateLimiter =
    Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

#[derive(Clone, Debug)]
enum TrustedEntry {
    Exact(IpAddr),
    Cidr(IpAddr, u8),
}

impl TrustedEntry {
    fn contains(&self, ip: &IpAddr) -> bool {
        match self {
            TrustedEntry::Exact(addr) => addr == ip,
            TrustedEntry::Cidr(network, prefix_len) => match (network, ip) {
                (IpAddr::V4(net), IpAddr::V4(addr)) => {
                    let mask = if *prefix_len == 0 {
                        0u32
                    } else {
                        u32::MAX << (32 - prefix_len)
                    };
                    u32::from(*net) & mask == u32::from(*addr) & mask
                }
                (IpAddr::V6(net), IpAddr::V6(addr)) => {
                    let mask = if *prefix_len == 0 {
                        0u128
                    } else {
                        u128::MAX << (128 - prefix_len)
                    };
                    u128::from(*net) & mask == u128::from(*addr) & mask
                }
                _ => false,
            },
        }
    }
}

impl fmt::Display for TrustedEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustedEntry::Exact(ip) => write!(f, "{ip}"),
            TrustedEntry::Cidr(ip, prefix) => write!(f, "{ip}/{prefix}"),
        }
    }
}

fn parse_trusted_entry(s: &str) -> Result<TrustedEntry, String> {
    if let Some((addr_str, prefix_str)) = s.split_once('/') {
        let addr: IpAddr = addr_str.parse().map_err(|e| format!("{e}"))?;
        let prefix_len: u8 = prefix_str.parse().map_err(|e| format!("{e}"))?;
        let max = if addr.is_ipv4() { 32 } else { 128 };
        if prefix_len > max {
            return Err(format!("prefix length {prefix_len} exceeds maximum {max}"));
        }
        Ok(TrustedEntry::Cidr(addr, prefix_len))
    } else {
        let addr: IpAddr = s.parse().map_err(|e| format!("{e}"))?;
        Ok(TrustedEntry::Exact(addr))
    }
}

#[derive(Clone, Debug)]
pub struct TrustedProxies(Arc<Vec<TrustedEntry>>);

impl TrustedProxies {
    pub fn new(ips: Vec<IpAddr>) -> Self {
        Self(Arc::new(ips.into_iter().map(TrustedEntry::Exact).collect()))
    }

    pub fn from_env() -> Self {
        let entries: Vec<TrustedEntry> = std::env::var("TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }
                match parse_trusted_entry(s) {
                    Ok(entry) => Some(entry),
                    Err(e) => {
                        tracing::warn!(value = %s, error = %e, "Ignoring invalid entry in TRUSTED_PROXIES");
                        None
                    }
                }
            })
            .collect();

        if !entries.is_empty() {
            let formatted: Vec<String> = entries.iter().map(|e| e.to_string()).collect();
            tracing::info!(proxies = ?formatted, "Loaded trusted proxies");
        }

        Self(Arc::new(entries))
    }

    fn contains(&self, ip: &IpAddr) -> bool {
        self.0.iter().any(|entry| entry.contains(ip))
    }
}

pub fn new_auth_failure_rate_limiter() -> AuthFailureRateLimiter {
    Arc::new(RateLimiter::keyed(
        Quota::per_minute(NonZeroU32::new(50).unwrap()).allow_burst(NonZeroU32::new(20).unwrap()),
    ))
}

pub fn new_health_check_rate_limiter() -> HealthCheckRateLimiter {
    Arc::new(RateLimiter::keyed(
        Quota::per_second(NonZeroU32::new(20).unwrap()).allow_burst(NonZeroU32::new(50).unwrap()),
    ))
}

pub fn extract_client_ip(
    headers: &http::HeaderMap,
    peer_addr: Option<IpAddr>,
    trusted_proxies: &TrustedProxies,
) -> IpAddr {
    let peer = peer_addr.unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if !trusted_proxies.contains(&peer) {
        return peer;
    }

    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first_ip) = forwarded.split(',').next()
        && let Ok(ip) = first_ip.trim().parse::<IpAddr>()
    {
        return ip;
    }

    if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok())
        && let Ok(ip) = real_ip.trim().parse::<IpAddr>()
    {
        return ip;
    }

    peer
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn exact_match() {
        let entry = TrustedEntry::Exact(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(entry.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!entry.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))));
    }

    #[test]
    fn cidr_v4() {
        let entry = TrustedEntry::Cidr(IpAddr::V4(Ipv4Addr::new(10, 244, 0, 0)), 16);
        assert!(entry.contains(&IpAddr::V4(Ipv4Addr::new(10, 244, 0, 1))));
        assert!(entry.contains(&IpAddr::V4(Ipv4Addr::new(10, 244, 251, 140))));
        assert!(!entry.contains(&IpAddr::V4(Ipv4Addr::new(10, 245, 0, 1))));
    }

    #[test]
    fn cidr_v6() {
        let entry = TrustedEntry::Cidr(IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 0)), 8);
        assert!(entry.contains(&IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1))));
        assert!(entry.contains(&IpAddr::V6(Ipv6Addr::new(0xfdff, 0xff, 0, 0, 0, 0, 0, 1))));
        assert!(!entry.contains(&IpAddr::V6(Ipv6Addr::new(0xfe00, 0, 0, 0, 0, 0, 0, 1))));
    }

    #[test]
    fn v4_v6_mismatch() {
        let entry = TrustedEntry::Cidr(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8);
        assert!(!entry.contains(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn parse_exact() {
        let entry = parse_trusted_entry("10.244.251.140").unwrap();
        assert!(matches!(entry, TrustedEntry::Exact(IpAddr::V4(_))));
    }

    #[test]
    fn parse_cidr() {
        let entry = parse_trusted_entry("10.244.0.0/16").unwrap();
        assert!(matches!(entry, TrustedEntry::Cidr(IpAddr::V4(_), 16)));
    }

    #[test]
    fn parse_invalid_prefix() {
        assert!(parse_trusted_entry("10.0.0.0/33").is_err());
    }

    #[test]
    fn extract_uses_peer_when_untrusted() {
        let headers = http::HeaderMap::new();
        let proxies = TrustedProxies(Arc::new(vec![]));
        let peer = Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
        assert_eq!(
            extract_client_ip(&headers, peer, &proxies),
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))
        );
    }

    #[test]
    fn extract_ignores_headers_when_untrusted() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
        let proxies = TrustedProxies(Arc::new(vec![]));
        let peer = Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
        assert_eq!(
            extract_client_ip(&headers, peer, &proxies),
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))
        );
    }

    #[test]
    fn extract_reads_xff_when_trusted() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9, 10.0.0.1".parse().unwrap());
        let proxies = TrustedProxies(Arc::new(vec![TrustedEntry::Exact(IpAddr::V4(
            Ipv4Addr::new(10, 244, 251, 140),
        ))]));
        let peer = Some(IpAddr::V4(Ipv4Addr::new(10, 244, 251, 140)));
        assert_eq!(
            extract_client_ip(&headers, peer, &proxies),
            IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9))
        );
    }

    #[test]
    fn extract_reads_xff_when_trusted_cidr() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
        let proxies = TrustedProxies(Arc::new(vec![TrustedEntry::Cidr(
            IpAddr::V4(Ipv4Addr::new(10, 244, 0, 0)),
            16,
        )]));
        let peer = Some(IpAddr::V4(Ipv4Addr::new(10, 244, 251, 140)));
        assert_eq!(
            extract_client_ip(&headers, peer, &proxies),
            IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9))
        );
    }
}
