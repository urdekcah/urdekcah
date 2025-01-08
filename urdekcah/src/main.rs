// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use config::Config;
use std::{env, path::PathBuf, time::Duration};
use tracing::{info, instrument, warn};
use wakatime::{StatsGenerator, WakaTimeApi, WakaTimeClient};
use weather::{WeatherConfig, WeatherService};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
  pub(crate) weather_api_key: String,
  pub(crate) wakatime_api_key: String,
  pub(crate) readme_path: PathBuf,
  pub(crate) config_path: PathBuf,
}

pub struct ServiceRunner<T: WakaTimeApi> {
  config: ServiceConfig,
  weather_service: WeatherService,
  stats_generator: StatsGenerator<T>,
}

fn setup_logging() {
  tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_thread_ids(true)
    // .with_target(true)
    .init();
}

#[tokio::main]
async fn main() -> Result<()> {
  setup_logging();

  let weather_key =
    env::var("OPENWEATHER_API_KEY").context("Missing OPENWEATHER_API_KEY environment variable")?;

  let wakatime_key =
    env::var("WAKATIME_API_KEY").context("Missing WAKATIME_API_KEY environment variable")?;

  let config = ServiceConfig::new(
    weather_key,
    wakatime_key,
    "README.md".to_string(),
    "urdekcah.toml",
  );

  let wakatime_client = WakaTimeClient::new(config.wakatime_api_key.clone().as_str());
  let runner =
    ServiceRunner::new(config, wakatime_client).context("Failed to initialize service runner")?;

  info!("Starting service runner");
  runner.run().await.context("Service runner failed")?;

  Ok(())
}

impl ServiceConfig {
  pub fn new(
    weather_key: String,
    wakatime_key: String,
    readme_path: impl Into<PathBuf> + std::fmt::Debug,
    config_path: impl Into<PathBuf> + std::fmt::Debug,
  ) -> Self {
    Self {
      weather_api_key: weather_key,
      wakatime_api_key: wakatime_key,
      readme_path: readme_path.into(),
      config_path: config_path.into(),
    }
  }
}

impl<T: WakaTimeApi> ServiceRunner<T> {
  #[instrument(skip(config, client))]
  pub fn new(config: ServiceConfig, client: T) -> Result<Self> {
    let weather_config = WeatherConfig::new(
      config.weather_api_key.clone(),
      config.readme_path.to_str().unwrap_or("README.md"),
      Duration::from_secs(300),
    )
    .context("Failed to create weather configuration")?;

    let wakatime_config =
      Config::from_file(&config.config_path).context("Failed to load WakaTime configuration")?;

    Ok(Self {
      weather_service: WeatherService::new(weather_config),
      stats_generator: StatsGenerator::new(wakatime_config, client),
      config,
    })
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<()> {
    info!("Starting services");

    if let Err(e) = self.weather_service.run().await {
      warn!("Weather service error: {:?}", e);
    }

    match self.update_wakatime_stats().await {
      Ok(updated) => {
        if updated {
          info!("Successfully updated WakaTime statistics");
        }
      }
      Err(e) => warn!("WakaTime update error: {:?}", e),
    }

    Ok(())
  }

  #[instrument(skip(self))]
  async fn update_wakatime_stats(&self) -> Result<bool> {
    info!("Generating WakaTime stats");
    let stats = self
      .stats_generator
      .generate_stats()
      .await
      .context("Failed to generate WakaTime stats")?;

    info!("Updating README.md with new stats");
    self
      .stats_generator
      .update_readme(
        self.config.readme_path.to_str().unwrap_or("README.md"),
        &stats,
      )
      .context("Failed to update README with stats")
  }
}
