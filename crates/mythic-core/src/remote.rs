//! Remote data fetching with file-system caching.
//!
//! Fetches JSON, YAML, or plain text from URLs and caches the response
//! to disk with a configurable TTL. Cached data is loaded from
//! `_data/remote/` and made available in templates alongside local data.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// A remote data source definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteSource {
    /// URL to fetch.
    pub url: String,
    /// Key name for the data (accessible as `{{ data.remote.<name> }}`).
    pub name: String,
    /// Cache TTL in seconds. Default: 3600 (1 hour).
    #[serde(default = "default_ttl")]
    pub ttl: u64,
}

fn default_ttl() -> u64 {
    3600
}

/// Fetch all remote data sources, using cached versions when available.
///
/// Returns a JSON map of `name → value` for each source, plus any warnings
/// for sources that failed to fetch.
pub fn fetch_remote_data(
    sources: &[RemoteSource],
    cache_dir: &Path,
) -> Result<(Value, Vec<String>)> {
    if sources.is_empty() {
        return Ok((Value::Object(serde_json::Map::new()), Vec::new()));
    }

    let remote_cache = cache_dir.join("remote");
    std::fs::create_dir_all(&remote_cache)?;

    let mut result = serde_json::Map::new();
    let mut warnings = Vec::new();

    for source in sources {
        let cache_file = remote_cache.join(format!("{}.json", source.name));
        let ttl = Duration::from_secs(source.ttl);

        // Check cache
        if let Some(cached) = read_cache(&cache_file, ttl) {
            result.insert(source.name.clone(), cached);
            continue;
        }

        // Fetch from URL
        match fetch_url(&source.url) {
            Ok(data) => {
                // Write cache
                if let Ok(json_str) = serde_json::to_string_pretty(&data) {
                    std::fs::write(&cache_file, json_str).ok();
                }
                result.insert(source.name.clone(), data);
            }
            Err(e) => {
                warnings.push(format!(
                    "failed to fetch remote data '{}' from {}: {e}",
                    source.name, source.url
                ));
                // Try stale cache as fallback
                if cache_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cache_file) {
                        if let Ok(val) = serde_json::from_str(&content) {
                            result.insert(source.name.clone(), val);
                            continue;
                        }
                    }
                }
                result.insert(source.name.clone(), Value::Null);
            }
        }
    }

    Ok((Value::Object(result), warnings))
}

fn read_cache(path: &Path, ttl: Duration) -> Option<Value> {
    if !path.exists() {
        return None;
    }

    // Check if cache is fresh
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;

    if age > ttl {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn is_private_host(host: &str) -> bool {
    let host_lower = host.to_lowercase();

    // Block obvious private hostnames
    if host_lower == "localhost"
        || host_lower == "[::1]"
        || host_lower == "0.0.0.0"
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
    {
        return true;
    }

    // Block private IP ranges by pattern
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return is_private_ip(&ip);
    }

    false
}

fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || {
                // Check for IPv6-mapped IPv4 addresses (::ffff:x.x.x.x)
                let segments = v6.segments();
                if segments[0..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
                    let v4 = std::net::Ipv4Addr::new(
                        (segments[6] >> 8) as u8,
                        segments[6] as u8,
                        (segments[7] >> 8) as u8,
                        segments[7] as u8,
                    );
                    v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
                } else {
                    false
                }
            }
        }
    }
}

