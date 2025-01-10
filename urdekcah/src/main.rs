// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use base::Config;
use std::{env, path::PathBuf};
use tracing::{info, instrument};
use wakatime::WakaTimeService;
use weather::{WeatherConfig, WeatherService};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
  weather_api_key: String,
  wakatime_api_key: String,
  readme_path: PathBuf,
  config_path: PathBuf,
}

pub struct ServiceRunner {
  weather_service: WeatherService,
  wakatime_service: WakaTimeService,
}

#[cfg(debug_assertions)]
fn setup_logging() {
  tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_thread_ids(true)
    // .with_target(true)
    .init();
}

#[cfg(not(debug_assertions))]
fn setup_logging() {
  tracing_subscriber::fmt().init();
}

#[tokio::main]
async fn main() -> Result<()> {
  setup_logging();

  let config = ServiceConfig {
    weather_api_key: env::var("OPENWEATHER_API_KEY").context("Missing OPENWEATHER_API_KEY")?,
    wakatime_api_key: env::var("WAKATIME_API_KEY").context("Missing WAKATIME_API_KEY")?,
    readme_path: "README.md".into(),
    config_path: "urdekcah.toml".into(),
  };

  ServiceRunner::new(config)?.run().await
}

impl ServiceRunner {
  #[instrument(skip(config))]
  pub fn new(config: ServiceConfig) -> Result<Self> {
    Ok(Self {
      weather_service: WeatherService::new(WeatherConfig::new(
        config.weather_api_key.clone(),
        config.readme_path.to_str().unwrap_or("README.md"),
        std::time::Duration::from_secs(300),
      )?),
      wakatime_service: WakaTimeService::new(
        Config::from_file(&config.config_path)?,
        config.wakatime_api_key.clone(),
      ),
    })
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<()> {
    if let Err(e) = self.weather_service.run().await {
      tracing::warn!("Weather service error: {e:?}");
    }

    match self.wakatime_service.run().await {
      Ok(update_result) => {
        if update_result.was_updated {
          info!(
            "WakaTime stats updated successfully. Previous update: {:?}, Current update: {}",
            update_result.last_update,
            update_result.current_update.format("%Y-%m-%d %H:%M:%S")
          );
        } else {
          info!(
            "No WakaTime stats update needed. Last update: {:?}",
            update_result.last_update
          );
        }
      }
      Err(e) => tracing::warn!("WakaTime service error: {e:?}"),
    }

    Ok(())
  }
}
