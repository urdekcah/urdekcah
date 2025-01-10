// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{
  template::Template,
  wakatime::{WakaStats, WakaTimeApi},
  DATETIME_FORMAT, HTML_COMMENT_END, LAST_UPDATE_PREFIX,
};
use base::{Config, Error};
use chrono::{DateTime, Utc};
use std::{fs, path::Path};
use tracing::{debug, instrument};

pub struct StatsGenerator<T: WakaTimeApi> {
  config: Config,
  client: T,
}

#[derive(Debug, Clone)]
pub struct WakaUpdateResult {
  pub stats: String,
  pub last_update: Option<DateTime<Utc>>,
  pub current_update: DateTime<Utc>,
  pub was_updated: bool,
}

impl<T: WakaTimeApi> StatsGenerator<T> {
  pub fn new(config: Config, client: T) -> Self {
    Self { config, client }
  }

  fn parse_last_update(content: &str) -> Option<DateTime<Utc>> {
    let update_pos = content.find(LAST_UPDATE_PREFIX)?;
    let timestamp_start = update_pos + LAST_UPDATE_PREFIX.len();
    let timestamp_end = content[timestamp_start..]
      .find(HTML_COMMENT_END)
      .unwrap_or(content.len());

    let timestamp = content[timestamp_start..timestamp_start + timestamp_end].trim();
    match DateTime::parse_from_str(
      &format!("{} +0000", timestamp),
      format!("{} %z", DATETIME_FORMAT).as_str(),
    ) {
      Ok(dt) => Some(dt.with_timezone(&Utc)),
      Err(_) => None,
    }
  }

  #[instrument(skip(self))]
  pub async fn generate_stats(&self) -> Result<WakaUpdateResult, Error> {
    debug!("Fetching WakaTime stats");
    let stats = self
      .client
      .fetch_stats(&self.config.wakatime.time_range)
      .await?;

    debug!("Preparing content from stats");
    let content = self.prepare_content(&stats)?;

    Ok(WakaUpdateResult {
      stats: content,
      last_update: None, // Will be populated in update_readme
      current_update: Utc::now(),
      was_updated: false,
    })
  }

  #[instrument(skip(self, stats))]
  fn prepare_content(&self, stats: &WakaStats) -> Result<String, Error> {
    let template = Template::new(&self.config);
    template.render(stats)
  }

  #[instrument(skip(self, content))]
  pub fn update_readme<P: AsRef<Path> + std::fmt::Debug>(
    &self,
    path: P,
    content: &str,
  ) -> Result<WakaUpdateResult, Error> {
    let readme = fs::read_to_string(path.as_ref())?;

    let start_comment = format!("<!--START_SECTION:{}-->", self.config.wakatime.section_name);
    let end_comment = format!("<!--END_SECTION:{}-->", self.config.wakatime.section_name);

    // Extract last update time from existing content
    let last_update = Self::parse_last_update(&readme);
    let current_update = Utc::now();

    let replacement = format!(
      "{}\n<!--LAST_WAKA_UPDATE:{}-->\n```{}\n{}```\n{}",
      start_comment,
      current_update.format(DATETIME_FORMAT),
      self.config.wakatime.code_lang,
      content,
      end_comment
    );

    let pattern = format!(
      "{}[\\s\\S]+{}",
      regex::escape(&start_comment),
      regex::escape(&end_comment)
    );

    let re = regex::Regex::new(&pattern)
      .map_err(|e| Error::TemplateError(format!("Invalid regex pattern: {}", e)))?;

    let new_readme = re.replace(&readme, replacement);
    let was_updated = new_readme != readme;

    if was_updated {
      fs::write(path, new_readme.as_bytes())?;
      debug!("README updated successfully");
    } else {
      debug!("No changes needed in README");
    }

    Ok(WakaUpdateResult {
      stats: content.to_string(),
      last_update,
      current_update,
      was_updated,
    })
  }
}
