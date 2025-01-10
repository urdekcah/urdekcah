// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{template::Template, wakatime::WakaStats, MARKDOWN_MARKERS};
use base::{Config, Error, WakaTimeRange};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use reqwest;
use std::{fs, path::Path, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

const API_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT_STRING: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct UpdateResult {
  pub stats: String,
  pub last_update: Option<DateTime<Utc>>,
  pub current_update: DateTime<Utc>,
  pub was_updated: bool,
}

pub struct WakaTimeService {
  config: Config,
  client: reqwest::Client,
  cache: RwLock<Option<(WakaStats, DateTime<Utc>)>>,
  base_url: String,
  api_key: String,
}

impl WakaTimeService {
  pub fn new(config: Config, api_key: String) -> Self {
    let client = reqwest::Client::builder()
      .timeout(API_TIMEOUT)
      .build()
      .expect("Failed to create HTTP client");

    Self {
      config,
      client,
      cache: RwLock::new(None),
      base_url: "https://wakatime.com/api".into(),
      api_key,
    }
  }

  #[cfg(test)]
  pub fn with_base_url(config: Config, api_key: String, base_url: String) -> Self {
    Self {
      config,
      client: reqwest::Client::builder()
        .timeout(API_TIMEOUT)
        .build()
        .expect("Failed to create HTTP client"),
      cache: RwLock::new(None),
      base_url,
      api_key,
    }
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<UpdateResult, Error> {
    info!("Starting WakaTime stats update");

    let stats = self.fetch_stats(&self.config.wakatime.time_range).await?;
    let content = self.prepare_content(&stats)?;

    let update_result = self.update_readme(Path::new("README.md"), &content)?;

    Ok(update_result)
  }

  async fn fetch_stats(&self, time_range: &WakaTimeRange) -> Result<WakaStats, Error> {
    let url = format!("{}/v1/users/current/stats/{}", self.base_url, time_range);
    let headers = self.build_headers()?;

    let response = tokio::time::timeout(API_TIMEOUT, self.client.get(&url).headers(headers).send())
      .await
      .map_err(|_| Error::TimeoutError)??;

    if !response.status().is_success() {
      return Err(Error::ApiError(format!(
        "API request failed: {}",
        response.status()
      )));
    }

    let data: serde_json::Value = response
      .json()
      .await
      .map_err(|e| Error::ParseError(format!("Failed to deserialize response: {}", e)))?;

    let stats: WakaStats = serde_json::from_value(data["data"].clone())
      .map_err(|e| Error::ParseError(format!("Failed to parse WakaStats: {}", e)))?;

    *self.cache.write().await = Some((stats.clone(), Utc::now()));

    Ok(stats)
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
      reqwest::header::HeaderValue::from_static(USER_AGENT_STRING),
    );

    Ok(headers)
  }

  fn prepare_content(&self, stats: &WakaStats) -> Result<String, Error> {
    let template = Template::new(self.config.clone());
    template.render(stats)
  }

  fn update_readme<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<UpdateResult, Error> {
    let readme = fs::read_to_string(path.as_ref())?;

    let start_comment = format!("<!--START_SECTION:{}-->", self.config.wakatime.section_name);
    let end_comment = format!("<!--END_SECTION:{}-->", self.config.wakatime.section_name);

    let last_update = self.parse_last_update(&readme);
    let current_update = Utc::now();

    let replacement = format!(
      "{}\n<!--LAST_WAKA_UPDATE:{}-->\n```{}\n{}```\n{}",
      start_comment,
      current_update.format(MARKDOWN_MARKERS.datetime_format),
      self.config.wakatime.code_lang,
      content,
      end_comment
    );

    let pattern = format!(
      "{}[\\s\\S]+{}",
      regex::escape(&start_comment),
      regex::escape(&end_comment)
    );

    let re = regex::Regex::new(&pattern)
      .map_err(|e| Error::TemplateError(format!("Invalid regex pattern: {}", e)))?;

    let new_readme = re.replace(&readme, replacement);
    let was_updated = new_readme != readme;

    if was_updated {
      fs::write(path, new_readme.as_bytes())?;
      debug!("README updated successfully");
    } else {
      debug!("No changes needed in README");
    }

    Ok(UpdateResult {
      stats: content.to_string(),
      last_update,
      current_update,
      was_updated,
    })
  }

  fn parse_last_update(&self, content: &str) -> Option<DateTime<Utc>> {
    let update_pos = content.find(MARKDOWN_MARKERS.last_update_prefix)?;
    let timestamp_start = update_pos + MARKDOWN_MARKERS.last_update_prefix.len();
    let timestamp_end = content[timestamp_start..]
      .find(MARKDOWN_MARKERS.html_comment_end)
      .unwrap_or(content.len());

    let timestamp = content[timestamp_start..timestamp_start + timestamp_end].trim();
    DateTime::parse_from_str(
      &format!("{} +0000", timestamp),
      format!("{} %z", MARKDOWN_MARKERS.datetime_format).as_str(),
    )
    .map(|dt| dt.with_timezone(&Utc))
    .ok()
  }
}
