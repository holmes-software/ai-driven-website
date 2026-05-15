use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use redis::AsyncCommands;
use tokio::sync::Mutex;

#[cfg(any(debug_assertions, test))]
use chrono::{DateTime, Duration, Utc};
#[cfg(any(debug_assertions, test))]
use std::path::Path;

/// TTL applied to cached CSS, regardless of backend.
pub const CACHE_TTL_MINUTES: i64 = 1440;

/// A pluggable cache for generated CSS, keyed on a validated theme slug.
#[rocket::async_trait]
pub trait StyleCache: Send + Sync {
    async fn get(&self, theme: &str) -> Option<String>;
    async fn put(&self, theme: &str, css: &str) -> Result<()>;
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Disk cache: <datetime>-<theme>-styles.css
// (debug + test only — release builds always use Redis)
// ---------------------------------------------------------------------------

#[cfg(any(debug_assertions, test))]
pub struct DiskCache {
    dir: PathBuf,
}

#[cfg(any(debug_assertions, test))]
impl DiskCache {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

#[cfg(any(debug_assertions, test))]
#[rocket::async_trait]
impl StyleCache for DiskCache {
    async fn get(&self, theme: &str) -> Option<String> {
        match find_fresh_for_theme(&self.dir, theme).await {
            Ok(opt) => opt,
            Err(e) => {
                eprintln!("[cache:disk] read error: {e:#}");
                None
            }
        }
    }

    async fn put(&self, theme: &str, css: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        // RFC3339 contains ':' which isn't filename-safe on Windows.
        let safe_now = now.replace(':', "_");
        let path = self.dir.join(format!("{safe_now}-{theme}-styles.css"));
        tokio::fs::write(&path, css)
            .await
            .with_context(|| format!("writing cache file {}", path.display()))?;
        Ok(())
    }
}

#[cfg(any(debug_assertions, test))]
async fn find_fresh_for_theme(dir: &Path, theme: &str) -> Result<Option<String>> {
    let suffix = format!("-{theme}-styles.css");
    let mut latest: Option<(DateTime<Utc>, PathBuf)> = None;

    let mut rd = tokio::fs::read_dir(dir)
        .await
        .with_context(|| format!("reading cache dir {}", dir.display()))?;
    while let Some(entry) = rd.next_entry().await? {
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        if !name.ends_with(&suffix) {
            continue;
        }
        let ts_part = &name[..name.len() - suffix.len()];
        let ts_decoded = ts_part.replace('_', ":");
        let Ok(ts) = DateTime::parse_from_rfc3339(&ts_decoded) else {
            continue;
        };
        let ts = ts.with_timezone(&Utc);
        if latest.as_ref().is_none_or(|(prev, _)| ts > *prev) {
            latest = Some((ts, entry.path()));
        }
    }

    let Some((ts, path)) = latest else {
        return Ok(None);
    };

    if Utc::now() > ts + Duration::minutes(CACHE_TTL_MINUTES) {
        return Ok(None);
    }

    let css = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("reading cached css at {}", path.display()))?;
    Ok(Some(css))
}

// ---------------------------------------------------------------------------
// Redis cache: keys are styles:<theme>, TTL set on write.
//
// Designed for Azure Cache for Redis / Azure Managed Redis. Use a TLS URL like:
//   rediss://:<access-key>@<name>.redis.cache.windows.net:6380
// ---------------------------------------------------------------------------

pub struct RedisCache {
    conn: Arc<Mutex<redis::aio::ConnectionManager>>,
}

impl RedisCache {
    pub async fn connect(url: &str) -> Result<Self> {
        let client = redis::Client::open(url).context("invalid REDIS_URL")?;
        let conn = redis::aio::ConnectionManager::new(client)
            .await
            .context("connecting to Redis")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn key(theme: &str) -> String {
        format!("styles:{theme}")
    }
}

#[rocket::async_trait]
impl StyleCache for RedisCache {
    async fn get(&self, theme: &str) -> Option<String> {
        let mut conn = self.conn.lock().await;
        match conn.get::<_, Option<String>>(Self::key(theme)).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[cache:redis] GET error: {e}");
                None
            }
        }
    }

    async fn put(&self, theme: &str, css: &str) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let ttl_seconds = (CACHE_TTL_MINUTES * 60) as u64;
        let _: () = conn
            .set_ex(Self::key(theme), css, ttl_seconds)
            .await
            .map_err(|e| anyhow!("redis SETEX failed: {e}"))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

/// In debug builds, fall back to disk if `REDIS_URL` is unset or unreachable.
/// In release builds, `REDIS_URL` is required and connection failures panic
/// — production must not silently degrade to ephemeral disk inside a container.
#[cfg(not(debug_assertions))]
pub async fn build(_cache_dir: PathBuf) -> Arc<dyn StyleCache> {
    let url = std::env::var("REDIS_URL").expect("REDIS_URL is required in release builds");
    let cache = RedisCache::connect(&url)
        .await
        .expect("could not connect to Redis");
    println!("[cache] using Redis cache");
    Arc::new(cache)
}

#[cfg(debug_assertions)]
pub async fn build(cache_dir: PathBuf) -> Arc<dyn StyleCache> {
    if let Ok(url) = std::env::var("REDIS_URL") {
        match RedisCache::connect(&url).await {
            Ok(c) => {
                println!("[cache] using Redis cache");
                return Arc::new(c);
            }
            Err(e) => {
                println!("[cache] Redis unavailable, falling back to disk: {e:#}");
            }
        }
    } else {
        println!("[cache] REDIS_URL not set; using disk cache at {cache_dir:?}");
    }
    Arc::new(DiskCache::new(cache_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir() -> PathBuf {
        let d = tempfile::tempdir().expect("tempdir");
        let p = d.path().to_path_buf();
        // Leak the TempDir to keep it alive for the test's duration; OS cleans /tmp.
        std::mem::forget(d);
        p
    }

    #[tokio::test]
    async fn disk_cache_roundtrip() {
        let cache = DiskCache::new(unique_dir());
        assert!(cache.get("dark").await.is_none());
        cache.put("dark", "body{color:red}").await.unwrap();
        assert_eq!(cache.get("dark").await.as_deref(), Some("body{color:red}"));
    }

    #[tokio::test]
    async fn disk_cache_isolates_themes() {
        let cache = DiskCache::new(unique_dir());
        cache.put("dark", "DARK").await.unwrap();
        cache.put("warm", "WARM").await.unwrap();
        assert_eq!(cache.get("dark").await.as_deref(), Some("DARK"));
        assert_eq!(cache.get("warm").await.as_deref(), Some("WARM"));
        assert!(cache.get("forest").await.is_none());
    }

    #[tokio::test]
    async fn disk_cache_treats_stale_as_missing() {
        let dir = unique_dir();
        let stale_ts = (Utc::now() - Duration::minutes(CACHE_TTL_MINUTES + 1))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            .replace(':', "_");
        let path = dir.join(format!("{stale_ts}-dark-styles.css"));
        tokio::fs::write(&path, "old").await.unwrap();
        let cache = DiskCache::new(dir);
        assert!(cache.get("dark").await.is_none());
    }

    #[tokio::test]
    async fn disk_cache_ignores_unrelated_files() {
        let dir = unique_dir();
        tokio::fs::write(dir.join("README.md"), "hi").await.unwrap();
        tokio::fs::write(dir.join("not-a-cache-file.css"), "x")
            .await
            .unwrap();
        let cache = DiskCache::new(dir);
        assert!(cache.get("dark").await.is_none());
    }
}
