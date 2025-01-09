// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use base::Error;
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};
use url::Url;

use crate::{
  config::WeatherConfig,
  models::{api::WeatherResponse, weather::WeatherInfo},
  API_BASE_URL, DATETIME_FORMAT, HTML_COMMENT_END, LAST_UPDATE_PREFIX, REQUEST_TIMEOUT,
  START_SECTION_PREFIX, WEATHER_END,
};

#[derive(Debug, Clone)]
pub struct WeatherSection {
  city: String,
  last_update: Option<DateTime<Utc>>,
  start_pos: usize,
  end_pos: usize,
  #[allow(dead_code)]
  content: String,
}

impl WeatherSection {
  #[instrument(skip(content))]
  fn parse(content: &str) -> Result<Self> {
    let start_pos = content
      .find(START_SECTION_PREFIX)
      .ok_or(Error::WeatherSectionNotFound)?;

    let end_pos = content[start_pos..]
      .find(WEATHER_END)
      .map(|pos| start_pos + pos)
      .ok_or(Error::WeatherSectionNotFound)?;

    let city_start = start_pos + START_SECTION_PREFIX.len();
    let city_end = content[city_start..]
      .find(HTML_COMMENT_END)
      .ok_or(Error::MissingCityInSection)?;

    let city = content[city_start..city_start + city_end].trim();
    if city.is_empty() {
      return Err(Error::MissingCityInSection.into());
    }

    let section_content = &content[start_pos..end_pos];
    let last_update = Self::parse_last_update(section_content)?;

    Ok(Self {
      city: city.to_string(),
      last_update,
      start_pos,
      end_pos,
      content: content[start_pos..end_pos + WEATHER_END.len()].to_string(),
    })
  }

  fn parse_last_update(content: &str) -> Result<Option<DateTime<Utc>>> {
    let update_pos = match content.find(LAST_UPDATE_PREFIX) {
      Some(pos) => pos,
      None => {
        debug!("No last update timestamp found");
        return Ok(None);
      }
    };

    let timestamp_start = update_pos + LAST_UPDATE_PREFIX.len();
    let timestamp_end = content[timestamp_start..]
      .find(HTML_COMMENT_END)
      .unwrap_or(content.len());

    let timestamp = content[timestamp_start..timestamp_start + timestamp_end].trim();
    debug!("Found timestamp: {}", timestamp);

    match DateTime::parse_from_str(
      &format!("{} +0000", timestamp),
      format!("{} %z", DATETIME_FORMAT).as_str(),
    ) {
      Ok(dt) => {
        info!("Successfully parsed timestamp: {}", dt);
        Ok(Some(dt.with_timezone(&Utc)))
      }
      Err(e) => {
        warn!("Failed to parse timestamp '{}': {}", timestamp, e);
        Ok(None)
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct UpdateResult {
  pub weather: WeatherInfo,
  pub last_update: Option<DateTime<Utc>>,
  pub current_update: DateTime<Utc>,
}

pub struct WeatherService {
  config: WeatherConfig,
  client: Client,
  cache: RwLock<Option<(WeatherInfo, DateTime<Utc>)>>,
}

impl WeatherService {
  #[instrument(skip(config))]
  pub fn new(config: WeatherConfig) -> Self {
    Self {
      config,
      client: reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("Failed to create HTTP client"),
      cache: RwLock::new(None),
    }
  }

  #[instrument(skip(self))]
  async fn get_weather_section(&self) -> Result<WeatherSection> {
    let content =
      fs::read_to_string(&self.config.readme_path).context("Failed to read README file")?;

    WeatherSection::parse(&content)
  }

  #[instrument(skip(self))]
  async fn fetch_weather(&self, city: &str) -> Result<WeatherInfo> {
    if let Some((cached_info, cached_time)) = self.cache.read().await.as_ref() {
      if (Utc::now() - *cached_time)
        < chrono::Duration::from_std(self.config.cache_duration)
          .expect("Invalid duration conversion")
      {
        info!("Returning cached weather data for {}", city);
        return Ok(cached_info.clone());
      }
    }

    if city.trim().is_empty() {
      return Err(Error::InvalidCity("City name cannot be empty".into()).into());
    }

    let url = Url::parse_with_params(
      API_BASE_URL,
      &[
        ("q", city),
        ("appid", &self.config.api_key),
        ("units", "metric"),
      ],
    )
    .context("Failed to build API URL")?;

    let response = self
      .client
      .get(url)
      .send()
      .await
      .context("Failed to send API request")?;

    match response.status() {
      reqwest::StatusCode::OK => (),
      reqwest::StatusCode::TOO_MANY_REQUESTS => return Err(Error::RateLimitExceeded.into()),
      status => {
        return Err(Error::ApiError(format!("API request failed with status: {}", status)).into())
      }
    }

    let weather_data: WeatherResponse = response
      .json()
      .await
      .context("Failed to parse weather response")?;

    if weather_data.cod != 200 {
      return Err(
        Error::InvalidResponse(format!("Invalid response code: {}", weather_data.cod)).into(),
      );
    }

    let weather_info = WeatherInfo::from_response(weather_data)?;
    *self.cache.write().await = Some((weather_info.clone(), Utc::now()));

    Ok(weather_info)
  }

  #[instrument(skip(self))]
  async fn update_readme(&self, weather: &WeatherInfo, section: &WeatherSection) -> Result<()> {
    let content =
      fs::read_to_string(&self.config.readme_path).context("Failed to read README file")?;

    let weather_text = weather.format_readme();

    let new_content = format!(
      "{}{}{}-->\n{}{}",
      &content[..section.start_pos],
      START_SECTION_PREFIX,
      section.city,
      weather_text,
      &content[section.end_pos + WEATHER_END.len()..]
    );

    let temp_path = self.config.readme_path.with_extension("tmp");
    fs::write(&temp_path, &new_content).context("Failed to write temporary file")?;
    fs::rename(&temp_path, &self.config.readme_path).context("Failed to update README file")?;

    info!("Successfully updated weather information");
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<UpdateResult> {
    info!("Starting weather update");

    let section = self.get_weather_section().await?;
    info!(
      "Found city: {}, last update: {:?}",
      section.city, section.last_update
    );

    let weather = self.fetch_weather(&section.city).await?;
    let current_update = Utc::now();

    self.update_readme(&weather, &section).await?;

    Ok(UpdateResult {
      weather,
      last_update: section.last_update,
      current_update,
    })
  }
}
