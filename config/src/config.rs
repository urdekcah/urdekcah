// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tracing::instrument;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
  pub wakatime: WakaTimeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WakaTimeConfig {
  pub show_title: bool,
  pub section_name: String,
  pub blocks: String,
  pub code_lang: String,
  pub time_range: WakaTimeRange,
  pub lang_count: i32,
  pub show_time: bool,
  pub show_total: bool,
  pub show_masked_time: bool,
  pub stop_at_other: bool,
  pub ignored_languages: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakaTimeRange {
  Last7Days,
  Last30Days,
  Last6Months,
  LastYear,
  AllTime,
}

impl Config {
  #[instrument(skip(path))]
  pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let content = fs::read_to_string(path)?;
    let config: Self = toml::from_str(&content)?;
    tracing::debug!("Loaded configuration successfully");
    Ok(config)
  }
}

impl std::fmt::Display for WakaTimeRange {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let range = match self {
      WakaTimeRange::Last7Days => "last_7_days",
      WakaTimeRange::Last30Days => "last_30_days",
      WakaTimeRange::Last6Months => "last_6_months",
      WakaTimeRange::LastYear => "last_year",
      WakaTimeRange::AllTime => "all_time",
    };
    write!(f, "{}", range)
  }
}