// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WakaError {
  #[error("API error: {0}")]
  ApiError(String),
  #[error("Configuration error: {0}")]
  ConfigError(String),
  #[error("IO error: {0}")]
  IoError(#[from] std::io::Error),
  #[error("Template error: {0}")]
  TemplateError(String),
  #[error("HTTP error: {0}")]
  HttpError(#[from] reqwest::Error),
  #[error("Regex error: {0}")]
  RegexError(#[from] regex::Error),
}
