// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use super::api::WeatherResponse;
use crate::constants::*;
use base::Error;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherInfo {
  pub temp: f64,
  pub feels_like: f64,
  pub condition: String,
  pub condition_desc: String,
  pub sunrise: DateTime<FixedOffset>,
  pub sunset: DateTime<FixedOffset>,
  pub location: String,
  pub country: String,
  pub emoji: String,
  pub last_update: DateTime<Utc>,
}

impl WeatherInfo {
  fn get_emoji(condition: &str) -> String {
    match condition {
      "Thunderstorm" => "⛈️",
      "Drizzle" => "🌦️",
      "Rain" => "🌧️",
      "Snow" => "❄️",
      "Atmosphere" => "🌫️",
      "Clear" => "☀️",
      "Clouds" => "☁️",
      _ => "❓",
    }
    .to_string()
  }

  pub fn from_response(response: WeatherResponse) -> Result<Self, Error> {
    let tz_offset = FixedOffset::east_opt(response.timezone)
      .ok_or_else(|| Error::InvalidResponse("Invalid timezone offset".to_string()))?;

    let weather = response
      .weather
      .first()
      .ok_or_else(|| Error::InvalidResponse("No weather data available".to_string()))?;

    let sunrise = Utc
      .timestamp_opt(response.sys.sunrise, 0)
      .single()
      .ok_or_else(|| Error::InvalidResponse("Invalid sunrise timestamp".to_string()))?
      .with_timezone(&tz_offset);

    let sunset = Utc
      .timestamp_opt(response.sys.sunset, 0)
      .single()
      .ok_or_else(|| Error::InvalidResponse("Invalid sunset timestamp".to_string()))?
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
      emoji: Self::get_emoji(&weather.main),
      last_update: Utc::now(),
    })
  }

  pub fn format_readme(&self) -> String {
    let today = self.sunrise.format("%B %d, %Y");
    format!(
      "{}{}{}\n{}",
      LAST_UPDATE_PREFIX,
      Utc::now().format(DATETIME_FORMAT),
      HTML_COMMENT_END,
      self.format_weather_text(today)
    )
  }

  fn format_weather_text(&self, today: impl std::fmt::Display) -> String {
    format!(
      "Currently in **{}** ({}), the weather is: **{:.1}°C** (feels like **{:.1}°C**), ***{}***<br/>\n\
      On *{}*, the *sun rises* at 🌅**{}** and *sets* at 🌇**{}**.",
      self.location,
      self.country,
      self.temp,
      self.feels_like,
      self.condition_desc,
      today,
      self.sunrise.format("%H:%M"),
      self.sunset.format("%H:%M")
    )
  }
}
