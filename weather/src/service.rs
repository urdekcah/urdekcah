// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{
  config::WeatherConfig,
  constants::*,
  models::{api::WeatherResponse, weather::WeatherInfo},
};
use async_trait::async_trait;
use base::Error;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};
use url::Url;

#[async_trait]
pub trait WeatherProvider: Send + Sync {
  async fn fetch_weather(&self, city: &str) -> Result<WeatherInfo, Error>;
}

pub struct WeatherService {
  config: WeatherConfig,
  client: reqwest::Client,
  cache: RwLock<Option<(WeatherInfo, DateTime<Utc>)>>,
}

#[derive(Debug, Clone)]
pub struct UpdateResult {
  pub weather: WeatherInfo,
  pub last_update: Option<DateTime<Utc>>,
  pub current_update: DateTime<Utc>,
}

impl WeatherService {
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

  async fn get_weather_section(&self) -> Result<WeatherSection, Error> {
    let content = tokio::fs::read_to_string(&self.config.readme_path).await?;
    WeatherSection::parse(&content)
  }

  async fn fetch_weather(&self, city: &str) -> Result<WeatherInfo, Error> {
    if let Some((cached_info, cached_time)) = self.cache.read().await.as_ref() {
      if (Utc::now() - *cached_time)
        < chrono::Duration::from_std(self.config.cache_duration)
          .map_err(|_| Error::InvalidResponse("Invalid duration conversion".to_string()))?
      {
        info!("Returning cached weather data for {}", city);
        return Ok(cached_info.clone());
      }
    }

    if city.trim().is_empty() {
      return Err(Error::InvalidCity("City name cannot be empty".into()));
    }

    let url = self.build_api_url(city)?;
    let response = self.client.get(url).send().await?;

    match response.status() {
      reqwest::StatusCode::OK => (),
      reqwest::StatusCode::TOO_MANY_REQUESTS => return Err(Error::RateLimitExceeded),
      status => return Err(Error::ApiError(format!("API request failed: {}", status))),
    }

    let weather_data: WeatherResponse = response.json().await?;
    if weather_data.cod != 200 {
      return Err(Error::InvalidResponse(format!(
        "Invalid response code: {}",
        weather_data.cod
      )));
    }

    let weather_info = WeatherInfo::from_response(weather_data)?;
    *self.cache.write().await = Some((weather_info.clone(), Utc::now()));

    Ok(weather_info)
  }

  fn build_api_url(&self, city: &str) -> Result<Url, Error> {
    Url::parse_with_params(
      API_BASE_URL,
      &[
        ("q", city),
        ("appid", &self.config.api_key),
        ("units", "metric"),
      ],
    )
    .map_err(|_| Error::InvalidCity("Failed to build API URL".into()))
  }

  async fn update_readme(
    &self,
    weather: &WeatherInfo,
    section: &WeatherSection,
  ) -> Result<(), Error> {
    let content = tokio::fs::read_to_string(&self.config.readme_path).await?;
    let weather_text = weather.format_readme();

    let new_content = format!(
      "{}{}{}-->\n{}\n{}{}",
      &content[..section.start_pos],
      START_SECTION_PREFIX,
      section.city,
      weather_text,
      WEATHER_END,
      &content[section.end_pos + WEATHER_END.len()..]
    );

    let temp_path = self.config.readme_path.with_extension("tmp");
    tokio::fs::write(&temp_path, &new_content).await?;
    tokio::fs::rename(&temp_path, &self.config.readme_path).await?;

    info!("Successfully updated weather information");
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<UpdateResult, Error> {
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

#[derive(Debug, Clone)]
struct WeatherSection {
  city: String,
  last_update: Option<DateTime<Utc>>,
  start_pos: usize,
  end_pos: usize,
}

impl WeatherSection {
  fn parse(content: &str) -> Result<Self, Error> {
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
      .ok_or(Error::MissingCity)?;

    let city = content[city_start..city_start + city_end].trim();
    if city.is_empty() {
      return Err(Error::MissingCity);
    }

    let section_content = &content[start_pos..end_pos];
    let last_update = Self::parse_last_update(section_content);

    Ok(Self {
      city: city.to_string(),
      last_update,
      start_pos,
      end_pos,
    })
  }

  fn parse_last_update(content: &str) -> Option<DateTime<Utc>> {
    let update_pos = content.find(LAST_UPDATE_PREFIX)?;
    let timestamp_start = update_pos + LAST_UPDATE_PREFIX.len();
    let timestamp_end = content[timestamp_start..]
      .find(HTML_COMMENT_END)
      .unwrap_or(content.len());

    let timestamp = content[timestamp_start..timestamp_start + timestamp_end].trim();
    debug!("Found timestamp: {}", timestamp);

    DateTime::parse_from_str(
      &format!("{} +0000", timestamp),
      format!("{} %z", DATETIME_FORMAT).as_str(),
    )
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| {
      warn!("Failed to parse timestamp '{}': {}", timestamp, e);
      e
    })
    .ok()
  }
}
