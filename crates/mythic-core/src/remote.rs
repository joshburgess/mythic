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
/// Returns a JSON map of `name → value` for each source.
pub fn fetch_remote_data(sources: &[RemoteSource], cache_dir: &Path) -> Result<Value> {
    if sources.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    let remote_cache = cache_dir.join("remote");
    std::fs::create_dir_all(&remote_cache)?;

    let mut result = serde_json::Map::new();

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
                eprintln!(
                    "  Warning: failed to fetch remote data '{}' from {}: {e}",
                    source.name, source.url
                );
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

    Ok(Value::Object(result))
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

fn fetch_url(url: &str) -> Result<Value> {
    let response =
        reqwest::blocking::get(url).with_context(|| format!("Failed to fetch: {url}"))?;

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
        let result = fetch_remote_data(&[], dir.path()).unwrap();
        assert_eq!(result, Value::Object(serde_json::Map::new()));
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
}
