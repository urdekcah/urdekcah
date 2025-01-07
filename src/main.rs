// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0, 
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use thiserror::Error;
use tokio;
use tracing::{error, info, instrument, warn};
use url::Url;

#[derive(Debug, Error)]
pub enum WeatherError {
  #[error("Weather section not found in README - skipping weather update")]
  WeatherSectionNotFound,
  #[error("API request failed: {0}")]
  ApiError(String),
  #[error("File operation failed: {0}")]
  FileError(#[from] std::io::Error),
  #[error("Invalid city name: {0}")]
  InvalidCity(String),
  #[error("Invalid API key")]
  InvalidApiKey,
  #[error("Invalid response from weather API: {0}")]
  InvalidResponse(String),
  #[error("Rate limit exceeded")]
  RateLimitExceeded,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
struct WeatherResponse {
  weather: Vec<Weather>,
  main: MainWeather,
  sys: SysInfo,
  name: String,
  cod: u16,
  timezone: i32,
}

#[derive(Debug, Deserialize, Clone)]
struct Weather {
  main: String,
  description: String,
}

#[derive(Debug, Deserialize, Clone)]
struct MainWeather {
  temp: f64,
  feels_like: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct SysInfo {
  sunrise: i64,
  sunset: i64,
  country: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeatherInfo {
  temp: f64,
  feels_like: f64,
  condition: String,
  condition_desc: String,
  sunrise: DateTime<FixedOffset>,
  sunset: DateTime<FixedOffset>,
  location: String,
  country: String,
}

impl WeatherInfo {
  fn from_response(response: WeatherResponse) -> Result<Self> {
    let tz_offset = FixedOffset::east_opt(response.timezone)
      .context("Invalid timezone offset")?;

    let weather = response.weather.first()
      .context("No weather data available")?;

    let sunrise = Utc.timestamp_opt(response.sys.sunrise, 0)
      .single()
      .context("Invalid sunrise timestamp")?
      .with_timezone(&tz_offset);

    let sunset = Utc.timestamp_opt(response.sys.sunset, 0)
      .single()
      .context("Invalid sunset timestamp")?
      .with_timezone(&tz_offset);

    Ok(Self {
      temp: response.main.temp,
      feels_like: response.main.feels_like,
      condition: weather.main.clone(),
      condition_desc: weather.description.clone(),
      sunrise,
      sunset,
      location: response.name,
      country: response.sys.country,
    })
  }

  fn format_readme(&self) -> String {
    let today = self.sunrise.format("%B %d, %Y");
    format!(
      "{}\nCurrently in **{}** ({}), the weather is: **{:.1}°C** (feels like **{:.1}°C**), ***{}***<br/>\n\
      On *{}*, the *sun rises* at 🌅**{}** and *sets* at 🌇**{}**.\n\
      {}",
      WEATHER_START,
      self.location,
      self.country,
      self.temp,
      self.feels_like,
      self.condition_desc,
      today,
      self.sunrise.format("%H:%M"),
      self.sunset.format("%H:%M"),
      WEATHER_END,
    )
  }
}

const WEATHER_START: &str = "<!--START_SECTION:weather-->";
const WEATHER_END: &str = "<!--END_SECTION:weather-->";
const API_BASE_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct WeatherConfig {
  api_key: String,
  readme_path: PathBuf,
  cache_duration: std::time::Duration,
}

impl WeatherConfig {
  pub fn new(
    api_key: impl Into<String>,
    readme_path: impl Into<PathBuf>,
    cache_duration: std::time::Duration,
  ) -> Result<Self> {
    let api_key = api_key.into();
    if api_key.trim().is_empty() {
      return Err(WeatherError::InvalidApiKey.into());
    }

    Ok(Self {
      api_key,
      readme_path: readme_path.into(),
      cache_duration,
    })
  }
}

#[derive(Debug)]
pub struct WeatherService {
  config: WeatherConfig,
  client: reqwest::Client,
  cache: tokio::sync::RwLock<Option<(WeatherInfo, DateTime<Utc>)>>,
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
      reqwest::StatusCode::TOO_MANY_REQUESTS => {
        return Err(WeatherError::RateLimitExceeded.into())
      }
      status => {
        return Err(WeatherError::ApiError(format!(
          "API request failed with status: {}",
          status
        ))
        .into())
      }
    }

    let weather_data: WeatherResponse = response
      .json()
      .await
      .context("Failed to parse weather response")?;

    if weather_data.cod != 200 {
      return Err(WeatherError::InvalidResponse(format!(
        "Invalid response code: {}",
        weather_data.cod
      ))
      .into());
    }

    let weather_info = WeatherInfo::from_response(weather_data)?;
    
    *self.cache.write().await = Some((weather_info.clone(), Utc::now()));

    Ok(weather_info)
  }

  #[instrument(skip(self))]
  async fn has_weather_section(&self) -> Result<bool> {
    let content = fs::read_to_string(&self.config.readme_path)
      .context("Failed to read README file")?;
        
    Ok(content.contains(WEATHER_START) && content.contains(WEATHER_END))
  }

  #[instrument(skip(self))]
  async fn update_readme(&self, weather: &WeatherInfo) -> Result<()> {
    let content = fs::read_to_string(&self.config.readme_path)
      .context("Failed to read README file")?;

    let start_idx = content
      .find(WEATHER_START)
      .ok_or(WeatherError::WeatherSectionNotFound)?;
    let end_idx = content
      .find(WEATHER_END)
      .ok_or(WeatherError::WeatherSectionNotFound)?;

    let weather_text = weather.format_readme();

    let new_content = format!(
      "{}{}{}",
      &content[..start_idx],
      weather_text,
      &content[end_idx + WEATHER_END.len()..]
    );

    let temp_path = self.config.readme_path.with_extension("tmp");
    fs::write(&temp_path, &new_content).context("Failed to write temporary file")?;
    fs::rename(&temp_path, &self.config.readme_path)
      .context("Failed to update README file")?;

    info!("Successfully updated weather information");
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn run(&self, city: &str) -> Result<()> {
    if !self.has_weather_section().await? {
      info!("No weather section found in README - skipping update");
      return Ok(());
    }

    info!("Starting weather update for city: {}", city);
    let weather = self.fetch_weather(city).await?;
    self.update_readme(&weather).await?;
    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_thread_ids(true)
    .init();

  let api_key = std::env::var("OPENWEATHER_API_KEY")
    .context("Missing OPENWEATHER_API_KEY environment variable")?;

  let config = WeatherConfig::new(
    api_key,
    "README.md",
    std::time::Duration::from_secs(300),
  )?;

  let service = WeatherService::new(config);
  let city = std::env::args()
    .nth(1)
    .unwrap_or_else(|| "moscow".to_string());

  if let Err(e) = service.run(&city).await {
    error!("Failed to update weather: {:?}", e);
    std::process::exit(1);
  }

  Ok(())
}