// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WeatherError {
  #[error("Weather section header must include city name (<!--START_SECTION:weather:city-->)")]
  MissingCityInSection,
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
