use anyhow::Result;
use chrono::{DateTime, Local};
use std::{fs::OpenOptions, io::Write, path::Path};

pub fn append_line(path: impl AsRef<Path>, line: &str) -> Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

pub fn heartbeat_line() -> String {
    let now: DateTime<Local> = Local::now();
    format!("{} heartbeat", now.to_rfc3339())
}
