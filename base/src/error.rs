// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
  #[error("API error: {0}")]
  ApiError(String),
  #[error("Configuration error: {0}")]
  ConfigError(String),
  #[error("IO error: {0}")]
  IoError(#[from] std::io::Error),
  #[error("HTTP error: {0}")]
  HttpError(#[from] reqwest::Error),
  #[error("Failed to parse response: {0}")]
  ParseError(String),
  #[error("Template error: {0}")]
  TemplateError(String),
  #[error("Regex error: {0}")]
  RegexError(#[from] regex::Error),
  #[error("Invalid city name: {0}")]
  InvalidCity(String),
  #[error("Invalid API key")]
  InvalidApiKey,
  #[error("Invalid response from weather API: {0}")]
  InvalidResponse(String),
  #[error("Rate limit exceeded")]
  RateLimitExceeded,
  #[error("Timeout error")]
  TimeoutError,
  #[error("Weather section header must include city name (<!--START_SECTION:weather:city-->)")]
  MissingCityInSection,
  #[error("Weather section not found in README - skipping weather update")]
  WeatherSectionNotFound,
}
