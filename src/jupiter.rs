use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct JupiterClient {
    base_url: String,
    http: Client,
}

impl JupiterClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::new(),
        }
    }

    pub async fn quote(&self, req: QuoteRequest) -> Result<QuoteResponse> {
        let url = format!("{}/quote", self.base_url);
        let resp = self
            .http
            .get(url)
            .query(&req)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn swap(&self, req: SwapRequest) -> Result<SwapResponse> {
        let url = format!("{}/swap", self.base_url);
        let resp = self
            .http
            .post(url)
            .json(&req)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    /// Amount in the input mint's base units (e.g. USDC has 6 decimals)
    pub amount: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u64,
    #[serde(rename = "onlyDirectRoutes", skip_serializing_if = "Option::is_none")]
    pub only_direct_routes: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    #[serde(rename = "outAmount")]
    pub out_amount: String,

    /// Jupiter returns a rich route plan. We keep it as JSON for forward-compat.
    #[serde(flatten)]
    pub rest: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    #[serde(rename = "quoteResponse")]
    pub quote_response: serde_json::Value,

    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,

    /// Recommended: true
    #[serde(rename = "wrapAndUnwrapSol", skip_serializing_if = "Option::is_none")]
    pub wrap_and_unwrap_sol: Option<bool>,

    /// If provided, Jupiter will include it in the transaction.
    #[serde(rename = "computeUnitPriceMicroLamports", skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_micro_lamports: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResponse {
    /// base64 encoded VersionedTransaction
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,

    #[serde(flatten)]
    pub rest: serde_json::Value,
}

pub fn ensure_slippage_bounds(slippage_bps: u64, max_slippage_bps: u64) -> Result<()> {
    if slippage_bps == 0 {
        return Err(anyhow!("slippage_bps cannot be 0"));
    }
    if slippage_bps > max_slippage_bps {
        return Err(anyhow!(
            "slippage_bps ({slippage_bps}) exceeds max_slippage_bps ({max_slippage_bps})"
        ));
    }
    Ok(())
}
