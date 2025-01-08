// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use config::Config;
use std::env;
use tracing::{error, info};
use wakatime::{StatsGenerator, WakaTimeClient};
use weather::{WeatherConfig, WeatherService};

#[tokio::main]
async fn main() -> Result<()> {
  tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_thread_ids(true)
    .init();

  let api_key =
    env::var("OPENWEATHER_API_KEY").context("Missing OPENWEATHER_API_KEY environment variable")?;

  let config = WeatherConfig::new(api_key, "README.md", std::time::Duration::from_secs(300))?;
  let service = WeatherService::new(config);

  if let Err(e) = service.run().await {
    error!("Failed to update weather: {:?}", e);
    std::process::exit(1);
  }

  let api_key =
    env::var("WAKATIME_API_KEY").context("Missing WAKATIME_API_KEY environment variable")?;

  let config = Config::from_file("urdekcah.toml")?;
  let client = WakaTimeClient::new(&api_key);
  let generator = StatsGenerator::new(config, client);

  info!("Generating WakaTime stats");
  let stats = generator.generate_stats().await?;

  info!("Updating README.md");
  if generator.update_readme("README.md", &stats)? {
    info!("README.md updated successfully");
  }

  Ok(())
}
