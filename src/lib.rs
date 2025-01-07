// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
pub mod config;
pub mod error;
pub mod models;
pub mod service;
pub mod utils;

pub use config::WeatherConfig;
pub use error::WeatherError;
pub use models::weather::WeatherInfo;
pub use service::weather::WeatherService;

pub(crate) const WEATHER_END: &str = "<!--END_SECTION:weather-->";
pub(crate) const API_BASE_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
pub(crate) const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
