// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::API_TIMEOUT;
use crate::USER_AGENT;
use async_trait::async_trait;
use base::{Error, WakaTimeRange};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::timeout;
use tracing::{error, instrument};

#[async_trait]
pub trait WakaTimeApi: Send + Sync {
  async fn fetch_stats(&self, time_range: &WakaTimeRange) -> Result<WakaStats, Error>;
}

#[derive(Debug, Clone)]
pub struct WakaTimeClient {
  client: Arc<reqwest::Client>,
  api_key: String,
  base_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WakaStats {
  pub start: String,
  pub end: String,
  pub languages: Vec<Language>,
  pub human_readable_total: Option<String>,
  pub human_readable_total_including_other_language: Option<String>,
  #[serde(default)]
  pub total_seconds: f64,
  #[serde(default)]
  pub total_seconds_including_other_language: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Language {
  pub name: String,
  pub text: String,
  #[serde(default)]
  pub percent: f64,
}

impl WakaTimeClient {
  pub fn new(api_key: &str) -> Self {
    let client = reqwest::Client::builder()
      .timeout(API_TIMEOUT)
      .build()
      .expect("Failed to create HTTP client");

    Self {
      client: Arc::new(client),
      api_key: api_key.to_string(),
      base_url: "https://wakatime.com/api".into(),
    }
  }

  #[cfg(test)]
  pub fn with_base_url(api_key: &str, base_url: &str) -> Self {
    let mut client = Self::new(api_key);
    client.base_url = base_url.to_string();
    client
  }

  fn build_headers(&self) -> Result<reqwest::header::HeaderMap, Error> {
    let mut headers = reqwest::header::HeaderMap::new();
    let encoded_key = STANDARD.encode(&self.api_key);

    headers.insert(
      reqwest::header::AUTHORIZATION,
      reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded_key))
        .map_err(|e| Error::ApiError(format!("Invalid API key: {}", e)))?,
    );

    headers.insert(
      reqwest::header::USER_AGENT,
      reqwest::header::HeaderValue::from_static(USER_AGENT),
    );

    Ok(headers)
  }
}

#[async_trait]
impl WakaTimeApi for WakaTimeClient {
  #[instrument(skip(self))]
  async fn fetch_stats(&self, time_range: &WakaTimeRange) -> Result<WakaStats, Error> {
    let url = format!("{}/v1/users/current/stats/{}", self.base_url, time_range);
    let headers = self.build_headers()?;

    let response = timeout(API_TIMEOUT, self.client.get(&url).headers(headers).send())
      .await
      .map_err(|_| Error::TimeoutError)??;

    if !response.status().is_success() {
      error!("API request failed with status: {}", response.status());
      return Err(Error::ApiError(format!(
        "API request failed: {}",
        response.status()
      )));
    }

    let data: serde_json::Value = response
      .json()
      .await
      .map_err(|e| Error::ParseError(format!("Failed to deserialize response: {}", e)))?;

    serde_json::from_value(data["data"].clone())
      .map_err(|e| Error::ParseError(format!("Failed to parse WakaStats: {}", e)))
  }
}
