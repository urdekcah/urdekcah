// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use super::api::WeatherResponse;
use crate::WEATHER_END;
use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WeatherInfo {
  pub temp: f64,
  pub feels_like: f64,
  pub condition: String,
  pub condition_desc: String,
  pub sunrise: DateTime<FixedOffset>,
  pub sunset: DateTime<FixedOffset>,
  pub location: String,
  pub country: String,
}

impl WeatherInfo {
  pub(crate) fn from_response(response: WeatherResponse) -> Result<Self> {
    let tz_offset = FixedOffset::east_opt(response.timezone).context("Invalid timezone offset")?;

    let weather = response
      .weather
      .first()
      .context("No weather data available")?;

    let sunrise = Utc
      .timestamp_opt(response.sys.sunrise, 0)
      .single()
      .context("Invalid sunrise timestamp")?
      .with_timezone(&tz_offset);

    let sunset = Utc
      .timestamp_opt(response.sys.sunset, 0)
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

  pub(crate) fn format_readme(&self) -> String {
    let today = self.sunrise.format("%B %d, %Y");
    format!(
      "Currently in **{}** ({}), the weather is: **{:.1}°C** (feels like **{:.1}°C**), ***{}***<br/>\n\
      On *{}*, the *sun rises* at 🌅**{}** and *sets* at 🌇**{}**.\n\
      {}",
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