fn fetch_url(url: &str) -> Result<Value> {
    // Validate URL and block private/internal addresses (SSRF prevention)
    let parsed: reqwest::Url = url.parse().with_context(|| format!("Invalid URL: {url}"))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL has no host: {url}"))?;

    if is_private_host(host) {
        anyhow::bail!("Blocked request to private/internal address: {url}");
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .with_context(|| "Failed to build HTTP client")?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to fetch: {url}"))?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} from {url}");
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = response.text()?;

    // Parse based on content type
    if content_type.contains("json") {
        Ok(serde_json::from_str(&body)?)
    } else if content_type.contains("yaml") || content_type.contains("yml") {
        Ok(serde_yaml::from_str(&body)?)
    } else {
        // Try JSON first, fall back to string value
        serde_json::from_str(&body).or_else(|_| Ok(Value::String(body)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sources_returns_empty_object() {
        let dir = tempfile::tempdir().unwrap();
        let (result, warnings) = fetch_remote_data(&[], dir.path()).unwrap();
        assert_eq!(result, Value::Object(serde_json::Map::new()));
        assert!(warnings.is_empty());
    }

    #[test]
    fn cache_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("remote");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let cache_file = cache_dir.join("test.json");
        let data = serde_json::json!({"key": "value"});
        std::fs::write(&cache_file, serde_json::to_string(&data).unwrap()).unwrap();

        let cached = read_cache(&cache_file, Duration::from_secs(3600));
        assert!(cached.is_some());
        assert_eq!(cached.unwrap()["key"], "value");
    }

    #[test]
    fn expired_cache_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cache_file = dir.path().join("expired.json");
        std::fs::write(&cache_file, r#"{"old": true}"#).unwrap();

        // TTL of 0 seconds means always expired
        let cached = read_cache(&cache_file, Duration::from_secs(0));
        assert!(cached.is_none());
    }

    #[test]
    fn missing_cache_returns_none() {
        let cached = read_cache(
            Path::new("/nonexistent/cache.json"),
            Duration::from_secs(3600),
        );
        assert!(cached.is_none());
    }

    #[test]
    fn private_ipv4_addresses_blocked() {
        // Loopback
        assert!(is_private_host("127.0.0.1"), "127.0.0.1 should be blocked");
        assert!(is_private_host("127.0.0.2"), "127.0.0.2 should be blocked");
        // RFC 1918 private ranges
        assert!(is_private_host("10.0.0.0"), "10.0.0.0 should be blocked");
        assert!(is_private_host("10.255.255.255"), "10.x should be blocked");
        assert!(
            is_private_host("192.168.1.1"),
            "192.168.1.1 should be blocked"
        );
        assert!(
            is_private_host("172.16.0.1"),
            "172.16.0.1 should be blocked"
        );
        // Link-local
        assert!(
            is_private_host("169.254.1.1"),
            "169.254.x.x should be blocked"
        );
        // Unspecified
        assert!(is_private_host("0.0.0.0"), "0.0.0.0 should be blocked");
    }

    #[test]
    fn localhost_and_local_domains_blocked() {
        assert!(is_private_host("localhost"), "localhost should be blocked");
        assert!(
            is_private_host("myhost.local"),
            ".local domains should be blocked"
        );
        assert!(
            is_private_host("service.internal"),
            ".internal domains should be blocked"
        );
        assert!(is_private_host("[::1]"), "[::1] should be blocked");
    }

    #[test]
    fn public_hosts_not_blocked() {
        assert!(!is_private_host("example.com"), "example.com is public");
        assert!(!is_private_host("8.8.8.8"), "8.8.8.8 is public");
        assert!(!is_private_host("1.1.1.1"), "1.1.1.1 is public");
    }

    #[test]
    fn ipv6_mapped_ipv4_private_addresses_blocked() {
        // ::ffff:127.0.0.1 is IPv6-mapped loopback
        assert!(
            is_private_host("::ffff:127.0.0.1"),
            "IPv6-mapped 127.0.0.1 should be blocked"
        );
        // ::ffff:10.0.0.1 is IPv6-mapped private
        assert!(
            is_private_host("::ffff:10.0.0.1"),
            "IPv6-mapped 10.0.0.1 should be blocked"
        );
        // ::ffff:192.168.1.1 is IPv6-mapped private
        assert!(
            is_private_host("::ffff:192.168.1.1"),
            "IPv6-mapped 192.168.1.1 should be blocked"
        );
    }

    #[test]
    fn ipv6_loopback_blocked() {
        assert!(
            is_private_host("::1"),
            "IPv6 loopback ::1 should be blocked"
        );
    }

    #[test]
    fn fetch_url_blocks_private_addresses() {
        let result = fetch_url("http://127.0.0.1/secret");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("private") || err.contains("Blocked"),
            "Error should mention blocking private address, got: {err}"
        );
    }
}
