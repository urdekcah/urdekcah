// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{config::TimeRange, error::WakaError};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

#[async_trait]
pub trait WakaTimeApi {
  async fn fetch_stats(&self, time_range: &TimeRange) -> anyhow::Result<WakaStats>;
}

#[derive(Debug, Clone)]
pub struct WakaTimeClient {
  client: Arc<reqwest::Client>,
  api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct WakaStats {
  pub start: String,
  pub end: String,
  pub languages: Vec<Language>,
  pub human_readable_total: Option<String>,
  pub human_readable_total_including_other_language: Option<String>,
  pub total_seconds: f64,
  pub total_seconds_including_other_language: f64,
}

#[derive(Debug, Deserialize)]
pub struct Language {
  pub name: String,
  pub text: String,
  pub percent: f64,
}

impl WakaTimeClient {
  pub fn new(api_key: &str) -> Self {
    Self {
      client: Arc::new(reqwest::Client::new()),
      api_key: api_key.to_string(),
    }
  }
}

#[async_trait]
impl WakaTimeApi for WakaTimeClient {
  #[instrument(skip(self))]
  async fn fetch_stats(&self, time_range: &TimeRange) -> anyhow::Result<WakaStats> {
    let encoded_key = STANDARD.encode(&self.api_key);
    let mut headers = HeaderMap::new();
    headers.insert(
      AUTHORIZATION,
      HeaderValue::from_str(&format!("Basic {}", encoded_key))?,
    );
    headers.insert(
      USER_AGENT,
      HeaderValue::from_static("Wakatime-FishyBot/0.1"),
    );

    let url = format!(
      "https://wakatime.com/api/v1/users/current/stats/{}",
      time_range
    );

    let response = self.client.get(&url).headers(headers).send().await?;

    if !response.status().is_success() {
      return Err(WakaError::ApiError(format!("API request failed: {}", response.status())).into());
    }

    let data: serde_json::Value = response.json().await?;

    match serde_json::from_value(data["data"].clone()) {
      Ok(stats) => {
        tracing::debug!("Successfully parsed WakaStats");
        Ok(stats)
      }
      Err(e) => {
        tracing::error!("Failed to parse API response: {}", e);
        Err(anyhow::anyhow!("Failed to parse API response: {}", e))
      }
    }
  }
}
