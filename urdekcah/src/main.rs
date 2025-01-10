// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use base::Config;
use std::{env, path::PathBuf};
use telegram::TelegramClient;
use tracing::instrument;
use wakatime::WakaTimeService;
use weather::{WeatherConfig, WeatherService};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
  weather_api_key: String,
  wakatime_api_key: String,
  telegram_bot_token: String,
  telegram_chat_id: i64,
  readme_path: PathBuf,
  config_path: PathBuf,
}

pub struct ServiceRunner {
  weather_service: WeatherService,
  wakatime_service: WakaTimeService,
  tg: TelegramClient,
  tg_chat_id: i64,
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
  #[cfg(debug_assertions)]
  base::dotenv::load()?;
  setup_logging();

  let config = ServiceConfig {
    weather_api_key: env::var("OPENWEATHER_API_KEY").context("Missing OPENWEATHER_API_KEY")?,
    wakatime_api_key: env::var("WAKATIME_API_KEY").context("Missing WAKATIME_API_KEY")?,
    telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").context("Missing TELEGRAM_BOT_TOKEN")?,
    telegram_chat_id: env::var("TELEGRAM_CHAT_ID")
      .context("Missing TELEGRAM_CHAT_ID")?
      .parse()?,
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
      tg: TelegramClient::builder()
        .token(config.telegram_bot_token.clone())
        .build()?,
      tg_chat_id: config.telegram_chat_id,
    })
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<()> {
    match self.weather_service.run().await {
      Ok(result) => {
        let weather = &result.weather;
        self.tg.message()
          .chat_id(self.tg_chat_id)
          .text(
            format!(
              "В настоящее время в *{}* погода,\nТекущая темп.: *{}°C*\nОщущается как: *{}°C*\nТекущая погода: *{}*\nПоследнее обновление было в: _{}_",
              weather.location, weather.temp, weather.feels_like,
              weather.condition_desc,
              result.last_update.map_or("N/A".to_string(), |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            ).as_str()
          )
          .parse_mode(telegram::ParseMode::MarkdownV2)
          .send(&self.tg)
          .await?;
      }
      Err(e) => tracing::warn!("Weather service error: {e:?}"),
    }

    match self.wakatime_service.run().await {
      Ok(update_result) => {
        if update_result.was_updated {
          self
            .tg
            .message()
            .chat_id(self.tg_chat_id)
            .text(
              format!(
                "WakaTime статистика успешно обновлена.\nПредыдущее обновление: *{}*",
                update_result.last_update.map_or("N/A".to_string(), |dt| dt
                  .format("%Y-%m-%d %H:%M:%S")
                  .to_string())
              )
              .as_str(),
            )
            .parse_mode(telegram::ParseMode::MarkdownV2)
            .send(&self.tg)
            .await?;
        } else {
          self
            .tg
            .message()
            .chat_id(self.tg_chat_id)
            .text(
              format!(
                "_Обновление статистики WakaTime не требуется._\nПоследнее обновление: *{}*",
                update_result.last_update.map_or("N/A".to_string(), |dt| dt
                  .format("%Y-%m-%d %H:%M:%S")
                  .to_string())
              )
              .as_str(),
            )
            .parse_mode(telegram::ParseMode::MarkdownV2)
            .send(&self.tg)
            .await?;
        }
      }
      Err(e) => tracing::warn!("WakaTime service error: {e:?}"),
    }

    Ok(())
  }
}
