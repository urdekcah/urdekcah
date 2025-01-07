// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::fs;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use url::Url;

use crate::{
  config::WeatherConfig,
  error::WeatherError,
  models::{api::WeatherResponse, weather::WeatherInfo},
  API_BASE_URL, REQUEST_TIMEOUT, WEATHER_END,
};

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
      cache: tokio::sync::RwLock::new(None),
    }
  }

  #[instrument(skip(self))]
  async fn get_city_from_section(&self) -> Result<String> {
    let content =
      fs::read_to_string(&self.config.readme_path).context("Failed to read README file")?;

    let start_marker = content
      .find("<!--START_SECTION:weather:")
      .ok_or(WeatherError::WeatherSectionNotFound)?;

    let city_start = start_marker + "<!--START_SECTION:weather:".len();
    let city_end = content[city_start..]
      .find("-->")
      .ok_or(WeatherError::MissingCityInSection)?;

    let city = content[city_start..city_start + city_end].trim();

    if city.is_empty() {
      return Err(WeatherError::MissingCityInSection.into());
    }

    Ok(city.to_string())
  }

  #[instrument(skip(self))]
  async fn fetch_weather(&self, city: &str) -> Result<WeatherInfo> {
    if let Some((cached_info, cached_time)) = self.cache.read().await.as_ref() {
      if Utc::now() - cached_time < chrono::Duration::from_std(self.config.cache_duration)? {
        info!("Returning cached weather data for {}", city);
        return Ok(cached_info.clone());
      }
    }

    if city.trim().is_empty() {
      return Err(WeatherError::InvalidCity("City name cannot be empty".into()).into());
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
      reqwest::StatusCode::TOO_MANY_REQUESTS => return Err(WeatherError::RateLimitExceeded.into()),
      status => {
        return Err(
          WeatherError::ApiError(format!("API request failed with status: {}", status)).into(),
        )
      }
    }

    let weather_data: WeatherResponse = response
      .json()
      .await
      .context("Failed to parse weather response")?;

    if weather_data.cod != 200 {
      return Err(
        WeatherError::InvalidResponse(format!("Invalid response code: {}", weather_data.cod))
          .into(),
      );
    }

    let weather_info = WeatherInfo::from_response(weather_data)?;

    *self.cache.write().await = Some((weather_info.clone(), Utc::now()));

    Ok(weather_info)
  }

  #[instrument(skip(self))]
  async fn update_readme(&self, weather: &WeatherInfo) -> Result<()> {
    let content =
      fs::read_to_string(&self.config.readme_path).context("Failed to read README file")?;

    let section_start = content
      .find("<!--START_SECTION:weather:")
      .ok_or(WeatherError::WeatherSectionNotFound)?;

    let end_idx = content
      .find(WEATHER_END)
      .ok_or(WeatherError::WeatherSectionNotFound)?;

    let start_marker = &content[section_start
      ..content[section_start..]
        .find("-->")
        .map(|pos| section_start + pos + 3)
        .ok_or(WeatherError::WeatherSectionNotFound)?];

    let weather_text = weather.format_readme();

    let new_content = format!(
      "{}{}\n{}\n{}",
      &content[..section_start],
      start_marker,
      weather_text,
      &content[end_idx + 1 + WEATHER_END.len()..]
    );

    let temp_path = self.config.readme_path.with_extension("tmp");
    fs::write(&temp_path, &new_content).context("Failed to write temporary file")?;
    fs::rename(&temp_path, &self.config.readme_path).context("Failed to update README file")?;

    info!("Successfully updated weather information");
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<()> {
    info!("Starting weather update");

    let city = self.get_city_from_section().await?;
    info!("Found city in section header: {}", city);

    let weather = self.fetch_weather(&city).await?;
    self.update_readme(&weather).await?;
    Ok(())
  }
}
