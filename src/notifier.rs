use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Serialize;

#[derive(Clone)]
pub struct Notifier {
    slack_webhook_url: Option<String>,
    http: Client,
}

#[derive(Serialize)]
struct SlackPayload<'a> {
    text: &'a str,
}

impl Notifier {
    pub fn new(slack_webhook_url: Option<String>) -> Self {
        Self { slack_webhook_url, http: Client::new() }
    }

    pub async fn alert(&self, text: &str) -> Result<()> {
        let Some(url) = self.slack_webhook_url.as_deref() else {
            // In dev/test we allow missing slack config.
            return Ok(());
        };

        let resp = self
            .http
            .post(url)
            .json(&SlackPayload { text })
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("slack webhook failed: {}", resp.status()));
        }

        Ok(())
    }
}
