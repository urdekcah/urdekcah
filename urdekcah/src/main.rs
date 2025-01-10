// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use base::Config;
use std::{
  env,
  path::{Path, PathBuf},
};
use tracing::{info, instrument};
use wakatime::{StatsGenerator, WakaTimeApi, WakaTimeClient};
use weather::{WeatherConfig, WeatherService};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
  weather_api_key: String,
  wakatime_api_key: String,
  readme_path: PathBuf,
  config_path: PathBuf,
}

pub struct ServiceRunner<T: WakaTimeApi> {
  weather_service: WeatherService,
  stats_generator: StatsGenerator<T>,
  config: ServiceConfig,
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

  let wak = config.wakatime_api_key.clone();
  ServiceRunner::new(config, WakaTimeClient::new(&wak))?
    .run()
    .await
}

impl<T: WakaTimeApi> ServiceRunner<T> {
  #[instrument(skip(config, client))]
  pub fn new(config: ServiceConfig, client: T) -> Result<Self> {
    Ok(Self {
      weather_service: WeatherService::new(WeatherConfig::new(
        config.weather_api_key.clone(),
        config.readme_path.to_str().unwrap_or("README.md"),
        std::time::Duration::from_secs(300),
      )?),
      stats_generator: StatsGenerator::new(
        Config::from_file(&config.config_path).context("Failed to load WakaTime config")?,
        client,
      ),
      config,
    })
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<()> {
    if let Err(e) = self.weather_service.run().await {
      tracing::warn!("Weather service error: {e:?}");
    }

    let stat_result = self.stats_generator.generate_stats().await?;
    let update_result = self
      .stats_generator
      .update_readme(Path::new("README.md"), &stat_result.stats)?;

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

    Ok(())
  }
}
