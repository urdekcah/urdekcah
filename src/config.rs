// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::error::WeatherError;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WeatherConfig {
  pub(crate) api_key: String,
  pub(crate) readme_path: PathBuf,
  pub(crate) cache_duration: std::time::Duration,
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
