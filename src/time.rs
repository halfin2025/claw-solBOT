use anyhow::{anyhow, Result};
use chrono::Datelike;

/// Returns YYYY-MM-DD in the configured timezone.
///
/// Uses chrono-tz; falls back to local time if tz parsing fails.
pub fn day_key(tz: &str) -> Result<String> {
    let tz: chrono_tz::Tz = tz.parse().map_err(|_| anyhow!("invalid tz: {tz}"))?;
    let now = chrono::Utc::now().with_timezone(&tz);
    Ok(format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day()))
}
