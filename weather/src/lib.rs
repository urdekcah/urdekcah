// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
pub mod config;
pub mod models;
pub mod service;

pub use config::WeatherConfig;
pub use models::weather::WeatherInfo;
pub use service::{UpdateResult, WeatherService};

pub mod constants {
  use std::time::Duration;
  pub(crate) const WEATHER_END: &str = "<!--END_SECTION:weather-->";
  pub(crate) const API_BASE_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
  pub(crate) const START_SECTION_PREFIX: &str = "<!--START_SECTION:weather:";
  pub(crate) const LAST_UPDATE_PREFIX: &str = "<!--LAST_WEATHER_UPDATE:";
  pub(crate) const HTML_COMMENT_END: &str = "-->";
  pub(crate) const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
  pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
}
